use super::errors::error;
use crate::errors::Result;
#[derive(Clone)]
pub struct Token {
    pub kind: u8,
    pub text: String,
    pub col: usize,
    pub value: f64,
}
pub fn lex(src: &str, mut pos: usize) -> Result<(Token, usize)> {
    let b = src.as_bytes();
    while pos < b.len() && matches!(b[pos], b' ' | b'\t') {
        pos += 1;
    }
    let col = src[..pos].encode_utf16().count() + 1;
    let mut t = Token {
        kind: 3,
        text: String::new(),
        col,
        value: 0.,
    };
    if pos == b.len() {
        return Ok((t, pos));
    }
    let tail = &src[pos..];
    let c = b[pos];
    if c.is_ascii_digit() || c == b'.' {
        let hex = tail.starts_with("0x") || tail.starts_with("0X");
        let pattern = if hex {
            r"^0[xX][0-9a-fA-F]+(?:_[0-9a-fA-F]+)*"
        } else {
            r"^(?:[0-9]+(?:_[0-9]+)*(?:\.(?:[0-9]+(?:_[0-9]+)*)?)?|\.[0-9]+(?:_[0-9]+)*)(?:[eE][+-]?[0-9]+(?:_[0-9]+)*)?"
        };
        let re = {
            static HEX: std::sync::OnceLock<std::result::Result<regex::Regex, regex::Error>> =
                std::sync::OnceLock::new();
            static DECIMAL: std::sync::OnceLock<std::result::Result<regex::Regex, regex::Error>> =
                std::sync::OnceLock::new();
            (if hex { &HEX } else { &DECIMAL })
                .get_or_init(|| regex::Regex::new(pattern))
                .as_ref()
        }
        .map_err(|e| error(src, col, e.to_string(), false))?;
        let m = re.find(tail).ok_or_else(|| {
            error(
                src,
                col + if hex { 2 } else { 0 },
                if hex {
                    "Digit heksadesimal diharapkan setelah 0x"
                } else {
                    "Titik harus diikuti digit, mis. '.5'"
                },
                false,
            )
        })?;
        t.text = m.as_str().into();
        let end = pos + t.text.len();
        if b.get(end) == Some(&b'_') {
            return Err(error(
                src,
                col + t.text.len(),
                "Pemisah '_' hanya boleh berada di antara dua digit",
                false,
            ));
        }
        let clean = t.text.replace('_', "");
        t.value = if hex {
            crate::tools::number::bigint::BigUint::parse_radix(&clean[2..], 16)
                .map(|n| n.to_radix_string(10).parse::<f64>().unwrap_or(f64::INFINITY))
                .unwrap_or(f64::NAN)
        } else {
            clean.parse().unwrap_or(f64::INFINITY)
        };
        if !t.value.is_finite() {
            return Err(error(src, col, format!("Angka terlalu besar: {}", t.text), false));
        }
        t.kind = 0;
        return Ok((t, end));
    }
    if c.is_ascii_alphabetic() || c == b'_' {
        let len = tail
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        t.kind = 1;
        t.text = tail[..len].into();
        return Ok((t, pos + len));
    }
    if tail.starts_with("**") {
        t.text = "**".into();
    } else if "+-*/%^!(),".as_bytes().contains(&c) {
        t.text = (c as char).to_string();
    } else {
        let ch = tail.chars().next().unwrap_or('\0').to_string();
        return Err(error(
            src,
            col,
            format!("Karakter tidak dikenal: {}", serde_json::json!(ch)),
            false,
        ));
    }
    t.kind = 2;
    let end = pos + t.text.len();
    Ok((t, end))
}
