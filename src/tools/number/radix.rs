use super::bigint::BigUint;
use super::jstext::js_trim;
use crate::errors::{Result, usage};

/// Output order of the JSON/text tables (binary, octal, decimal, hex), as in legacy `BASES`.
pub const BASES: [u32; 4] = [2, 8, 10, 16];
pub const WIDTHS: [u32; 4] = [8, 16, 32, 64];

pub fn base_prefix(base: u32) -> &'static str {
    match base {
        2 => "0b",
        8 => "0o",
        16 => "0x",
        _ => "",
    }
}

pub struct ParsedNumber {
    pub negative: bool,
    pub magnitude: BigUint,
    /// Base actually used: from the prefix, or the explicit --from.
    pub base: u32,
}

fn prefix_base(rest: &str) -> Option<u32> {
    let b = rest.as_bytes();
    if b.len() >= 2 && b[0] == b'0' {
        match b[1].to_ascii_lowercase() {
            b'x' => Some(16),
            b'b' => Some(2),
            b'o' => Some(8),
            _ => None,
        }
    } else {
        None
    }
}

/// Parses [sign][0x|0b|0o]digits with optional single `_` separators between digits.
pub fn parse_number(text: &str, from: Option<u32>) -> Result<ParsedNumber> {
    let trimmed = js_trim(text);
    let mut rest = trimmed;
    let mut negative = false;
    if let Some(r) = rest.strip_prefix('-') {
        negative = true;
        rest = r;
    } else if let Some(r) = rest.strip_prefix('+') {
        rest = r;
    }
    let mut base = from.unwrap_or(10);
    // With --from 16, "0b1" is plain hex digits, not a binary prefix.
    if let Some(p) = prefix_base(rest)
        && (from.is_none() || from == Some(p))
    {
        base = p;
        rest = &rest[2..];
    }
    if rest.is_empty() {
        return Err(usage(format!(
            "Angka tidak valid: {}",
            if trimmed.is_empty() { "(kosong)" } else { trimmed }
        )));
    }
    if rest.starts_with('_') || rest.ends_with('_') || rest.contains("__") {
        return Err(usage(format!("Pemisah _ hanya boleh di antara digit: {trimmed}")));
    }
    let digits = rest.replace('_', "");
    let magnitude = BigUint::parse_radix(&digits, base)
        .ok_or_else(|| usage(format!("Angka tidak valid untuk basis {base}: {trimmed}")))?;
    Ok(ParsedNumber {
        negative: negative && !magnitude.is_zero(),
        magnitude,
        base,
    })
}

fn group(digits: &str, size: usize) -> String {
    let target = digits.len().div_ceil(size) * size;
    let padded = format!("{}{digits}", "0".repeat(target - digits.len()));
    padded
        .as_bytes()
        .chunks(size)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Clone, Copy, Default)]
pub struct FormatOptions {
    pub prefix: bool,
    pub group: bool,
    /// Zero-pad digits to this count (two's complement).
    pub pad: usize,
}

/// Formats a value in `base`; negatives become -0x.. (sign before prefix). Grouping: binary by nibble, hex by byte.
pub fn format_base(negative: bool, magnitude: &BigUint, base: u32, options: FormatOptions) -> String {
    let mut digits = magnitude.to_radix_string(base);
    if options.pad > digits.len() {
        digits = format!("{}{digits}", "0".repeat(options.pad - digits.len()));
    }
    if options.group && base == 2 {
        digits = group(&digits, 4);
    } else if options.group && base == 16 {
        digits = group(&digits, 2);
    }
    format!(
        "{}{}{digits}",
        if negative { "-" } else { "" },
        if options.prefix { base_prefix(base) } else { "" }
    )
}

/// Two's complement bit pattern of the value for `width` bits; accepts the signed range and the unsigned range.
pub fn twos_complement(negative: bool, magnitude: &BigUint, width: u32) -> Result<BigUint> {
    let min_mag: u128 = 1u128 << (width - 1);
    let max: u128 = (1u128 << width) - 1;
    let fits = match magnitude.to_u128() {
        Some(m) => (negative && m <= min_mag) || (!negative && m <= max),
        None => false,
    };
    if !fits {
        let sign = if negative { "-" } else { "" };
        return Err(usage(format!(
            "Nilai {sign}{} tidak muat dalam {width} bit (-{min_mag} sampai {max}).",
            magnitude.to_radix_string(10)
        )));
    }
    let m = magnitude.to_u128().unwrap_or(0);
    let pattern = if negative { (1u128 << width) - m } else { m };
    Ok(BigUint::from_u64(pattern as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(neg: bool, mag: u64, base: u32, o: FormatOptions) -> String {
        format_base(neg, &BigUint::from_u64(mag), base, o)
    }

    #[test]
    fn parses_prefixes_and_separators() {
        let p = parse_number("0xFF_FF", None).unwrap();
        assert_eq!((p.base, p.magnitude.to_u128()), (16, Some(65535)));
        let p = parse_number("-0b101", None).unwrap();
        assert!(p.negative);
        assert_eq!(p.base, 2);
        let p = parse_number("0b1", Some(16)).unwrap();
        assert_eq!((p.base, p.magnitude.to_u128()), (16, Some(177)));
        assert!(parse_number("0b1", Some(10)).is_err());
        assert!(parse_number("_10", None).is_err());
        assert!(parse_number("1__0", None).is_err());
        assert!(parse_number("0x", None).is_err());
        assert!(!parse_number("-0", None).unwrap().negative);
    }

    #[test]
    fn grouping_and_padding() {
        let g = FormatOptions {
            group: true,
            ..Default::default()
        };
        assert_eq!(fmt(false, 5, 2, g), "0101");
        assert_eq!(fmt(false, 0xabc, 16, g), "0a_bc");
        assert_eq!(
            fmt(
                true,
                255,
                16,
                FormatOptions {
                    prefix: true,
                    ..Default::default()
                }
            ),
            "-0xff"
        );
        assert_eq!(
            fmt(
                false,
                1,
                2,
                FormatOptions {
                    pad: 8,
                    ..Default::default()
                }
            ),
            "00000001"
        );
    }

    #[test]
    fn twos_complement_ranges() {
        let one = BigUint::from_u64(1);
        assert_eq!(twos_complement(true, &one, 8).unwrap().to_u128(), Some(255));
        assert_eq!(
            twos_complement(true, &BigUint::from_u64(128), 8).unwrap().to_u128(),
            Some(128)
        );
        assert!(twos_complement(true, &BigUint::from_u64(129), 8).is_err());
        assert!(twos_complement(false, &BigUint::from_u64(256), 8).is_err());
        assert_eq!(
            twos_complement(true, &one, 64).unwrap().to_u128(),
            Some(u64::MAX as u128)
        );
    }
}
