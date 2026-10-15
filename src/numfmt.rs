//! ECMAScript `Number::toString` formatting for f64.

/// Formats like JavaScript's `String(number)`.
pub fn js_number(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v == 0.0 {
        return "0".into();
    }
    let sign = if v < 0.0 { "-" } else { "" };
    let (digits, n) = shortest_digits(v.abs());
    let k = digits.len() as i32;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let exp = format!("{}{}", if e < 0 { '-' } else { '+' }, e.abs());
        if k == 1 {
            format!("{digits}e{exp}")
        } else {
            format!("{}.{}e{exp}", &digits[..1], &digits[1..])
        }
    };
    format!("{sign}{body}")
}

/// Shortest round-trip decimal digits and exponent `n` such that value = 0.DIGITS × 10^n.
fn shortest_digits(v: f64) -> (String, i32) {
    let mut buf = ryu::Buffer::new();
    let s = buf.format_finite(v);
    let (mantissa, exp) = match s.split_once('e') {
        Some((m, e)) => (m, e.parse::<i32>().unwrap_or(0)),
        None => (s, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all = format!("{int}{frac}");
    let trimmed_start = all.trim_start_matches('0');
    let leading = (all.len() - trimmed_start.len()) as i32;
    let digits = trimmed_start.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    (digits.to_string(), int.len() as i32 + exp - leading)
}

#[cfg(test)]
mod tests {
    use super::js_number;

    #[test]
    fn matches_ecmascript() {
        assert_eq!(js_number(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(1e-7), "1e-7");
        assert_eq!(js_number(123456789012345680000.0), "123456789012345680000");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(100.0), "100");
        assert_eq!(js_number(1.5), "1.5");
        assert_eq!(js_number(0.000001), "0.000001");
        assert_eq!(js_number(-2.5e-7), "-2.5e-7");
        assert_eq!(js_number(1.2345e25), "1.2345e+25");
        assert_eq!(js_number(f64::NAN), "NaN");
        assert_eq!(js_number(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(js_number(5e-324), "5e-324");
        assert_eq!(js_number(0.5), "0.5");
    }
}
