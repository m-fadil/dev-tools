use super::bytes::{ImageError, Parsed, Res, Src};
use super::gif::parse_gif;
use super::jpeg::parse_jpeg;
use super::png::parse_png;
use super::webp::parse_webp;
use serde_json::{Map, Value, json};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MIB: u64 = 1024 * 1024;

struct Format {
    name: &'static str,
    mime: &'static str,
    /// Max bytes read from the file.
    limit: u64,
    parse: fn(&Src) -> Res<Parsed>,
}

const PNG: Format = Format {
    name: "PNG",
    mime: "image/png",
    limit: 4 * MIB,
    parse: parse_png,
};
const JPEG: Format = Format {
    name: "JPEG",
    mime: "image/jpeg",
    limit: 4 * MIB,
    parse: parse_jpeg,
};
const GIF: Format = Format {
    name: "GIF",
    mime: "image/gif",
    limit: 64 * MIB,
    parse: parse_gif,
};
const WEBP: Format = Format {
    name: "WebP",
    mime: "image/webp",
    limit: 4 * MIB,
    parse: parse_webp,
};

/// Detects the format from magic bytes only.
fn detect(head: &[u8]) -> Option<&'static Format> {
    if head.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some(&PNG);
    }
    if head.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(&JPEG);
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return Some(&GIF);
    }
    if head.len() == 12 && head.starts_with(b"RIFF") && head.ends_with(b"WEBP") {
        return Some(&WEBP);
    }
    None
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Reads up to `length` bytes (bounded by the file size) from the start of the file.
fn read(file: &mut File, size: u64, length: u64) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(size.min(length) as usize);
    std::io::Seek::seek(file, std::io::SeekFrom::Start(0))?;
    file.take(size.min(length)).read_to_end(&mut buf)?;
    Ok(buf)
}

fn fs_reason(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "file tidak ditemukan".to_string(),
        PermissionDenied => "izin ditolak".to_string(),
        _ => e.to_string(),
    }
}

enum Failure {
    /// Complete message.
    Full(String),
    /// Reason, to be prefixed with the path.
    Reason(String),
}

impl From<std::io::Error> for Failure {
    fn from(e: std::io::Error) -> Self {
        Failure::Reason(fs_reason(&e))
    }
}

impl From<ImageError> for Failure {
    fn from(e: ImageError) -> Self {
        Failure::Reason(e.message)
    }
}

fn inspect(path: &Path) -> std::result::Result<Map<String, Value>, Failure> {
    let not_file = || Failure::Reason("bukan file biasa".into());
    if !std::fs::metadata(path)?.is_file() {
        return Err(not_file());
    }
    let mut file = File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(not_file());
    }
    let size = meta.len();
    let head = read(&mut file, size, 12)?;
    let shown = path.to_string_lossy();
    let Some(format) = detect(&head) else {
        return Err(Failure::Full(format!("format gambar tidak dikenal: {shown}")));
    };
    let data = read(&mut file, size, format.limit)?;
    let parsed = (format.parse)(&Src::new(&data, size))?;
    let divisor = gcd(parsed.width, parsed.height);
    let mut out = Map::new();
    out.insert("path".into(), json!(shown));
    out.insert("ok".into(), json!(true));
    out.insert("format".into(), json!(format.name));
    out.insert("mime".into(), json!(format.mime));
    out.insert("size".into(), json!(size));
    out.insert("width".into(), json!(parsed.width));
    out.insert("height".into(), json!(parsed.height));
    out.insert(
        "aspectRatio".into(),
        json!(format!("{}:{}", parsed.width / divisor, parsed.height / divisor)),
    );
    for (k, v) in parsed.details {
        out.insert(k, v);
    }
    out.insert("truncated".into(), json!(parsed.truncated));
    out.insert("warnings".into(), json!(parsed.warnings));
    Ok(out)
}

/// Reads at most the format's read limit and extracts metadata; never fails, failures become `{ ok: false, error }`.
pub fn inspect_file(path: &Path) -> Map<String, Value> {
    let shown = path.to_string_lossy();
    let message = match inspect(path) {
        Ok(result) => return result,
        Err(Failure::Full(m)) => m,
        Err(Failure::Reason(r)) => format!("{shown}: {r}"),
    };
    let mut out = Map::new();
    out.insert("path".into(), json!(shown));
    out.insert("ok".into(), json!(false));
    out.insert("error".into(), json!(message));
    out
}
