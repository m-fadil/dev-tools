use crate::errors::{Result, usage};

#[derive(Clone, Copy, PartialEq)]
pub enum MacFormat {
    Colon,
    Hyphen,
    Dot,
    Plain,
}

pub const FORMAT_CHOICES: [(&str, MacFormat); 4] = [
    ("colon", MacFormat::Colon),
    ("hyphen", MacFormat::Hyphen),
    ("dot", MacFormat::Dot),
    ("plain", MacFormat::Plain),
];

/// JS `String.prototype.trim` whitespace set.
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    })
}

fn all_hex(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn matches_pattern(t: &str) -> bool {
    if !t.is_ascii() {
        return false;
    }
    let grouped = |sep: char| {
        let parts: Vec<&str> = t.split(sep).collect();
        parts.len() == 6 && parts.iter().all(|p| p.len() == 2 && all_hex(p))
    };
    if grouped(':') || grouped('-') {
        return true;
    }
    let dots: Vec<&str> = t.split('.').collect();
    if dots.len() == 3 && dots.iter().all(|p| p.len() == 4 && all_hex(p)) {
        return true;
    }
    t.len() == 12 && all_hex(t)
}

/// Parses EUI-48 (colon, hyphen, Cisco dot or plain, surrounding whitespace ignored) into 12 lowercase hex digits.
pub fn parse_mac(input: &str) -> Result<String> {
    let text = js_trim(input);
    if !matches_pattern(text) {
        return Err(usage(format!(
            "Alamat MAC tidak valid: {text} (bentuk: aa:bb:cc:dd:ee:ff, aa-bb-cc-dd-ee-ff, aabb.ccdd.eeff, aabbccddeeff)"
        )));
    }
    Ok(text
        .chars()
        .filter(|c| !matches!(c, ':' | '.' | '-'))
        .collect::<String>()
        .to_lowercase())
}

fn join(hex: &str, size: usize, sep: &str) -> String {
    hex.as_bytes()
        .chunks(size)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(sep)
}

pub fn format_mac(hex: &str, format: MacFormat, upper: bool) -> String {
    let out = match format {
        MacFormat::Colon => join(hex, 2, ":"),
        MacFormat::Hyphen => join(hex, 2, "-"),
        MacFormat::Dot => join(hex, 4, "."),
        MacFormat::Plain => hex.to_string(),
    };
    if upper { out.to_uppercase() } else { out }
}

pub struct MacFlags {
    pub multicast: bool,
    pub locally_administered: bool,
}

/// I/G bit (LSB of first octet) and U/L bit (second LSB).
pub fn mac_flags(hex: &str) -> MacFlags {
    let first = u8::from_str_radix(&hex[..2], 16).unwrap_or(0);
    MacFlags {
        multicast: first & 1 == 1,
        locally_administered: first & 2 == 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forms() {
        for f in [
            "00:1A:2B:3C:4D:5E",
            "00-1a-2b-3c-4d-5e",
            "001a.2b3c.4d5e",
            "001A2B3C4D5E",
            " \n001A2B3C4D5E\t",
        ] {
            assert_eq!(parse_mac(f).ok().as_deref(), Some("001a2b3c4d5e"), "{f}");
        }
        for bad in [
            "aa:bb:cc:dd:ee",
            "aa-bb:cc-dd:ee-ff",
            "aabb.ccdd",
            "a:b:c:d:e:f",
            "",
            "aa:bb:cc:dd:ee:é",
        ] {
            assert!(parse_mac(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn formats() {
        assert_eq!(format_mac("aabbccddeeff", MacFormat::Dot, true), "AABB.CCDD.EEFF");
    }
}
