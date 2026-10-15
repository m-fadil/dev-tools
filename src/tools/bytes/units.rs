use crate::errors::{Result, usage};
use crate::tools::number::{bigint::BigUint, jstext::js_trim};
pub const NAMES: [&str; 11] = ["B", "KB", "MB", "GB", "TB", "PB", "KiB", "MiB", "GiB", "TiB", "PiB"];
pub fn factor(i: usize) -> u64 {
    if i == 0 {
        1
    } else if i <= 5 {
        1000u64.pow(i as u32)
    } else {
        1024u64.pow((i - 5) as u32)
    }
}
pub fn find(s: &str) -> Option<usize> {
    NAMES.iter().position(|n| n.eq_ignore_ascii_case(s))
}
pub fn parse_size(text: &str) -> Result<(BigUint, bool)> {
    let input = js_trim(text);
    if input.starts_with('-') {
        return Err(usage(format!("Ukuran tidak boleh negatif: {input}")));
    }
    let re = regex::Regex::new(r"^(?:([0-9]+)(?:\.([0-9]+))?|\.([0-9]+))\s*([a-zA-Z]*)$")
        .map_err(|e| usage(e.to_string()))?;
    let c = re
        .captures(input)
        .ok_or_else(|| usage(format!("Ukuran tidak valid: {text}. Contoh: 1536, 1.5GB, 2 GiB")))?;
    let whole = c.get(1).map_or("0", |m| m.as_str());
    let fraction = c.get(2).or_else(|| c.get(3)).map_or("", |m| m.as_str());
    let unit = c.get(4).map_or("", |m| m.as_str());
    let index = if unit.is_empty() {
        0
    } else {
        find(unit).ok_or_else(|| usage(format!("Satuan tidak dikenal: {unit}. Pilihan: {}", NAMES.join(", "))))?
    };
    let mut n = BigUint::parse_radix(&format!("{whole}{fraction}"), 10).ok_or_else(|| usage("Ukuran tidak valid"))?;
    n.mul_u64(factor(index));
    let digits = n.to_radix_string(10);
    let padded = format!("{:0>width$}", digits, width = fraction.len() + 1);
    let split = padded.len() - fraction.len();
    let (w, f) = padded.split_at(split);
    let rounded = f.bytes().any(|b| b != b'0');
    let mut n = BigUint::parse_radix(w, 10).ok_or_else(|| usage("Ukuran tidak valid"))?;
    if f.as_bytes().first().is_some_and(|b| *b >= b'5') {
        n.add_u64(1);
    }
    Ok((n, rounded))
}
pub fn format_value(n: &BigUint, index: usize, p: usize) -> String {
    let mut scaled = n.clone();
    scaled.mul_u64(10u64.pow(p as u32));
    let f = factor(index);
    let rem = scaled.divmod_u64(f);
    if u128::from(rem) * 2 >= u128::from(f) {
        scaled.add_u64(1);
    }
    let s = scaled.to_radix_string(10);
    if p == 0 {
        return s;
    }
    let s = format!("{:0>width$}", s, width = p + 1);
    let (w, f) = s.split_at(s.len() - p);
    let f = f.trim_end_matches('0');
    if f.is_empty() { w.into() } else { format!("{w}.{f}") }
}
pub fn readable(n: &BigUint, start: usize, p: usize) -> String {
    let mut i = 0;
    for j in start..start + 5 {
        if !n.cmp_u64(factor(j)).is_lt() {
            i = j;
        }
    }
    let mut v = format_value(n, i, p);
    let next = if i == 0 { start } else { i + 1 };
    if next < start + 5 && v == (factor(next) / factor(i)).to_string() {
        i = next;
        v = format_value(n, i, p);
    }
    format!("{v} {}", NAMES[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_fraction_and_unbounded_magnitude() {
        let (bytes, rounded) = parse_size("0.0000000000000005PB").unwrap();
        assert!(rounded);
        assert_eq!(bytes.to_radix_string(10), "1");
        let (bytes, rounded) = parse_size("123456789012345678901234567890.5").unwrap();
        assert!(rounded);
        assert_eq!(bytes.to_radix_string(10), "123456789012345678901234567891");
        assert_eq!(format_value(&bytes, 0, 10), "123456789012345678901234567891");
    }
}
