use crate::errors::{Result, usage};
use crate::numfmt::js_number;
use crate::tools::time::instant::{Wall, Zone, days_from_civil, format_year, parse_instant};
use regex::Regex;
use std::collections::BTreeSet;

pub const HORIZON_YEARS: i64 = 8;
pub struct Schedule {
    pub minute: Vec<i64>,
    pub hour: Vec<i64>,
    pub dom: Vec<i64>,
    pub month: Vec<i64>,
    pub dow: Vec<i64>,
    dom_star: bool,
    dow_star: bool,
}
const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
const MACROS: [(&str, &str); 7] = [
    ("@yearly", "0 0 1 1 *"),
    ("@annually", "0 0 1 1 *"),
    ("@monthly", "0 0 1 * *"),
    ("@weekly", "0 0 * * 0"),
    ("@daily", "0 0 * * *"),
    ("@midnight", "0 0 * * *"),
    ("@hourly", "0 * * * *"),
];
struct Spec {
    label: &'static str,
    min: i64,
    max: i64,
    names: &'static [&'static str],
    base: i64,
}
fn field_error(spec: &Spec, text: &str, reason: impl std::fmt::Display) -> crate::errors::CliError {
    usage(format!("Field {} tidak valid: \"{text}\" ({reason})", spec.label))
}
fn value(s: &Spec, field: &str, token: &str) -> Result<i64> {
    let n = if !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit()) {
        token.parse::<f64>().unwrap_or(f64::INFINITY)
    } else if !s.names.is_empty() && !token.is_empty() && token.bytes().all(|b| b.is_ascii_alphabetic()) {
        s.names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(token))
            .map(|i| i as f64 + s.base as f64)
            .ok_or_else(|| field_error(s, field, format!("nama tidak dikenal: {token}")))?
    } else {
        return Err(field_error(
            s,
            field,
            format!(
                "nilai bukan angka{}: {}",
                if s.names.is_empty() { "" } else { " atau nama" },
                if token.is_empty() { "(kosong)" } else { token }
            ),
        ));
    };
    if n < s.min as f64 || n > s.max as f64 {
        return Err(field_error(
            s,
            field,
            format!("{} di luar rentang {}-{}", js_number(n), s.min, s.max),
        ));
    }
    Ok(n as i64)
}
fn field(s: &Spec, text: &str) -> Result<Vec<i64>> {
    let mut set = BTreeSet::new();
    for item in text.split(',') {
        if item.is_empty() {
            return Err(field_error(s, text, "elemen kosong"));
        }
        let parts: Vec<_> = item.split('/').collect();
        if parts.len() > 2 {
            return Err(field_error(s, text, format!("terlalu banyak \"/\" di \"{item}\"")));
        }
        let step = if let Some(raw) = parts.get(1) {
            if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) || raw.parse::<f64>().unwrap_or(0.0) < 1.0 {
                return Err(field_error(
                    s,
                    text,
                    format!(
                        "step harus bilangan bulat >= 1: {}",
                        if raw.is_empty() { "(kosong)" } else { raw }
                    ),
                ));
            }
            raw.parse::<f64>().unwrap_or(f64::INFINITY)
        } else {
            1.0
        };
        let base = parts[0];
        let (lo, hi) = if base == "*" {
            (s.min, if s.max == 7 { 6 } else { s.max })
        } else if base.contains('-') {
            let range: Vec<_> = base.split('-').collect();
            if range.len() != 2 {
                return Err(field_error(s, text, format!("rentang salah: {base}")));
            }
            let lo = value(s, text, range[0])?;
            let hi = value(s, text, range[1])?;
            if lo > hi {
                return Err(field_error(s, text, format!("rentang terbalik: {base}")));
            }
            (lo, hi)
        } else {
            let lo = value(s, text, base)?;
            (lo, if parts.len() == 2 { s.max } else { lo })
        };
        let mut v = lo as f64;
        while v <= hi as f64 {
            set.insert(if s.max == 7 && v == 7.0 { 0 } else { v as i64 });
            v += step;
        }
    }
    Ok(set.into_iter().collect())
}
pub fn parse_cron(tokens: &[String]) -> Result<Schedule> {
    let mut tokens = tokens.to_vec();
    if tokens.len() == 1 && tokens[0].starts_with('@') {
        let expansion = MACROS.iter().find(|(key,_)| key.eq_ignore_ascii_case(&tokens[0])).map(|(_,v)| *v)
            .ok_or_else(|| usage(format!("Makro tidak dikenal: {}. Pilihan: @yearly, @annually, @monthly, @weekly, @daily, @midnight, @hourly", tokens[0])))?;
        tokens = expansion.split_whitespace().map(str::to_string).collect();
    }
    if tokens.len() != 5 {
        return Err(usage(format!(
            "Ekspresi cron harus 5 field (menit jam tanggal bulan hari), diterima {}: {}",
            tokens.len(),
            tokens.join(" ")
        )));
    }
    let specs = [
        Spec {
            label: "menit",
            min: 0,
            max: 59,
            names: &[],
            base: 0,
        },
        Spec {
            label: "jam",
            min: 0,
            max: 23,
            names: &[],
            base: 0,
        },
        Spec {
            label: "tanggal",
            min: 1,
            max: 31,
            names: &[],
            base: 0,
        },
        Spec {
            label: "bulan",
            min: 1,
            max: 12,
            names: &MONTHS,
            base: 1,
        },
        Spec {
            label: "hari",
            min: 0,
            max: 7,
            names: &DAYS,
            base: 0,
        },
    ];
    Ok(Schedule {
        minute: field(&specs[0], &tokens[0])?,
        hour: field(&specs[1], &tokens[1])?,
        dom: field(&specs[2], &tokens[2])?,
        month: field(&specs[3], &tokens[3])?,
        dow: field(&specs[4], &tokens[4])?,
        dom_star: tokens[2].starts_with('*'),
        dow_star: tokens[4].starts_with('*'),
    })
}
pub fn parse_from(text: &str, zone: &Zone) -> Result<i64> {
    let bad = || usage(format!("Nilai --from bukan waktu ISO-8601 yang valid: {text}"));
    let re = Regex::new(
        r"(?i-u)^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.\d{1,3})?)?(Z|[+-]\d{2}:\d{2})?)?$",
    )
    .map_err(|_| bad())?;
    let m = re.captures(text).ok_or_else(bad)?;
    // cron discards --from fractional seconds, unlike the time tool.
    let clean = format!(
        "{}-{}-{}T{}:{}:{}{}",
        &m[1],
        &m[2],
        &m[3],
        m.get(4).map(|v| v.as_str()).unwrap_or("00"),
        m.get(5).map(|v| v.as_str()).unwrap_or("00"),
        m.get(6).map(|v| v.as_str()).unwrap_or("00"),
        m.get(7).map(|v| v.as_str()).unwrap_or("")
    );
    parse_instant(&clean, zone, None).map_err(|_| bad())
}
fn search(s: &Schedule, mut start: i64, limit: i64) -> Option<i64> {
    while start <= limit {
        let mut w = Wall::from_ms(start);
        let dom = s.dom.contains(&w.day);
        let dow = s
            .dow
            .contains(&(days_from_civil(w.year, w.month, w.day) + 4).rem_euclid(7));
        let day_matches = if !s.dom_star && !s.dow_star {
            dom || dow
        } else {
            dom && dow
        };
        if !s.month.contains(&w.month) {
            if w.month == 12 {
                w.year += 1;
                w.month = 1;
            } else {
                w.month += 1;
            }
            w.day = 1;
            w.hour = 0;
            w.minute = 0;
            start = w.to_ms();
        } else if !day_matches {
            start = start.div_euclid(86_400_000) * 86_400_000 + 86_400_000;
        } else if !s.hour.contains(&w.hour) {
            start = start.div_euclid(3_600_000) * 3_600_000 + 3_600_000;
        } else if let Some(minute) = s.minute.iter().find(|m| **m >= w.minute) {
            w.minute = *minute;
            return Some(w.to_ms());
        } else {
            start = start.div_euclid(3_600_000) * 3_600_000 + 3_600_000;
        }
    }
    None
}
pub fn next_fires(s: &Schedule, from: i64, count: usize, zone: &Zone) -> (Vec<i64>, bool) {
    let mut times = Vec::with_capacity(count);
    let mut previous = from;
    let mut base = zone.local_ms(from).div_euclid(60_000) * 60_000;
    let mut start = base + 60_000;
    while times.len() < count {
        let mut limit = Wall::from_ms(base);
        limit.year += HORIZON_YEARS;
        // JS setUTCFullYear rolls Feb 29 into March in a non-leap target year.
        let limit_ms = limit.to_ms();
        loop {
            let Some(naive) = search(s, start, limit_ms) else {
                return (times, true);
            };
            start = naive + 60_000;
            let instant = zone.resolve_wall(naive);
            if zone.local_ms(instant) == naive && instant > previous {
                times.push(instant);
                previous = instant;
                base = naive;
                break;
            }
        }
    }
    (times, false)
}
pub fn format_time(ms: i64, zone: &Zone, utc: bool) -> String {
    let w = Wall::from_ms(zone.local_ms(ms));
    let prefix = format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}",
        format_year(w.year),
        w.month,
        w.day,
        w.hour,
        w.minute,
        w.second
    );
    if utc {
        format!("{prefix}Z")
    } else {
        let offset = zone.offset(ms) / 60_000;
        format!(
            "{prefix}{}{:02}:{:02}",
            if offset < 0 { "-" } else { "+" },
            offset.abs() / 60,
            offset.abs() % 60
        )
    }
}
pub fn explain(s: &Schedule) -> Vec<String> {
    let month_names = [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
    ];
    let day_names = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];
    let describe = |label: &str, v: &[i64], total: usize, every: &str, names: &[&str], base: i64| {
        format!(
            "{label}: {}",
            if v.len() == total {
                every.to_string()
            } else {
                v.iter()
                    .map(|n| {
                        if names.is_empty() {
                            n.to_string()
                        } else {
                            names[(n - base) as usize].to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
    };
    let mut lines = vec![
        describe("menit", &s.minute, 60, "setiap menit", &[], 0),
        describe("jam", &s.hour, 24, "setiap jam", &[], 0),
        describe("tanggal", &s.dom, 31, "setiap tanggal", &[], 0),
        describe("bulan", &s.month, 12, "setiap bulan", &month_names, 1),
        describe("hari", &s.dow, 7, "setiap hari", &day_names, 0),
    ];
    if !s.dom_star && !s.dow_star {
        lines.push("catatan: tanggal dan hari sama-sama dibatasi, cocok bila salah satunya terpenuhi (ATAU)".into());
    }
    lines
}
