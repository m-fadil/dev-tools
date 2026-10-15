//! Exact integer formatting of a millisecond span.

use super::parse::{MS_PER_DAY, MS_PER_HOUR, MS_PER_MINUTE, MS_PER_SECOND, MS_PER_WEEK, Span};

/// Fractional digits kept for `--unit` results that have no finite decimal form (e.g. 1 ms in hours).
const MAX_FRACTION_DIGITS: u32 = 9;

/// Output units for `--unit`: name and milliseconds per unit.
pub const UNITS: &[(&str, u128)] = &[
    ("ms", 1),
    ("s", MS_PER_SECOND),
    ("m", MS_PER_MINUTE),
    ("h", MS_PER_HOUR),
    ("d", MS_PER_DAY),
    ("w", MS_PER_WEEK),
];

/// `1w 2d 3h 4m 5s 6ms` without zero parts; zero is `0s`.
pub fn compact(span: Span) -> String {
    if span.ms == 0 {
        return "0s".to_string();
    }
    let mut out = String::new();
    if span.negative {
        out.push('-');
    }
    let mut rest = span.ms;
    let mut first = true;
    for (suffix, unit) in [
        ("w", MS_PER_WEEK),
        ("d", MS_PER_DAY),
        ("h", MS_PER_HOUR),
        ("m", MS_PER_MINUTE),
        ("s", MS_PER_SECOND),
        ("ms", 1),
    ] {
        let n = rest / unit;
        rest %= unit;
        if n == 0 {
            continue;
        }
        if !first {
            out.push(' ');
        }
        first = false;
        out.push_str(&n.to_string());
        out.push_str(suffix);
    }
    out
}

/// ISO-8601: weeks only when the span is a whole number of weeks, otherwise days (`P8DT1H`),
/// the form accepted by the widest range of parsers. Milliseconds become fractional seconds.
pub fn iso(span: Span) -> String {
    let sign = if span.negative { "-" } else { "" };
    if span.ms == 0 {
        return "PT0S".to_string();
    }
    if span.ms.is_multiple_of(MS_PER_WEEK) {
        return format!("{sign}P{}W", span.ms / MS_PER_WEEK);
    }
    let mut out = format!("{sign}P");
    let days = span.ms / MS_PER_DAY;
    let mut rest = span.ms % MS_PER_DAY;
    if days > 0 {
        out.push_str(&format!("{days}D"));
    }
    if rest > 0 {
        out.push('T');
        let hours = rest / MS_PER_HOUR;
        rest %= MS_PER_HOUR;
        let minutes = rest / MS_PER_MINUTE;
        rest %= MS_PER_MINUTE;
        if hours > 0 {
            out.push_str(&format!("{hours}H"));
        }
        if minutes > 0 {
            out.push_str(&format!("{minutes}M"));
        }
        if rest > 0 {
            out.push_str(&format!("{}S", decimal(rest, MS_PER_SECOND, false)));
        }
    }
    out
}

/// `ms / unit` as a decimal string without trailing zeros. Integer arithmetic only: exact when
/// the quotient has a finite expansion within 9 digits, otherwise rounded half away from zero.
pub fn decimal(ms: u128, unit: u128, negative: bool) -> String {
    let scale = 10u128.pow(MAX_FRACTION_DIGITS);
    // ms <= i64::MAX (or 2 * 8.64e15 for diff), so ms * 1e9 < 1e28 fits in u128.
    let scaled = (ms * scale + unit / 2) / unit;
    let (int, frac) = (scaled / scale, scaled % scale);
    let sign = if negative && scaled != 0 { "-" } else { "" };
    if frac == 0 {
        return format!("{sign}{int}");
    }
    let frac = format!("{frac:09}");
    format!("{sign}{int}.{}", frac.trim_end_matches('0'))
}

/// Total of the span in `unit` milliseconds.
pub fn total(span: Span, unit: u128) -> String {
    decimal(span.ms, unit, span.negative)
}
