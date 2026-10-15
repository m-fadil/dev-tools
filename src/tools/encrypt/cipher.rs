use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};

use crate::errors::{CliError, Result};
use crate::random;

// Token layout (base64url): version(1) | salt(16) | iv(12) | auth tag(16) | ciphertext.
// AES-256-GCM, key = scrypt(password, salt) with the parameters below. The version byte is bound as GCM AAD and
// identifies the whole format (including scrypt parameters), so any change to them needs a new version.
const VERSION: u8 = 1;
const SALT: usize = 16;
const IV: usize = 12;
const TAG: usize = 16;
const SCRYPT_LOG_N: u8 = 14;
const AAD: [u8; 1] = [VERSION];

fn cipher(password: &str, salt: &[u8]) -> Result<Aes256Gcm> {
    let params = scrypt::Params::new(SCRYPT_LOG_N, 8, 1, 32).map_err(|e| CliError::Failed(e.to_string()))?;
    let mut key = [0u8; 32];
    scrypt::scrypt(password.as_bytes(), salt, &params, &mut key).map_err(|e| CliError::Failed(e.to_string()))?;
    Aes256Gcm::new_from_slice(&key).map_err(|e| CliError::Failed(e.to_string()))
}

pub fn encrypt(plain: &[u8], password: &str) -> Result<String> {
    let salt = random::bytes(SALT)?;
    let iv = random::bytes(IV)?;
    let sealed = cipher(password, &salt)?
        .encrypt(Nonce::from_slice(&iv), Payload { msg: plain, aad: &AAD })
        .map_err(|_| CliError::Failed("Enkripsi gagal.".into()))?;
    let (body, tag) = sealed.split_at(sealed.len() - TAG);
    let mut raw = Vec::with_capacity(1 + SALT + IV + TAG + body.len());
    raw.push(VERSION);
    raw.extend_from_slice(&salt);
    raw.extend_from_slice(&iv);
    raw.extend_from_slice(tag);
    raw.extend_from_slice(body);
    Ok(base64url(&raw))
}

fn base64url(data: &[u8]) -> String {
    crate::codec::encode(data, crate::codec::Codec::Base64Url)
}

/// Lenient base64/base64url decoder like Node's `Buffer.from(s, 'base64url')`: both alphabets are accepted,
/// unknown characters are skipped and decoding stops at the first `=`.
fn lenient_base64(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

pub fn decrypt(token: &str, password: &str) -> Result<Vec<u8>> {
    let raw = lenient_base64(token.trim());
    if raw.len() < 1 + SALT + IV + TAG || raw[0] != VERSION {
        return Err(CliError::Failed("Format data terenkripsi tidak dikenal.".into()));
    }
    let salt = &raw[1..1 + SALT];
    let iv = &raw[1 + SALT..1 + SALT + IV];
    let tag = &raw[1 + SALT + IV..1 + SALT + IV + TAG];
    let body = &raw[1 + SALT + IV + TAG..];
    let mut sealed = Vec::with_capacity(body.len() + TAG);
    sealed.extend_from_slice(body);
    sealed.extend_from_slice(tag);
    cipher(password, salt)?
        .decrypt(
            Nonce::from_slice(iv),
            Payload {
                msg: &sealed,
                aad: &AAD,
            },
        )
        .map_err(|_| CliError::Failed("Dekripsi gagal: password salah atau data rusak.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = encrypt(b"halo", "rahasia").unwrap();
        assert_eq!(decrypt(&t, "rahasia").unwrap(), b"halo");
        assert!(decrypt(&t, "salah").is_err());
    }

    #[test]
    fn lenient_decoder() {
        assert_eq!(lenient_base64("YQ==YQ=="), b"a");
        assert_eq!(lenient_base64("YQ YQ"), vec![0x61, 0x06, 0x10]);
        assert_eq!(lenient_base64("Y-_+Q"), vec![0x63, 0xef, 0xfe]);
    }
}
