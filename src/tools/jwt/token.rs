use crate::errors::{CliError, Result, usage};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::NaiveDate;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Sha256, Sha384, Sha512};
use subtle::ConstantTimeEq;

fn failed(s: impl Into<String>) -> CliError {
    CliError::Failed(s.into())
}
pub fn space(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub fn normalize(text: &str) -> &str {
    let text = text.trim_matches(space);
    if text.get(..6).is_some_and(|p| p.eq_ignore_ascii_case("bearer")) && text[6..].starts_with(space) {
        text[6..].trim_start_matches(space)
    } else {
        text
    }
}
fn bytes(segment: &str, label: &str) -> Result<Vec<u8>> {
    let invalid = || failed(format!("{label} bukan base64url yang valid."));
    let bytes = URL_SAFE_NO_PAD.decode(segment).map_err(|_| invalid())?;
    if URL_SAFE_NO_PAD.encode(&bytes) != segment {
        return Err(invalid());
    }
    Ok(bytes)
}
pub fn parse_value(text: &str) -> std::result::Result<Value, serde_json::Error> {
    let prepared = crate::tools::json::document::prepare_strings(text);
    let mut de = serde_json::Deserializer::from_str(&prepared);
    de.disable_recursion_limit();
    let value = Value::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}
fn object(segment: &str, label: &str) -> Result<Value> {
    let decoded = bytes(segment, label)?;
    let value = parse_value(&String::from_utf8_lossy(&decoded))
        .map_err(|_| failed(format!("{label} bukan JSON yang valid.")))?;
    if !value.is_object() {
        return Err(failed(format!("{label} harus berupa objek JSON.")));
    }
    Ok(value)
}
pub struct Decoded {
    pub header: Value,
    pub payload: Value,
    pub input: String,
    pub signature: Vec<u8>,
}
pub fn decode(token: &str) -> Result<Decoded> {
    if token.is_empty() {
        return Err(failed("Token kosong."));
    }
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(failed(format!(
            "Token harus terdiri dari 3 segmen dipisah titik, ditemukan {}.",
            parts.len()
        )));
    }
    Ok(Decoded {
        header: object(parts[0], "Header")?,
        payload: object(parts[1], "Payload")?,
        input: format!("{}.{}", parts[0], parts[1]),
        signature: bytes(parts[2], "Signature")?,
    })
}
pub fn mac(alg: &str, key: &str, input: &str) -> Result<Vec<u8>> {
    macro_rules! hmac {
        ($hash:ty) => {{
            let mut h = Hmac::<$hash>::new_from_slice(key.as_bytes()).map_err(|_| failed("Kunci HMAC tidak valid."))?;
            h.update(input.as_bytes());
            h.finalize().into_bytes().to_vec()
        }};
    }
    Ok(match alg {
        "HS256" => hmac!(Sha256),
        "HS384" => hmac!(Sha384),
        "HS512" => hmac!(Sha512),
        _ => return Err(failed("Algoritma HMAC tidak valid.")),
    })
}
pub fn verify(decoded: &Decoded, alg: &str, key: &str) -> Result<bool> {
    let expected = mac(alg, key, &decoded.input)?;
    Ok(expected.len() == decoded.signature.len() && bool::from(expected.ct_eq(&decoded.signature)))
}
pub fn stringify(value: &Value, indent: usize) -> Result<String> {
    crate::tools::json::document::stringify_jwt(value, indent)
}
pub fn sign(header: &Value, payload: &Value, alg: &str, key: &str) -> Result<String> {
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(stringify(header, 0)?),
        URL_SAFE_NO_PAD.encode(stringify(payload, 0)?)
    );
    Ok(format!("{input}.{}", URL_SAFE_NO_PAD.encode(mac(alg, key, &input)?)))
}
pub fn now(value: &str) -> Result<f64> {
    let invalid = || {
        usage(format!(
            "Nilai --now tidak valid: {value}. Pakai ISO-8601 atau epoch detik."
        ))
    };
    let numeric = regex::Regex::new(r"^\d+(\.\d+)?$").map_err(|e| failed(e.to_string()))?;
    if numeric.is_match(value) {
        return value.parse().map_err(|_| invalid());
    }
    let re = regex::Regex::new(
        r"(?i)^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(\.\d+)?)?(?:(Z)|([+-])(\d{2}):?(\d{2}))?)?$",
    )
    .map_err(|e| failed(e.to_string()))?;
    let m = re.captures(value).ok_or_else(invalid)?;
    let part = |n: usize| m.get(n).and_then(|v| v.as_str().parse::<u32>().ok()).unwrap_or(0);
    let date = NaiveDate::from_ymd_opt(part(1) as i32, part(2), part(3)).ok_or_else(invalid)?;
    let dt = date.and_hms_opt(part(4), part(5), part(6)).ok_or_else(invalid)?;
    if part(10) >= 24 || part(11) >= 60 {
        return Err(invalid());
    }
    let fraction = m.get(7).and_then(|v| v.as_str().parse::<f64>().ok()).unwrap_or(0.0);
    let offset = (part(10) * 3600 + part(11) * 60) as f64
        * if m.get(9).is_some_and(|v| v.as_str() == "-") {
            -1.0
        } else {
            1.0
        };
    Ok(dt.and_utc().timestamp() as f64 + fraction - offset)
}
pub fn expiry(value: &str, now: f64) -> Result<f64> {
    let invalid = || {
        usage(format!(
            "Nilai --exp tidak valid: {value}. Pakai durasi (30s, 15m, 1h, 7d, 2w) atau epoch detik."
        ))
    };
    let (digits, unit) = if let Some(last) = value.chars().next_back().filter(|c| "smhdw".contains(*c)) {
        (&value[..value.len() - 1], Some(last))
    } else {
        (value, None)
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let n = digits.parse::<f64>().unwrap_or(f64::INFINITY);
    let result = match unit {
        Some(u) => {
            now.floor()
                + n * match u {
                    's' => 1.0,
                    'm' => 60.0,
                    'h' => 3600.0,
                    'd' => 86400.0,
                    _ => 604800.0,
                }
        }
        None => n,
    };
    if !result.is_finite() || result.fract() != 0.0 || result.abs() > 9_007_199_254_740_991.0 {
        return Err(usage(format!("Nilai --exp terlalu besar: {value}")));
    }
    Ok(result)
}
fn span(seconds: f64) -> String {
    let n = seconds.abs().floor();
    let (n, unit) = if n < 60.0 {
        (n, "detik")
    } else if n < 3600.0 {
        ((n / 60.0).floor(), "menit")
    } else if n < 86400.0 {
        ((n / 3600.0).floor(), "jam")
    } else {
        ((n / 86400.0).floor(), "hari")
    };
    format!("{} {unit}", crate::numfmt::js_number(n))
}
// Date's ±100,000,000-day range is wider than chrono's supported years.
fn iso_millis(millis: i64) -> String {
    let z = millis.div_euclid(86_400_000) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let year = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{}{:06}", if year < 0 { "-" } else { "+" }, year.abs())
    };
    let rest = millis.rem_euclid(86_400_000);
    format!(
        "{year}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000
    )
}
pub struct Claim {
    pub name: &'static str,
    pub iso: Option<String>,
    pub status: String,
    pub invalid: bool,
}
impl Claim {
    pub fn line(&self) -> String {
        format!(
            "{}: {} ({})",
            self.name,
            self.iso.as_deref().unwrap_or("di luar rentang waktu"),
            self.status
        )
    }
}
pub fn claim(name: &'static str, value: &Value, now: f64) -> Option<Claim> {
    let raw = value.as_number()?.as_str();
    let value = raw.parse::<f64>().ok()?;
    if value.is_nan() {
        return None;
    }
    let millis = (value * 1000.0).trunc();
    let iso = if millis.is_finite() && millis.abs() <= 8.64e15 {
        Some(iso_millis(millis as i64))
    } else {
        None
    };
    let diff = value - now;
    let distance = span(diff);
    let (status, invalid) = match name {
        "exp" => {
            let expired = value <= now;
            (
                if !value.is_finite() {
                    if expired {
                        "kedaluwarsa".into()
                    } else {
                        "tidak pernah kedaluwarsa".into()
                    }
                } else if expired {
                    format!("kedaluwarsa {distance} lalu")
                } else {
                    format!("berlaku {distance} lagi")
                },
                expired,
            )
        }
        "nbf" => (
            if value > now {
                if value.is_finite() {
                    format!("belum berlaku, {distance} lagi")
                } else {
                    "belum berlaku".into()
                }
            } else {
                "sudah berlaku".into()
            },
            value > now,
        ),
        _ => (
            if !value.is_finite() {
                if diff > 0.0 {
                    "di masa depan".into()
                } else {
                    "di masa lalu".into()
                }
            } else if diff > 0.0 {
                format!("{distance} lagi (di masa depan)")
            } else {
                format!("{distance} lalu")
            },
            false,
        ),
    };
    Some(Claim {
        name,
        iso,
        status,
        invalid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_helpers() {
        assert_eq!(normalize("  bEaReR  abc\n"), "abc");
        assert!(bytes("Zh", "X").is_err());
        assert_eq!(expiry("2w", 1.9).unwrap(), 1209601.0);
        assert!(expiry("9007199254740992", 0.0).is_err());
        assert_eq!(now("2024-02-29T07:00:00+07:00").unwrap(), 1709164800.0);
        assert_eq!(iso_millis(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_millis(-1), "1969-12-31T23:59:59.999Z");
        assert_eq!(iso_millis(8_640_000_000_000_000), "+275760-09-13T00:00:00.000Z");
        assert_eq!(iso_millis(-8_640_000_000_000_000), "-271821-04-20T00:00:00.000Z");
    }
}
