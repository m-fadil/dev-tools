use serde_json::{Map, Value};

/// A malformed or truncated image; the message is the reason only (the caller prefixes the file name).
#[derive(Debug, Clone)]
pub struct ImageError {
    pub message: String,
    /// Read past the end of the available bytes.
    pub short: bool,
}

pub type Res<T> = std::result::Result<T, ImageError>;

pub fn err<T>(message: impl Into<String>) -> Res<T> {
    Err(ImageError {
        message: message.into(),
        short: false,
    })
}

pub struct Parsed {
    pub width: u64,
    pub height: u64,
    /// The scan stopped at the read limit before the end of the file.
    pub truncated: bool,
    pub warnings: Vec<String>,
    /// Format specific fields, serialized flat into the result.
    pub details: Map<String, Value>,
}

const MIB: usize = 1024 * 1024;

/// JS `String.prototype.trim` whitespace set.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    })
}

/// A JS number (finite) as a JSON value, printed the way JS prints it.
pub fn num(n: f64) -> Value {
    serde_json::from_str(&crate::numfmt::js_number(n)).unwrap_or(Value::Null)
}

/// `+(len / MiB).toFixed(1)` as JS prints it.
fn mib(len: usize) -> String {
    // len * 10 is exact in f64 for any realistic size; ties round up like toFixed.
    let n = ((len as f64) * 10.0 / MIB as f64 + 0.5).floor() as u64;
    if n.is_multiple_of(10) {
        format!("{}", n / 10)
    } else {
        format!("{}.{}", n / 10, n % 10)
    }
}

/// Bounds-checked view over the bytes read from a file; every accessor fails with a short-data error instead of
/// reading out of range.
pub struct Src<'a> {
    pub b: &'a [u8],
    pub file_size: u64,
    /// All bytes of the file are present (the read limit was not hit).
    pub complete: bool,
}

impl<'a> Src<'a> {
    pub fn new(b: &'a [u8], file_size: u64) -> Self {
        Src {
            b,
            file_size,
            complete: b.len() as u64 >= file_size,
        }
    }

    pub fn length(&self) -> usize {
        self.b.len()
    }

    pub fn has(&self, off: usize, len: usize) -> bool {
        off.checked_add(len).is_some_and(|e| e <= self.b.len())
    }

    pub fn need(&self, off: usize, len: usize, what: &str) -> Res<()> {
        if self.has(off, len) {
            return Ok(());
        }
        let message = if self.complete {
            format!("file terpotong ({what})")
        } else {
            format!("{what} melewati batas baca {} MiB", mib(self.b.len()))
        };
        Err(ImageError { message, short: true })
    }

    pub fn u8(&self, off: usize) -> Res<u8> {
        self.u8w(off, "data")
    }

    pub fn u8w(&self, off: usize, what: &str) -> Res<u8> {
        self.need(off, 1, what)?;
        Ok(self.b[off])
    }

    pub fn u16(&self, off: usize, little: bool) -> Res<u16> {
        self.u16w(off, little, "data")
    }

    pub fn u16w(&self, off: usize, little: bool, what: &str) -> Res<u16> {
        self.need(off, 2, what)?;
        let a = [self.b[off], self.b[off + 1]];
        Ok(if little {
            u16::from_le_bytes(a)
        } else {
            u16::from_be_bytes(a)
        })
    }

    pub fn u32(&self, off: usize, little: bool) -> Res<u32> {
        self.u32w(off, little, "data")
    }

    pub fn u32w(&self, off: usize, little: bool, what: &str) -> Res<u32> {
        self.need(off, 4, what)?;
        let a = [self.b[off], self.b[off + 1], self.b[off + 2], self.b[off + 3]];
        Ok(if little {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    }

    pub fn bytes(&self, off: usize, len: usize, what: &str) -> Res<&'a [u8]> {
        self.need(off, len, what)?;
        Ok(&self.b[off..off + len])
    }

    pub fn latin1(&self, off: usize, len: usize) -> Res<String> {
        self.latin1w(off, len, "data")
    }

    pub fn latin1w(&self, off: usize, len: usize, what: &str) -> Res<String> {
        Ok(self.bytes(off, len, what)?.iter().map(|&c| c as char).collect())
    }

    /// Warning for a scan that stopped at the read limit; `consequence` says what may be missing.
    pub fn limit_warning(&self, consequence: &str) -> String {
        format!(
            "hanya {} MiB pertama dari {} B dibaca; {consequence}",
            mib(self.b.len()),
            self.file_size
        )
    }
}

/// Latin-1 bytes to a string with control characters (Cc) replaced by U+FFFD.
pub fn replace_cc(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { '\u{FFFD}' } else { c }).collect()
}
