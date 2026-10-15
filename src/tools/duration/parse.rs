//! Parsing of compact (`1h30m`) and ISO-8601 (`PT1H30M`) durations into exact milliseconds.

use crate::errors::{Result, usage};

/// Largest accepted magnitude in milliseconds (`i64::MAX`, about 292 million years).
pub const MAX_MS: u128 = i64::MAX as u128;

pub const MS_PER_SECOND: u128 = 1000;
pub const MS_PER_MINUTE: u128 = 60 * MS_PER_SECOND;
pub const MS_PER_HOUR: u128 = 60 * MS_PER_MINUTE;
pub const MS_PER_DAY: u128 = 24 * MS_PER_HOUR;
pub const MS_PER_WEEK: u128 = 7 * MS_PER_DAY;

/// Compact units; lookup is by whole alphabetic token.
const COMPACT_UNITS: [(&str, u128); 6] = [
    ("w", MS_PER_WEEK),
    ("d", MS_PER_DAY),
    ("h", MS_PER_HOUR),
    ("m", MS_PER_MINUTE),
    ("s", MS_PER_SECOND),
    ("ms", 1),
];

/// Units that exist in everyday speech but have no fixed length.
const AMBIGUOUS_UNITS: [&str; 11] = [
    "y", "yr", "yrs", "year", "years", "mo", "mon", "month", "months", "M", "Y",
];

/// A signed duration; `negative` is never set for zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub negative: bool,
    pub ms: u128,
}

impl Span {
    pub fn new(negative: bool, ms: u128) -> Self {
        Self {
            negative: negative && ms != 0,
            ms,
        }
    }

    /// Builds a span from a signed millisecond difference.
    pub fn from_signed(ms: i128) -> Self {
        Self::new(ms < 0, ms.unsigned_abs())
    }

    /// -1, 0 or 1.
    pub fn sign(&self) -> i8 {
        if self.ms == 0 {
            0
        } else if self.negative {
            -1
        } else {
            1
        }
    }
}

fn ambiguous(unit: &str) -> crate::errors::CliError {
    usage(format!(
        "Satuan {unit} tidak didukung: tahun dan bulan panjangnya tidak tetap. Pakai w, d, h, m, s, ms."
    ))
}

/// Splits a decimal number (`12`, `1.5`; `,` also accepted when `comma`) off the front of `s`.
fn number(s: &str, comma: bool) -> Option<(&str, &str, &str)> {
    let int_len = s.bytes().take_while(u8::is_ascii_digit).count();
    if int_len == 0 {
        return None;
    }
    let int = &s[..int_len];
    let rest = &s[int_len..];
    if let Some(after) = rest
        .strip_prefix('.')
        .or_else(|| rest.strip_prefix(',').filter(|_| comma))
    {
        let frac_len = after.bytes().take_while(u8::is_ascii_digit).count();
        if frac_len == 0 {
            return None;
        }
        return Some((int, &after[..frac_len], &after[frac_len..]));
    }
    Some((int, "", rest))
}

fn digits_value(digits: &str, text: &str) -> Result<u128> {
    digits.bytes().try_fold(0u128, |acc, b| {
        acc.checked_mul(10)
            .and_then(|v| v.checked_add(u128::from(b - b'0')))
            .ok_or_else(|| overflow(text))
    })
}

fn overflow(text: &str) -> crate::errors::CliError {
    usage(format!("Durasi terlalu besar (maksimum {MAX_MS} ms): {text}"))
}

/// `int.frac` times `unit` milliseconds, which must be a whole number of milliseconds.
fn component_ms(int: &str, frac: &str, unit: u128, text: &str) -> Result<u128> {
    let mut ms = digits_value(int, text)?
        .checked_mul(unit)
        .ok_or_else(|| overflow(text))?;
    let frac = frac.trim_end_matches('0');
    if !frac.is_empty() {
        // frac_value * unit stays far below u128::MAX for <= 28 digits (unit < 2^30).
        let scale = u32::try_from(frac.len())
            .ok()
            .filter(|&n| n <= 28)
            .map(|n| 10u128.pow(n))
            .ok_or_else(|| precision(text))?;
        let part = digits_value(frac, text)? * unit;
        if !part.is_multiple_of(scale) {
            return Err(precision(text));
        }
        ms = ms.checked_add(part / scale).ok_or_else(|| overflow(text))?;
    }
    if ms > MAX_MS {
        return Err(overflow(text));
    }
    Ok(ms)
}

fn precision(text: &str) -> crate::errors::CliError {
    usage(format!("Presisi di bawah 1 ms tidak didukung: {text}"))
}

fn add(total: u128, part: u128, text: &str) -> Result<u128> {
    total
        .checked_add(part)
        .filter(|t| *t <= MAX_MS)
        .ok_or_else(|| overflow(text))
}

/// Parses a duration; a leading `-` (or `+`) applies to the whole value.
pub fn parse(text: &str) -> Result<Span> {
    let text = text.trim();
    if text.is_empty() {
        return Err(usage("Durasi kosong."));
    }
    let (negative, body) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    if body.is_empty() {
        return Err(usage(format!("Durasi kosong setelah tanda: {text}")));
    }
    let ms = if body.starts_with(['P', 'p']) {
        parse_iso(&body[1..], text)?
    } else {
        parse_compact(body, text)?
    };
    Ok(Span::new(negative, ms))
}

fn parse_compact(body: &str, text: &str) -> Result<u128> {
    let mut rest = body;
    let mut seen = [false; COMPACT_UNITS.len()];
    let mut total = 0u128;
    let mut first = true;
    while !rest.is_empty() {
        let (int, frac, after) = number(rest, false).ok_or_else(|| {
            usage(format!(
                "Durasi tidak valid: {text}. Contoh: 1h30m, 90m, 1.5h, 250ms, PT1H30M."
            ))
        })?;
        let unit_len = after.bytes().take_while(u8::is_ascii_alphabetic).count();
        if unit_len == 0 {
            if first && after.is_empty() {
                // A lone number is seconds.
                return component_ms(int, frac, MS_PER_SECOND, text);
            }
            return Err(usage(format!(
                "Angka tanpa satuan: {int}. Tambahkan w, d, h, m, s, atau ms."
            )));
        }
        let unit = &after[..unit_len];
        let Some(index) = COMPACT_UNITS.iter().position(|(name, _)| *name == unit) else {
            return Err(if AMBIGUOUS_UNITS.contains(&unit) {
                ambiguous(unit)
            } else {
                usage(format!("Satuan tidak dikenal: {unit}. Pilihan: w, d, h, m, s, ms."))
            });
        };
        if std::mem::replace(&mut seen[index], true) {
            return Err(usage(format!("Satuan {unit} muncul lebih dari sekali: {text}")));
        }
        total = add(total, component_ms(int, frac, COMPACT_UNITS[index].1, text)?, text)?;
        rest = after[unit_len..].trim_start();
        first = false;
    }
    Ok(total)
}

/// ISO-8601 designators in their mandatory order: (designator, date part?, ms).
const ISO_UNITS: [(char, bool, u128); 5] = [
    ('W', true, MS_PER_WEEK),
    ('D', true, MS_PER_DAY),
    ('H', false, MS_PER_HOUR),
    ('M', false, MS_PER_MINUTE),
    ('S', false, MS_PER_SECOND),
];

fn parse_iso(body: &str, text: &str) -> Result<u128> {
    let invalid = || {
        usage(format!(
            "Durasi ISO-8601 tidak valid: {text}. Contoh: PT1H30M, P1DT2H, P2W."
        ))
    };
    let mut rest = body;
    let mut in_time = false;
    let mut time_parts = 0usize;
    let mut parts = 0usize;
    let mut last: Option<usize> = None;
    let mut total = 0u128;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix(['T', 't']) {
            if in_time {
                return Err(invalid());
            }
            in_time = true;
            rest = after;
            continue;
        }
        let (int, frac, after) = number(rest, true).ok_or_else(invalid)?;
        let letter = after.chars().next().ok_or_else(invalid)?;
        let designator = letter.to_ascii_uppercase();
        if !in_time && matches!(designator, 'Y' | 'M') {
            return Err(ambiguous(&designator.to_string()));
        }
        let index = ISO_UNITS
            .iter()
            .position(|(d, date, _)| *d == designator && *date != in_time)
            .ok_or_else(invalid)?;
        match last {
            Some(l) if l == index => {
                return Err(usage(format!("Komponen {designator} muncul lebih dari sekali: {text}")));
            }
            Some(l) if l > index => {
                return Err(usage(format!(
                    "Urutan komponen ISO-8601 harus W, D, lalu T H M S: {text}"
                )));
            }
            _ => {}
        }
        last = Some(index);
        total = add(total, component_ms(int, frac, ISO_UNITS[index].2, text)?, text)?;
        parts += 1;
        time_parts += usize::from(in_time);
        rest = &after[letter.len_utf8()..];
    }
    if parts == 0 || (in_time && time_parts == 0) {
        return Err(invalid());
    }
    Ok(total)
}
