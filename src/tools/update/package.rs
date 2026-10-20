use crate::errors::{CliError, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn failed(message: impl Into<String>) -> CliError {
    CliError::Failed(message.into())
}

/// Largest `.sha256` file accepted; a real one is ~100 bytes.
const MAX_CHECKSUM_FILE: u64 = 4096;

/// Hash from `sha256sum` output (`<hex>  <name>`) or a bare hex digest, lowercased.
pub fn parse_expected_hash(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    (token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())).then(|| token.to_ascii_lowercase())
}

/// Fails unless the SHA-256 of `archive` equals the digest in the `.sha256` file.
pub fn verify_checksum(archive: &Path, checksum_file: &Path, name: &str) -> Result<()> {
    let mut text = String::new();
    File::open(checksum_file)
        .and_then(|f| f.take(MAX_CHECKSUM_FILE).read_to_string(&mut text))
        .map_err(|e| failed(format!("Gagal membaca checksum {name}.sha256: {e}")))?;
    let Some(expected) = parse_expected_hash(&text) else {
        return Err(failed(format!("Format checksum {name}.sha256 tidak valid.")));
    };
    let mut hasher = Sha256::new();
    File::open(archive)
        .and_then(|mut f| std::io::copy(&mut f, &mut hasher))
        .map_err(|e| failed(format!("Gagal membaca {name}: {e}")))?;
    if hex::encode(hasher.finalize()) != expected {
        return Err(failed(format!("Checksum tidak cocok untuk {name}.")));
    }
    Ok(())
}

fn copy_binary(entry: &mut impl Read, out: &mut File, bin: &str) -> Result<()> {
    let written = std::io::copy(entry, out).map_err(|e| failed(format!("Gagal mengekstrak {bin}: {e}")))?;
    if written == 0 {
        return Err(failed(format!("Berkas {bin} di dalam arsip kosong.")));
    }
    out.flush().map_err(|e| failed(format!("Gagal menulis {bin}: {e}")))
}

/// Copies `<stem>/dt` out of the `.tar.gz` into `out`.
pub fn extract(archive: &Path, stem: &str, bin: &str, out: &mut File) -> Result<()> {
    let ctx = |e: std::io::Error| failed(format!("Gagal mengekstrak {stem}.tar.gz: {e}"));
    let file = File::open(archive).map_err(ctx)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let wanted = Path::new(stem).join(bin);
    for entry in tar.entries().map_err(ctx)? {
        let mut entry = entry.map_err(ctx)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let hit = entry.path().map_err(ctx)? == wanted;
        if hit {
            return copy_binary(&mut entry, out, bin);
        }
    }
    Err(failed(format!("Arsip tidak berisi {bin}.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn parses_sha256sum_line_and_bare_digest() {
        assert_eq!(
            parse_expected_hash(&format!("{HASH}  dt-v1.tar.gz\n")).as_deref(),
            Some(HASH)
        );
        assert_eq!(
            parse_expected_hash(&format!("{HASH} *dt-v1.tar.gz")).as_deref(),
            Some(HASH)
        );
        assert_eq!(parse_expected_hash(&HASH.to_uppercase()).as_deref(), Some(HASH));
    }

    #[test]
    fn rejects_malformed_digest() {
        assert_eq!(parse_expected_hash(""), None);
        assert_eq!(parse_expected_hash("abc  file"), None);
        assert_eq!(parse_expected_hash(&format!("{}zz", &HASH[..62])), None);
        assert_eq!(parse_expected_hash(&format!("{HASH}0")), None);
    }
}
