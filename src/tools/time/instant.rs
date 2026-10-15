use crate::errors::{Result, usage};
use crate::numfmt::js_number;
use regex::Regex;

pub(crate) const MAX_MS: i64 = 8_640_000_000_000_000;
const DAY: i64 = 86_400_000;

pub(crate) fn whitespace(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | '\u{20}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

#[derive(Clone)]
enum ZoneKind {
    Named(jiff::tz::TimeZone),
    Fixed(i32),
    SystemV { standard: i32, daylight: bool },
}
#[derive(Clone)]
pub(crate) struct Zone {
    /// Zone id; for `invalid_default` zones, the id carried by the Intl error.
    pub name: String,
    pub missing_name: bool,
    pub invalid_default: bool,
    kind: ZoneKind,
    /// Offset rule used by Date-native local fields when it differs from the Intl zone.
    native: Option<ZoneKind>,
}

/// Outcome of interpreting the `TZ` environment variable like Node/ICU does.
enum EnvZone {
    Name(String),
    Zone(Zone),
}

const SYSTEM_V_STANDARD: [&str; 7] = [
    "SystemV/AST4",
    "SystemV/CST6",
    "SystemV/EST5",
    "SystemV/HST10",
    "SystemV/MST7",
    "SystemV/PST8",
    "SystemV/YST9",
];

impl Zone {
    fn unnamed(kind: ZoneKind) -> Self {
        Self {
            name: String::new(),
            missing_name: true,
            invalid_default: false,
            kind,
            native: None,
        }
    }
    fn invalid(name: &str) -> Self {
        Self {
            name: name.to_string(),
            invalid_default: true,
            ..Self::utc()
        }
    }
    /// ICU matches `TZ` case-sensitively; anything it cannot name either falls
    /// back to the system zone or leaves the Intl zone name undefined.
    fn from_env(tz: &str, system: impl Fn() -> String) -> EnvZone {
        if tz.is_empty() {
            return EnvZone::Zone(Self::invalid("Etc/Unknown"));
        }
        let id = tz.strip_prefix(':').unwrap_or(tz);
        let id = id
            .strip_prefix("posix/")
            .or_else(|| id.strip_prefix("right/"))
            .unwrap_or(id);
        if id == "Factory" || id == "Etc/Unknown" {
            return EnvZone::Zone(Self::invalid("Etc/Unknown"));
        }
        if jiff_tzdb::get(id).is_some_and(|(name, _)| name == id) {
            return EnvZone::Name(id.to_string());
        }
        if SYSTEM_V_STANDARD.contains(&tz) {
            return EnvZone::Name(tz.to_string());
        }
        let split = tz.find(|c: char| c.is_ascii_digit() || c == ',').unwrap_or(tz.len());
        let (prefix, rest) = tz.split_at(split);
        if !rest.is_empty() && !(rest.len() <= 2 && rest.bytes().all(|b| b.is_ascii_digit())) {
            return EnvZone::Name(system());
        }
        // `GMT+7` is a custom zone for Intl (sign kept, `+07:00`), while libc
        // (and so Date) reads it as POSIX, i.e. the opposite direction.
        let sign = prefix.bytes().last().filter(|b| matches!(b, b'+' | b'-'));
        let letters = if sign.is_some() {
            &prefix[..prefix.len() - 1]
        } else {
            prefix
        };
        let native_hours = if !rest.is_empty() && letters.len() >= 3 && letters.bytes().all(|b| b.is_ascii_alphabetic())
        {
            let hours = rest.parse::<i32>().unwrap_or(0).min(24);
            Some(if sign == Some(b'-') { hours } else { -hours } * 3600)
        } else {
            None
        };
        let native = ZoneKind::Fixed(native_hours.unwrap_or(0));
        if let (Some(sign), true, Some(seconds)) = (sign, letters.eq_ignore_ascii_case("gmt"), native_hours) {
            let hours = rest.parse::<i32>().unwrap_or(0);
            if hours == 0 {
                return EnvZone::Zone(Self::invalid(""));
            }
            if hours < 24 {
                let east = sign == b'+';
                return EnvZone::Zone(Self {
                    name: format!("{}{hours:02}:00", if east { '+' } else { '-' }),
                    missing_name: false,
                    invalid_default: false,
                    kind: ZoneKind::Fixed(if east { hours } else { -hours } * 3600),
                    native: Some(ZoneKind::Fixed(seconds)),
                });
            }
        }
        EnvZone::Zone(Self::unnamed(native))
    }
}

impl Zone {
    pub(crate) fn resolve(requested: Option<&str>) -> Result<Self> {
        let system = || iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into());
        let name = match requested {
            Some(name) => name.to_string(),
            None => match std::env::var("TZ") {
                Ok(tz) => match Self::from_env(&tz, system) {
                    EnvZone::Name(name) => name,
                    EnvZone::Zone(zone) => return Ok(zone),
                },
                Err(_) => system(),
            },
        };
        let name = if requested.is_none() {
            if name == "GMT" {
                "+00:00".to_string()
            } else if let Some((_, canonical)) = super::zones::ALIASES.iter().find(|(alias, _)| *alias == name) {
                (*canonical).to_string()
            } else {
                jiff_tzdb::get(&name)
                    .map(|(canonical, _)| canonical.to_string())
                    .unwrap_or(name)
            }
        } else {
            name
        };
        let bad = || usage(format!("Zona waktu tidak valid: {name}"));
        if name.starts_with(['+', '-']) {
            let digits = name[1..].replace(':', "");
            if digits.bytes().all(|b| b.is_ascii_digit())
                && ((name.len() == 3 && digits.len() == 2)
                    || (name.len() == 5 && digits.len() == 4)
                    || (name.len() == 6 && digits.len() == 4 && name.as_bytes()[3] == b':'))
            {
                let h = digits[..2].parse::<i32>().map_err(|_| bad())?;
                let m = if digits.len() == 4 {
                    digits[2..].parse::<i32>().map_err(|_| bad())?
                } else {
                    0
                };
                if h < 24 && m < 60 {
                    return Ok(Self {
                        kind: ZoneKind::Fixed((h * 3600 + m * 60) * if name.starts_with('-') { -1 } else { 1 }),
                        name,
                        missing_name: false,
                        invalid_default: false,
                        native: None,
                    });
                }
            }
            return Err(bad());
        }
        if let Some((standard, daylight)) = [
            ("AST4", 4, false),
            ("AST4ADT", 4, true),
            ("CST6", 6, false),
            ("CST6CDT", 6, true),
            ("EST5", 5, false),
            ("EST5EDT", 5, true),
            ("HST10", 10, false),
            ("MST7", 7, false),
            ("MST7MDT", 7, true),
            ("PST8", 8, false),
            ("PST8PDT", 8, true),
            ("YST9", 9, false),
            ("YST9YDT", 9, true),
        ]
        .iter()
        .find(|(suffix, _, _)| {
            name.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("SystemV/"))
                && name.get(8..).is_some_and(|s| s.eq_ignore_ascii_case(suffix))
        })
        .map(|(_, h, d)| (-h * 3600, *d))
        {
            return Ok(Self {
                name,
                kind: ZoneKind::SystemV { standard, daylight },
                missing_name: false,
                invalid_default: false,
                native: None,
            });
        }
        let alias = match name.to_ascii_lowercase().as_str() {
            "us/pacific-new" => "America/Los_Angeles",
            "canada/east-saskatchewan" => "America/Regina",
            _ => &name,
        };
        if alias.eq_ignore_ascii_case("Factory") || alias.eq_ignore_ascii_case("Etc/Unknown") {
            return Err(bad());
        }
        // chrono-tz supplies IANA name metadata, but its older transition data
        // cannot reproduce the legacy ICU database. Pin the actual offset source.
        let canonical = chrono_tz::TZ_VARIANTS
            .iter()
            .find(|t| t.name().eq_ignore_ascii_case(alias))
            .map(|t| t.name())
            .unwrap_or(alias);
        let (canonical, data) = jiff_tzdb::get(canonical).ok_or_else(bad)?;
        let zone = jiff::tz::TimeZone::tzif(canonical, data).map_err(|_| bad())?;
        Ok(Self {
            name,
            kind: ZoneKind::Named(zone),
            missing_name: false,
            invalid_default: false,
            native: None,
        })
    }
    pub(crate) fn utc() -> Self {
        Self {
            name: "UTC".into(),
            kind: ZoneKind::Named(jiff::tz::TimeZone::UTC),
            missing_name: false,
            invalid_default: false,
            native: None,
        }
    }
    pub(crate) fn local() -> Result<Self> {
        let mut zone = Self::resolve(None)?;
        // Cron uses Date's native local fields, not an Intl zone-name formatter.
        zone.invalid_default = false;
        if let Some(kind) = zone.native.take() {
            zone.kind = kind;
        }
        Ok(zone)
    }
    pub(crate) fn offset(&self, ms: i64) -> i64 {
        let mut wall = Wall::from_ms(ms);
        match &self.kind {
            ZoneKind::Fixed(seconds) => i64::from(*seconds) * 1000,
            ZoneKind::SystemV { standard, daylight } => {
                // ICU retains the old SystemV zones: standard time before
                // 1902, then last-Sunday April/October transitions at 02:00.
                let standard = i64::from(*standard) * 1000;
                if !daylight || wall.year < 1902 {
                    return standard;
                }
                let last_sunday = |month, day| day - (days_from_civil(wall.year, month, day) + 4).rem_euclid(7);
                let transition = |month, day| {
                    Wall {
                        year: wall.year,
                        month,
                        day,
                        hour: 2,
                        minute: 0,
                        second: 0,
                        millis: 0,
                    }
                    .to_ms()
                };
                let start = transition(4, last_sunday(4, 30)) - standard;
                let end = transition(10, last_sunday(10, 31)) - standard - 3_600_000;
                standard + if ms >= start && ms < end { 3_600_000 } else { 0 }
            }
            ZoneKind::Named(zone) => {
                // Before all recorded transitions, the first offset applies.
                // After all finite rules, Gregorian POSIX rules repeat every
                // 400 years. This covers Date's range beyond Jiff/chrono.
                if wall.year < -9998 {
                    wall.year = 0;
                }
                if wall.year > 9998 {
                    wall.year = 2400 + wall.year.rem_euclid(400);
                }
                let timestamp = match jiff::Timestamp::from_millisecond(wall.to_ms()) {
                    Ok(t) => t,
                    Err(_) => unreachable!("projected Gregorian date is within Jiff's range"),
                };
                i64::from(zone.to_offset(timestamp).seconds()) * 1000
            }
        }
    }
    pub(crate) fn local_ms(&self, ms: i64) -> i64 {
        ms + self.offset(ms)
    }
    pub(crate) fn resolve_wall(&self, wall_ms: i64) -> i64 {
        let before = self.offset((wall_ms - DAY).clamp(-MAX_MS, MAX_MS));
        let after = self.offset((wall_ms + DAY).clamp(-MAX_MS, MAX_MS));
        [before, after]
            .into_iter()
            .map(|o| wall_ms - o)
            .filter(|u| self.offset((*u).clamp(-MAX_MS, MAX_MS)) == wall_ms - *u)
            .min()
            .unwrap_or(wall_ms - before)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Wall {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub millis: i64,
}

pub(crate) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
pub(crate) fn days_in_month(y: i64, m: i64) -> i64 {
    days_from_civil(if m == 12 { y + 1 } else { y }, if m == 12 { 1 } else { m + 1 }, 1) - days_from_civil(y, m, 1)
}
impl Wall {
    pub(crate) fn to_ms(self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * DAY
            + ((self.hour * 60 + self.minute) * 60 + self.second) * 1000
            + self.millis
    }
    pub(crate) fn from_ms(ms: i64) -> Self {
        let z = ms.div_euclid(DAY) + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let mut year = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = mp + if mp < 10 { 3 } else { -9 };
        year += i64::from(month <= 2);
        let seconds = ms.rem_euclid(DAY) / 1000;
        Self {
            year,
            month,
            day,
            hour: seconds / 3600,
            minute: seconds / 60 % 60,
            second: seconds % 60,
            millis: ms.rem_euclid(1000),
        }
    }
}
pub(crate) fn format_year(y: i64) -> String {
    if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        format!("{}{abs:06}", if y < 0 { "-" } else { "+" }, abs = y.abs())
    }
}
fn format_wall(w: Wall) -> String {
    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
        format_year(w.year),
        w.month,
        w.day,
        w.hour,
        w.minute,
        w.second,
        w.millis
    )
}
pub(crate) fn utc_iso(ms: i64) -> String {
    format!("{}Z", format_wall(Wall::from_ms(ms)))
}
pub(crate) fn format_in_zone(ms: i64, zone: &Zone) -> String {
    let offset = zone.offset(ms) / 1000;
    let a = offset.abs();
    format!(
        "{}{}{:02}:{:02}{}",
        format_wall(Wall::from_ms(ms + offset * 1000)),
        if offset < 0 { "-" } else { "+" },
        a / 3600,
        a / 60 % 60,
        if a % 60 != 0 {
            format!(":{:02}", a % 60)
        } else {
            String::new()
        }
    )
}
pub(crate) fn check_range(ms: f64, what: &str) -> Result<i64> {
    if !ms.is_finite() || ms.abs() > MAX_MS as f64 {
        return Err(usage(format!(
            "{what} di luar rentang yang didukung (±8,64e15 ms): {}",
            js_number(ms)
        )));
    }
    Ok(ms as i64)
}

pub(crate) fn parse_instant(text: &str, zone: &Zone, unit: Option<&str>) -> Result<i64> {
    let text = text.trim_matches(whitespace);
    let digits = text.strip_prefix('-').unwrap_or(text);
    let epoch = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
    if !epoch && unit.is_some() {
        return Err(usage(format!("--unit hanya berlaku untuk epoch: {text}")));
    }
    if epoch {
        let n = text.parse::<f64>().unwrap_or(f64::INFINITY);
        let seconds = unit.map(|u| u == "s").unwrap_or(n.abs() < 1e11);
        return check_range(if seconds { n * 1000.0 } else { n }, "Epoch");
    }
    let re = Regex::new(r"(?i-u)^([+-]\d{6}|\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,9}))?)?(Z|[+-]\d{2}(?::?\d{2})?)?)?$").map_err(|e| usage(e.to_string()))?;
    let m = re.captures(text).ok_or_else(|| {
        usage(format!(
            "Format waktu tidak dikenali: {text}. Pakai epoch atau ISO-8601."
        ))
    })?;
    if &m[1] == "-000000" {
        return Err(usage(format!("Tahun tidak valid: {text}")));
    }
    let number = |i| m.get(i).and_then(|v| v.as_str().parse::<i64>().ok()).unwrap_or(0);
    let frac = m.get(7).map(|v| v.as_str()).unwrap_or("");
    let millis = format!("{frac:0<3}");
    let w = Wall {
        year: number(1),
        month: number(2),
        day: number(3),
        hour: number(4),
        minute: number(5),
        second: number(6),
        millis: millis[..3].parse().unwrap_or(0),
    };
    if !(1..=12).contains(&w.month)
        || w.day < 1
        || w.day > days_in_month(w.year, w.month)
        || w.hour > 23
        || w.minute > 59
        || w.second > 59
    {
        return Err(usage(format!("Tanggal atau jam tidak valid: {text}")));
    }
    let wall_ms = w.to_ms();
    let ms = if let Some(tz) = m.get(8) {
        let tz = tz.as_str();
        if tz.eq_ignore_ascii_case("Z") {
            wall_ms
        } else {
            let digits = tz[1..].replace(':', "");
            let h = digits[..2].parse::<i64>().unwrap_or(0);
            let min = digits
                .get(2..)
                .filter(|s| !s.is_empty())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            if h > 23 || min > 59 {
                return Err(usage(format!("Offset zona tidak valid: {text}")));
            }
            wall_ms - if tz.starts_with('-') { -1 } else { 1 } * (h * 60 + min) * 60_000
        }
    } else {
        zone.resolve_wall(wall_ms)
    };
    check_range(ms as f64, "Waktu")
}
pub(crate) fn relative(ms: i64, now: i64) -> String {
    let diff = ms - now;
    let abs = diff.abs();
    if abs < 1000 {
        return "sekarang".into();
    }
    let s = abs / 1000;
    let m = s / 60;
    let h = m / 60;
    let d = h / 24;
    let (n, unit) = if s < 60 {
        (s, "detik")
    } else if m < 60 {
        (m, "menit")
    } else if h < 24 {
        (h, "jam")
    } else if d < 30 {
        (d, "hari")
    } else if d < 365 {
        (d / 30, "bulan")
    } else {
        (d / 365, "tahun")
    };
    format!("{n} {unit} {}", if diff < 0 { "lalu" } else { "lagi" })
}
