use crate::errors::Result;
use crate::random;
use crate::tools::password::CryptoInts;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};

pub const TOKEN_FORMATS: &[(&str, TokenFormat)] = &[
    ("hex", TokenFormat::Hex),
    ("base64", TokenFormat::Base64),
    ("base64url", TokenFormat::Base64Url),
    ("alnum", TokenFormat::Alnum),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenFormat {
    Hex,
    Base64,
    Base64Url,
    Alnum,
}

impl TokenFormat {
    pub fn name(self) -> &'static str {
        match self {
            TokenFormat::Hex => "hex",
            TokenFormat::Base64 => "base64",
            TokenFormat::Base64Url => "base64url",
            TokenFormat::Alnum => "alnum",
        }
    }
}

const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn encode(format: TokenFormat, data: &[u8]) -> String {
    match format {
        TokenFormat::Hex => hex::encode(data),
        TokenFormat::Base64 => STANDARD_NO_PAD.encode(data),
        TokenFormat::Base64Url => URL_SAFE_NO_PAD.encode(data),
        TokenFormat::Alnum => String::new(),
    }
}

/// Exactly `length` characters. alnum draws each char uniformly (rejection sampling avoids modulo bias).
pub fn token_by_length(format: TokenFormat, length: usize, ints: &mut CryptoInts) -> Result<String> {
    if format == TokenFormat::Alnum {
        let mut s = String::with_capacity(length);
        for _ in 0..length {
            s.push(char::from(ALNUM[ints.below(ALNUM.len())?]));
        }
        return Ok(s);
    }
    // Unpadded alphabets carry 4 (hex) or 6 (base64) bits per char; slice off the surplus.
    let bits_per_char = if format == TokenFormat::Hex { 4 } else { 6 };
    let bytes = (length * bits_per_char).div_ceil(8);
    let mut s = encode(format, &random::bytes(bytes)?);
    s.truncate(length);
    Ok(s)
}

/// Token carrying exactly `bytes` random bytes in the given encoding (padding "=" is kept as legacy).
pub fn token_by_bytes(format: TokenFormat, bytes: usize) -> Result<String> {
    let data = random::bytes(bytes)?;
    Ok(match format {
        TokenFormat::Base64 => base64::engine::general_purpose::STANDARD.encode(&data),
        // Node's base64url is unpadded
        other => encode(other, &data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_exact() {
        let mut ints = CryptoInts::new();
        for f in [
            TokenFormat::Hex,
            TokenFormat::Base64,
            TokenFormat::Base64Url,
            TokenFormat::Alnum,
        ] {
            for len in [1usize, 5, 7, 50, 4096] {
                assert_eq!(token_by_length(f, len, &mut ints).map(|s| s.len()).ok(), Some(len));
            }
        }
    }

    #[test]
    fn by_bytes_pads_base64_like_node() {
        assert_eq!(token_by_bytes(TokenFormat::Base64, 1).map(|s| s.len()).ok(), Some(4));
        assert_eq!(
            token_by_bytes(TokenFormat::Base64Url, 24).map(|s| s.len()).ok(),
            Some(32)
        );
        assert_eq!(token_by_bytes(TokenFormat::Hex, 4).map(|s| s.len()).ok(), Some(8));
    }
}
