use std::fmt::Write;

/// Escapes `s` as the content of a JSON string (RFC 8259).
pub fn json_encode(s: &str, ascii: bool, quotes: bool) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    if quotes {
        out.push('"');
    }
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if ascii && !c.is_ascii() => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
            c => out.push(c),
        }
    }
    if quotes {
        out.push('"');
    }
    out
}

fn hex4(it: &mut std::str::Chars) -> Option<u32> {
    let mut v = 0u32;
    for _ in 0..4 {
        v = v * 16 + it.next()?.to_digit(16)?;
    }
    Some(v)
}

/// Decodes JSON string content, with or without the surrounding quotes.
pub fn json_decode(s: &str) -> Result<String, String> {
    let body = match s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner) => inner,
        None => s,
    };
    let mut out = String::with_capacity(body.len());
    let mut it = body.chars();
    let mut pos = 0usize; // character index of the char being read, for messages
    while let Some(c) = it.next() {
        pos += 1;
        match c {
            '"' => return Err(format!("Tanda kutip tanpa escape pada posisi {pos}.")),
            c if (c as u32) < 0x20 => {
                return Err(format!(
                    "Karakter kontrol mentah (U+{:04X}) pada posisi {pos}; harus di-escape.",
                    c as u32
                ));
            }
            '\\' => {
                let start = pos;
                let Some(e) = it.next() else {
                    return Err(format!("Backslash di akhir input (posisi {start})."));
                };
                pos += 1;
                match e {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        let bad = || format!("Escape \\u tidak valid pada posisi {start}.");
                        let hi = hex4(&mut it).ok_or_else(bad)?;
                        pos += 4;
                        let cp = match hi {
                            0xD800..=0xDBFF => {
                                let mut look = it.clone();
                                let lo = (look.next() == Some('\\') && look.next() == Some('u'))
                                    .then(|| hex4(&mut look))
                                    .flatten()
                                    .filter(|lo| (0xDC00..=0xDFFF).contains(lo));
                                let Some(lo) = lo else {
                                    return Err(format!("Surrogate tinggi tanpa pasangan pada posisi {start}."));
                                };
                                it = look;
                                pos += 6;
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            }
                            0xDC00..=0xDFFF => {
                                return Err(format!("Surrogate rendah tanpa pasangan pada posisi {start}."));
                            }
                            cp => cp,
                        };
                        match char::from_u32(cp) {
                            Some(ch) => out.push(ch),
                            None => return Err(bad()),
                        }
                    }
                    other => return Err(format!("Escape tidak dikenal \\{other} pada posisi {start}.")),
                }
            }
            c => out.push(c),
        }
    }
    Ok(out)
}

pub fn html_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn named_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        "copy" => '\u{a9}',
        "reg" => '\u{ae}',
        "hellip" => '\u{2026}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        _ => return None,
    })
}

fn entity(name: &str) -> Option<char> {
    match name.strip_prefix('#') {
        Some(num) => {
            let cp = match num.strip_prefix(['x', 'X']) {
                Some(h) if h.bytes().all(|b| b.is_ascii_hexdigit()) && !h.is_empty() => {
                    u32::from_str_radix(h, 16).ok()?
                }
                Some(_) => return None,
                None if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) => num.parse().ok()?,
                None => return None,
            };
            if cp == 0 { None } else { char::from_u32(cp) }
        }
        None => named_entity(name),
    }
}

/// Decodes known HTML entities; anything unrecognised is kept verbatim.
pub fn html_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        // The terminating ';' must follow within a short window.
        let decoded = rest.as_bytes()[1..]
            .iter()
            .take(32)
            .position(|&b| b == b';')
            .and_then(|end| entity(&rest[1..1 + end]).map(|c| (c, end + 2)));
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

pub fn url_decode(s: &str, plus: bool) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
                let byte = hex
                    .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .and_then(|h| u8::from_str_radix(h, 16).ok());
                let Some(byte) = byte else {
                    return Err(format!("Urutan persen tidak valid pada posisi {i}."));
                };
                out.push(byte);
                i += 3;
            }
            b'+' if plus => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| "Hasil decode bukan UTF-8 yang valid.".to_string())
}

pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}
