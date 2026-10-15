use std::fs::File;
use std::io::Read;
use std::path::Path;

use subtle::ConstantTimeEq;

use crate::tools::hash::digest::new_hasher;

/// Failure to hash a path; `reason` is a short message without the path, `code` the errno name if any.
pub struct HashFileError {
    pub reason: String,
    pub code: Option<&'static str>,
}

#[cfg(unix)]
fn errno_name(errno: i32) -> Option<&'static str> {
    const TABLE: &[(i32, &str)] = &[
        (libc::ENOENT, "ENOENT"),
        (libc::ENOTDIR, "ENOTDIR"),
        (libc::EACCES, "EACCES"),
        (libc::EPERM, "EPERM"),
        (libc::EISDIR, "EISDIR"),
        (libc::ELOOP, "ELOOP"),
        (libc::ENAMETOOLONG, "ENAMETOOLONG"),
        (libc::EIO, "EIO"),
        (libc::ENXIO, "ENXIO"),
        (libc::EMFILE, "EMFILE"),
        (libc::ENFILE, "ENFILE"),
        (libc::ENOMEM, "ENOMEM"),
        (libc::EOVERFLOW, "EOVERFLOW"),
    ];
    TABLE.iter().find(|(n, _)| *n == errno).map(|(_, name)| *name)
}

#[cfg(not(unix))]
fn errno_name(_errno: i32) -> Option<&'static str> {
    None
}

pub fn io_error(error: &std::io::Error) -> HashFileError {
    let code = match error.kind() {
        std::io::ErrorKind::NotFound => Some("ENOENT"),
        _ => error.raw_os_error().and_then(errno_name),
    };
    HashFileError {
        reason: code.map_or_else(|| error.to_string(), str::to_string),
        code,
    }
}

/// Streaming hex digest of a file; non-regular files and unreadable paths yield a HashFileError.
pub fn hash_file(path: &Path, algorithm: &str) -> Result<String, HashFileError> {
    let info = std::fs::metadata(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HashFileError {
                reason: "tidak ditemukan".into(),
                code: Some("ENOENT"),
            }
        } else {
            io_error(&e)
        }
    })?;
    if info.is_dir() {
        return Err(HashFileError {
            reason: "direktori".into(),
            code: None,
        });
    }
    if !info.is_file() {
        return Err(HashFileError {
            reason: "bukan file biasa".into(),
            code: None,
        });
    }
    let Some(mut hasher) = new_hasher(algorithm) else {
        return Err(HashFileError {
            reason: format!("algoritma tidak dikenal: {algorithm}"),
            code: None,
        });
    };
    let mut file = File::open(path).map_err(|e| io_error(&e))?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(io_error(&e)),
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

pub struct SumEntry {
    pub digest: String,
    pub path: String,
}

pub struct ParsedSums {
    pub entries: Vec<SumEntry>,
    /// 1-based numbers of malformed lines
    pub invalid: Vec<usize>,
}

fn is_js_space(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

/// Matches `^([0-9a-fA-F]+) [ *](.+)$` (JS `.` excludes \r, \u2028, \u2029).
fn parse_line(line: &str) -> Option<(&str, &str)> {
    let hex_len = line.bytes().take_while(u8::is_ascii_hexdigit).count();
    if hex_len == 0 {
        return None;
    }
    let (digest, rest) = line.split_at(hex_len);
    let path = rest.strip_prefix(' ')?;
    let path = path.strip_prefix(' ').or_else(|| path.strip_prefix('*'))?;
    if path.is_empty() || path.contains(['\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    Some((digest, path))
}

/// Parses `<hex>  <path>` / `<hex> *<path>` lines; blank lines and `#` comments are skipped.
pub fn parse_sums(text: &str, digest_length: usize) -> ParsedSums {
    let mut entries = Vec::new();
    let mut invalid = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.trim_matches(is_js_space).is_empty() || line.trim_start_matches(is_js_space).starts_with('#') {
            continue;
        }
        match parse_line(line) {
            Some((digest, path)) if digest.len() == digest_length * 2 => {
                entries.push(SumEntry {
                    digest: digest.to_string(),
                    path: path.to_string(),
                });
            }
            _ => invalid.push(index + 1),
        }
    }
    ParsedSums { entries, invalid }
}

/// Case-insensitive hex comparison in constant time.
pub fn digests_equal(a: &str, b: &str) -> bool {
    match (hex::decode(a), hex::decode(b)) {
        (Ok(x), Ok(y)) => x.len() == y.len() && bool::from(x.ct_eq(&y)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lines() {
        let h = "ab".repeat(16);
        let text = format!("# c\n\n{h}  a b.txt\n{h} *x\r\nxx  y\n{h}  \n");
        let p = parse_sums(&text, 16);
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[0].path, "a b.txt");
        assert_eq!(p.entries[1].path, "x");
        assert_eq!(p.invalid, vec![5, 6]);
    }

    #[test]
    fn equal_is_case_insensitive() {
        assert!(digests_equal("ABcd", "abCD"));
        assert!(!digests_equal("abcd", "abce"));
    }
}
