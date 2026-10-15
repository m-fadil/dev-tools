use base64::Engine;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig, general_purpose};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Codec {
    Base64,
    Base64Url,
    Hex,
    Url,
}

pub const CODECS: &[(&str, Codec)] = &[
    ("base64", Codec::Base64),
    ("base64url", Codec::Base64Url),
    ("hex", Codec::Hex),
    ("url", Codec::Url),
];

impl Codec {
    pub fn name(self) -> &'static str {
        match self {
            Codec::Base64 => "base64",
            Codec::Base64Url => "base64url",
            Codec::Hex => "hex",
            Codec::Url => "url",
        }
    }
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b)
}

fn encode_url(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len());
    for &b in data {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode_url(value: &str) -> Result<Vec<u8>, String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let hi = bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16));
        let lo = bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16));
        match (hi, lo) {
            (Some(h), Some(l)) => out.push((h * 16 + l) as u8),
            _ => return Err("Input bukan URL-encoding yang valid.".into()),
        }
        i += 3;
    }
    Ok(out)
}

fn engine(url_safe: bool) -> GeneralPurpose {
    let cfg = GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(false);
    if url_safe {
        GeneralPurpose::new(&base64::alphabet::URL_SAFE, cfg)
    } else {
        GeneralPurpose::new(&base64::alphabet::STANDARD, cfg)
    }
}

pub fn encode(data: &[u8], codec: Codec) -> String {
    match codec {
        Codec::Base64 => general_purpose::STANDARD.encode(data),
        Codec::Base64Url => general_purpose::URL_SAFE_NO_PAD.encode(data),
        Codec::Hex => hex::encode(data),
        Codec::Url => encode_url(data),
    }
}

/// Strict decode. base64/base64url/hex ignore all whitespace.
pub fn decode(text: &str, codec: Codec) -> Result<Vec<u8>, String> {
    use crate::tools::number::jstext::{is_js_space, js_trim};
    let squash = || text.chars().filter(|c| !is_js_space(*c)).collect::<String>();
    match codec {
        Codec::Base64 | Codec::Base64Url => {
            let value = squash();
            let bad = || format!("Input bukan {} yang valid.", codec.name());
            // Padding, when present, must complete the final quantum exactly.
            let body = value.trim_end_matches('=');
            let pad = value.len() - body.len();
            let rest = body.len() % 4;
            if pad > 2 || rest == 1 || (pad > 0 && pad != 4 - rest) {
                return Err(bad());
            }
            engine(codec == Codec::Base64Url).decode(body).map_err(|_| bad())
        }
        Codec::Hex => hex::decode(squash()).map_err(|_| "Input bukan hex yang valid.".to_string()),
        Codec::Url => decode_url(js_trim(text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_strict() {
        assert_eq!(decode("aGk", Codec::Base64).unwrap(), b"hi");
        assert_eq!(decode("aGk=", Codec::Base64).unwrap(), b"hi");
        assert!(decode("aGk==", Codec::Base64).is_err());
        assert!(decode("aGl=", Codec::Base64).is_err());
        assert!(decode("a", Codec::Base64).is_err());
        assert!(decode("-_", Codec::Base64).is_err());
        assert_eq!(decode("a G\nk", Codec::Base64).unwrap(), b"hi");
    }

    #[test]
    fn url_round() {
        assert_eq!(encode(b"a b+c/", Codec::Url), "a%20b%2Bc%2F");
        assert_eq!(decode("a%20b+c", Codec::Url).unwrap(), b"a b+c");
        assert!(decode("%2", Codec::Url).is_err());
    }
}
