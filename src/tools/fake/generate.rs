use super::data::{self, COMPANY_WORDS, LOREM_WORDS, LocaleData, TLDS};
use super::rng::Rng;

pub const TYPES: &[&str] = &[
    "name",
    "firstname",
    "lastname",
    "email",
    "username",
    "phone",
    "company",
    "address",
    "city",
    "country",
    "ip",
    "ipv6",
    "mac",
    "uuid",
    "date",
    "datetime",
    "number",
    "bool",
    "color",
    "url",
    "word",
    "sentence",
    "hex",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Id,
    En,
}

pub const LOCALES: &[(&str, Locale)] = &[("id", Locale::Id), ("en", Locale::En)];

impl Locale {
    pub fn name(self) -> &'static str {
        match self {
            Locale::Id => "id",
            Locale::En => "en",
        }
    }

    fn data(self) -> &'static LocaleData {
        match self {
            Locale::Id => &data::ID,
            Locale::En => &data::EN,
        }
    }

    fn countries(self) -> &'static [&'static str] {
        match self {
            Locale::Id => data::ID_COUNTRIES,
            Locale::En => data::EN_COUNTRIES,
        }
    }
}

pub enum Value {
    Str(String),
    Num(i64),
    Bool(bool),
}

impl Value {
    pub fn text(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Num(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
        }
    }

    pub fn json(&self) -> serde_json::Value {
        match self {
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::Num(n) => serde_json::Value::from(*n),
            Value::Bool(b) => serde_json::Value::Bool(*b),
        }
    }
}

pub struct Params {
    pub rng: Rng,
    pub locale: Locale,
    pub email_domain: String,
    /// epoch ms bounds for date/datetime
    pub from: i64,
    pub to: i64,
    pub min: i64,
    pub max: i64,
    pub length: usize,
}

const DAY: i64 = 86_400_000;

fn hex(rng: &mut Rng, digits: usize) -> String {
    (0..digits)
        .map(|_| char::from_digit(rng.rand_int(0, 15) as u32, 16).unwrap_or('0'))
        .collect()
}

fn full_name(p: &mut Params) -> (&'static str, &'static str) {
    let d = p.locale.data();
    let first = *p.rng.pick(d.first_names);
    let last = *p.rng.pick(d.last_names);
    (first, last)
}

/// lowercase, then drop everything outside a-z0-9
fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

fn uuid(rng: &mut Rng) -> String {
    let mut h: Vec<char> = hex(rng, 32).chars().collect();
    h[12] = '4';
    h[16] = ['8', '9', 'a', 'b'][rng.rand_int(0, 3) as usize];
    let s: String = h.into_iter().collect();
    format!("{}-{}-{}-{}-{}", &s[..8], &s[8..12], &s[12..16], &s[16..20], &s[20..])
}

fn company(p: &mut Params) -> String {
    let d = p.locale.data();
    let word = *p.rng.pick(COMPANY_WORDS);
    let last = *p.rng.pick(d.last_names);
    let suffix = *p.rng.pick(d.company_suffixes);
    if suffix == "PT" || suffix == "CV" {
        return format!("{suffix} {word} {last}");
    }
    if p.locale == Locale::Id {
        format!("PT {word} {last} {suffix}")
    } else {
        format!("{last} {word} {suffix}")
    }
}

// ---- lorem (private port of legacy lorem/text.ts used by url/word/sentence) ----

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn random_words(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in 0..n {
        let mut word = *rng.pick(LOREM_WORDS);
        while i > 0 && out.get(i - 1).map(String::as_str) == Some(word) {
            word = rng.pick(LOREM_WORDS);
        }
        out.push(word.to_string());
    }
    out
}

fn lorem_word(rng: &mut Rng) -> String {
    random_words(rng, 1).join(" ")
}

fn lorem_sentence(rng: &mut Rng) -> String {
    let length = rng.rand_int(5, 15) as usize;
    let mut words = random_words(rng, length);
    let mut i = 0;
    while i + 1 < words.len() {
        if i >= 2 && !words[i].ends_with(',') && rng.next() < 0.12 {
            words[i].push(',');
        }
        if words[i].ends_with(',') && i + 1 < words.len() {
            i += 1;
        }
        i += 1;
    }
    format!("{}.", capitalize(&words.join(" ")))
}

// ---- dates ----

/// Days since 1970-01-01 of a proleptic Gregorian civil date.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `Date.prototype.toISOString` for an epoch-millisecond value.
fn iso_string(ms: i64) -> String {
    let days = ms.div_euclid(DAY);
    let rem = ms.rem_euclid(DAY);
    let (y, m, d) = civil_from_days(days);
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        format!("{}{:06}", if y < 0 { '-' } else { '+' }, y.abs())
    };
    format!(
        "{year}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000
    )
}

fn slice(s: &str, end: usize) -> String {
    s.chars().take(end).collect()
}

fn ceil_div(a: i64, b: i64) -> i64 {
    -((-a).div_euclid(b))
}

pub fn make(kind: &str, p: &mut Params) -> Value {
    match kind {
        "name" => {
            let (f, l) = full_name(p);
            Value::Str(format!("{f} {l}"))
        }
        "firstname" => Value::Str((*p.rng.pick(p.locale.data().first_names)).to_string()),
        "lastname" => Value::Str((*p.rng.pick(p.locale.data().last_names)).to_string()),
        "email" => {
            let (f, l) = full_name(p);
            let n = if p.rng.next() < 0.4 {
                p.rng.rand_int(1, 99).to_string()
            } else {
                String::new()
            };
            Value::Str(format!("{}.{}{}@{}", slug(f), slug(l), n, p.email_domain))
        }
        "username" => {
            let (f, l) = full_name(p);
            let style = p.rng.rand_int(0, 2);
            let n = p.rng.rand_int(1, 999);
            let (fs, ls) = (slug(f), slug(l));
            Value::Str(match style {
                0 => format!("{fs}{n}"),
                1 => format!("{fs}_{ls}"),
                _ => format!("{}{ls}{n}", fs.chars().next().map(String::from).unwrap_or_default()),
            })
        }
        "phone" => {
            // id: +62 8xx mobile-style format only; en: NANP fictional range 555-0100..0199
            if p.locale == Locale::Id {
                let a = p.rng.rand_int(11, 99);
                let b = p.rng.rand_int(1000, 9999);
                let c = p.rng.rand_int(1000, 9999);
                Value::Str(format!("+62 8{a}-{b}-{c}"))
            } else {
                let a = p.rng.rand_int(201, 989);
                let b = p.rng.rand_int(0, 99);
                Value::Str(format!("+1 {a}-555-01{b:02}"))
            }
        }
        "company" => Value::Str(company(p)),
        "address" => {
            let d = p.locale.data();
            let street = *p.rng.pick(d.streets);
            let city = *p.rng.pick(d.cities);
            let no = p.rng.rand_int(1, 999);
            Value::Str(if p.locale == Locale::Id {
                format!("Jl. {street} No. {no}, {city}")
            } else {
                format!("{no} {street} Street, {city}")
            })
        }
        "city" => Value::Str((*p.rng.pick(p.locale.data().cities)).to_string()),
        "country" => Value::Str((*p.rng.pick(p.locale.countries())).to_string()),
        "ip" => {
            let net = *p.rng.pick(&["192.0.2", "198.51.100", "203.0.113"]);
            let n = p.rng.rand_int(1, 254);
            Value::Str(format!("{net}.{n}"))
        }
        "ipv6" => {
            let parts: Vec<String> = (0..6).map(|_| format!("{:x}", p.rng.rand_int(0, 0xffff))).collect();
            Value::Str(format!("2001:db8:{}", parts.join(":")))
        }
        "mac" => {
            // locally administered, unicast
            let first = format!("{:02x}", p.rng.rand_int(0, 63) * 4 + 2);
            let mut parts = vec![first];
            for _ in 0..5 {
                parts.push(hex(&mut p.rng, 2));
            }
            Value::Str(parts.join(":"))
        }
        "uuid" => Value::Str(uuid(&mut p.rng)),
        "date" => {
            let day = p.rng.rand_int(p.from.div_euclid(DAY), p.to.div_euclid(DAY));
            Value::Str(slice(&iso_string(day * DAY), 10))
        }
        "datetime" => {
            let sec = p.rng.rand_int(ceil_div(p.from, 1000), p.to.div_euclid(1000));
            Value::Str(format!("{}Z", slice(&iso_string(sec * 1000), 19)))
        }
        "number" => Value::Num(p.rng.rand_int(p.min, p.max)),
        "bool" => Value::Bool(p.rng.next() < 0.5),
        "color" => Value::Str(format!("#{}", hex(&mut p.rng, 6))),
        "url" => {
            let tld = *p.rng.pick(TLDS);
            let word = lorem_word(&mut p.rng);
            let n = p.rng.rand_int(1, 9999);
            Value::Str(format!("https://www.example.{tld}/{word}/{n}"))
        }
        "word" => Value::Str(lorem_word(&mut p.rng)),
        "sentence" => Value::Str(lorem_sentence(&mut p.rng)),
        _ => Value::Str(hex(&mut p.rng, p.length)),
    }
}
