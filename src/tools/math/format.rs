use crate::numfmt::js_number;
fn scientific(v: f64, precision: Option<usize>) -> String {
    use crate::tools::number::bigint::BigUint;
    let significant = precision.unwrap_or(11) + 1;
    if v == 0.0 {
        return if let Some(p) = precision {
            if p == 0 {
                "0e+0".into()
            } else {
                format!("0.{}e+0", "0".repeat(p))
            }
        } else {
            "0e+0".into()
        };
    }
    let bits = v.abs().to_bits();
    let e = ((bits >> 52) & 2047) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | if e == 0 { 0 } else { 1u64 << 52 };
    let shift = if e == 0 { -1074 } else { e - 1075 };
    let mut n = BigUint::from_u64(mantissa);
    let scale = if shift >= 0 {
        n.shl(shift as usize);
        0
    } else {
        for _ in 0..-shift {
            n.mul_u64(5);
        }
        -shift as usize
    };
    let digits = n.to_radix_string(10);
    let mut exponent = digits.len() as i32 - scale as i32 - 1;
    let mut prefix = digits[..digits.len().min(significant)].to_owned();
    prefix.extend(std::iter::repeat_n('0', significant - prefix.len()));
    if digits.as_bytes().get(significant).is_some_and(|b| *b >= b'5') {
        let mut rounded = BigUint::parse_radix(&prefix, 10).unwrap_or_else(BigUint::zero);
        rounded.add_u64(1);
        prefix = rounded.to_radix_string(10);
        if prefix.len() > significant {
            exponent += 1;
            prefix.pop();
        }
    }
    let mut mantissa = prefix[..1].to_owned();
    if significant > 1 {
        let fraction = if precision.is_none() {
            prefix[1..].trim_end_matches('0')
        } else {
            &prefix[1..]
        };
        if !fraction.is_empty() {
            mantissa.push('.');
            mantissa.push_str(fraction);
        }
    }
    format!("{}{mantissa}e{exponent:+}", if v < 0.0 { "-" } else { "" })
}
fn fixed(v: f64, p: usize) -> String {
    use crate::tools::number::bigint::BigUint;
    let bits = v.abs().to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let shift = if exponent == 0 { -1074 } else { exponent - 1075 } + p as i32;
    let mut n = BigUint::from_u64(mantissa);
    n.mul_u64(5u64.pow(p as u32));
    if shift >= 0 {
        n.shl(shift as usize);
    } else {
        let mut last = 0;
        for _ in 0..-shift {
            last = n.divmod_u64(2);
        }
        if last == 1 {
            n.add_u64(1);
        }
    }
    let digits = n.to_radix_string(10);
    let digits = format!("{digits:0>width$}", width = p + 1);
    let sign = if v < 0.0 && !n.is_zero() { "-" } else { "" };
    if p == 0 {
        format!("{sign}{digits}")
    } else {
        let (whole, fraction) = digits.split_at(digits.len() - p);
        format!("{sign}{whole}.{fraction}")
    }
}
pub fn format_number(v: f64, p: Option<usize>, sci: bool) -> String {
    if v == f64::INFINITY {
        return "inf".into();
    }
    if v == f64::NEG_INFINITY {
        return "-inf".into();
    }
    if sci {
        return scientific(v, p);
    }
    if let Some(p) = p {
        return fixed(v, p);
    }
    if v.fract() == 0.0 && v.abs() < 1e21 {
        return fixed(v, 0);
    }
    let rounded = scientific(v, None).parse::<f64>().unwrap_or(v);
    if rounded.fract() == 0.0 && rounded.abs() < 1e21 {
        fixed(rounded, 0)
    } else {
        js_number(rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_rounding_and_exact_integers() {
        assert_eq!(format_number(1.25, Some(1), false), "1.3");
        assert_eq!(format_number(1.25, Some(1), true), "1.3e+0");
        assert_eq!(format_number(-0.0, None, true), "0e+0");
        assert_eq!(format_number(2f64.powi(64), None, false), "18446744073709551616");
        assert_eq!(
            format_number(2f64.powi(70), Some(2), false),
            "1180591620717411303424.00"
        );
    }
}
