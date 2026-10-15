use hmac::{Mac, SimpleHmac};
use sha2::digest::DynDigest;

use crate::errors::{CliError, Result};

pub const HASH_ALGORITHMS: &[&str] = &[
    "md5",
    "sha1",
    "sha224",
    "sha256",
    "sha384",
    "sha512",
    "sha3-256",
    "sha3-512",
    "blake2b512",
];

/// Streaming hasher for a known algorithm name.
pub fn new_hasher(algorithm: &str) -> Option<Box<dyn DynDigest>> {
    Some(match algorithm {
        "md5" => Box::new(md5::Md5::default()),
        "sha1" => Box::new(sha1::Sha1::default()),
        "sha224" => Box::new(sha2::Sha224::default()),
        "sha256" => Box::new(sha2::Sha256::default()),
        "sha384" => Box::new(sha2::Sha384::default()),
        "sha512" => Box::new(sha2::Sha512::default()),
        "sha3-256" => Box::new(sha3::Sha3_256::default()),
        "sha3-512" => Box::new(sha3::Sha3_512::default()),
        "blake2b512" => Box::new(blake2::Blake2b512::default()),
        _ => return None,
    })
}

/// Digest length in bytes for an algorithm.
pub fn digest_length(algorithm: &str) -> usize {
    match algorithm {
        "md5" => 16,
        "sha1" => 20,
        "sha224" => 28,
        "sha256" | "sha3-256" => 32,
        "sha384" => 48,
        "sha512" | "sha3-512" | "blake2b512" => 64,
        _ => 0,
    }
}

fn mac_of<D>(key: &[u8], data: &[u8]) -> Result<Vec<u8>>
where
    D: sha2::Digest + sha2::digest::core_api::BlockSizeUser,
{
    let mut mac = <SimpleHmac<D> as Mac>::new_from_slice(key).map_err(|e| CliError::Failed(e.to_string()))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Digest of data; with `hmac_key`, an HMAC instead of a plain hash.
pub fn hash(data: &[u8], algorithm: &str, hmac_key: Option<&str>) -> Result<Vec<u8>> {
    if let Some(key) = hmac_key {
        let key = key.as_bytes();
        return match algorithm {
            "md5" => mac_of::<md5::Md5>(key, data),
            "sha1" => mac_of::<sha1::Sha1>(key, data),
            "sha224" => mac_of::<sha2::Sha224>(key, data),
            "sha256" => mac_of::<sha2::Sha256>(key, data),
            "sha384" => mac_of::<sha2::Sha384>(key, data),
            "sha512" => mac_of::<sha2::Sha512>(key, data),
            "sha3-256" => mac_of::<sha3::Sha3_256>(key, data),
            "sha3-512" => mac_of::<sha3::Sha3_512>(key, data),
            "blake2b512" => mac_of::<blake2::Blake2b512>(key, data),
            other => Err(CliError::Failed(format!("Algoritma tidak dikenal: {other}"))),
        };
    }
    let mut hasher =
        new_hasher(algorithm).ok_or_else(|| CliError::Failed(format!("Algoritma tidak dikenal: {algorithm}")))?;
    hasher.update(data);
    Ok(hasher.finalize().to_vec())
}
