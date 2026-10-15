mod data;
mod generate;
pub mod rng;

use crate::errors::{Result, usage};
use crate::io::{choice, integer, parse, print_json};
use crate::tool::Tool;
use clap::Parser;
use generate::{LOCALES, Locale, Params, TYPES, Value, days_from_civil, make};
use std::ffi::OsString;
use std::io::Write;

pub static TOOL: Tool = Tool {
    name: "fake",
    aliases: &[],
    description: "Buat data palsu (nama, email, alamat, IP, tanggal, dll.) untuk pengujian",
    category: "Utilitas",
    run,
};

const MAX_SPAN: i64 = 4_294_967_295;
const YEAR_MS: i64 = 31_557_600_000; // 365.25 days
const DAY_MS: i64 = 86_400_000;

/// Legacy parseArgs config; help text depends on the type so it is generated, not static.
#[derive(Parser)]
#[command(name = "dt fake", disable_help_flag = true, disable_version_flag = true)]
struct Args {
    #[arg(short = 'c', long, overrides_with = "count")]
    count: Option<String>,
    #[arg(long, overrides_with = "locale")]
    locale: Option<String>,
    #[arg(long, overrides_with = "seed")]
    seed: Option<String>,
    #[arg(long, overrides_with = "domain")]
    domain: Option<String>,
    #[arg(long, overrides_with = "from")]
    from: Option<String>,
    #[arg(long, overrides_with = "to")]
    to: Option<String>,
    #[arg(long, overrides_with = "min")]
    min: Option<String>,
    #[arg(long, overrides_with = "max")]
    max: Option<String>,
    #[arg(long, overrides_with = "length")]
    length: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    positionals: Vec<String>,
}

struct SpecialHelp {
    types: &'static [&'static str],
    options: &'static [&'static str],
}

const SPECIAL_HELP: &[SpecialHelp] = &[
    SpecialHelp {
        types: &["email"],
        options: &["--domain <domain>  default: example.com"],
    },
    SpecialHelp {
        types: &["date", "datetime"],
        options: &[
            "--from <tanggal>   batas awal (ISO 8601)",
            "--to <tanggal>     batas akhir (ISO 8601)",
        ],
    },
    SpecialHelp {
        types: &["number"],
        options: &["--min <n>          default: 0", "--max <n>          default: 1000"],
    },
    SpecialHelp {
        types: &["hex"],
        options: &["--length <n>       default: 16"],
    },
];

fn help_text(kind: Option<&str>) -> String {
    let groups: Vec<&SpecialHelp> = SPECIAL_HELP
        .iter()
        .filter(|g| kind.is_none_or(|k| g.types.contains(&k)))
        .collect();
    let special = if groups.is_empty() {
        String::new()
    } else {
        let body: Vec<String> = groups
            .iter()
            .map(|g| {
                let title = kind.map_or_else(|| g.types.join(", "), str::to_string);
                let opts: Vec<String> = g.options.iter().map(|l| format!("    {l}")).collect();
                format!("  {title}\n{}", opts.join("\n"))
            })
            .collect();
        format!("\nOpsi khusus:\n{}\n", body.join("\n"))
    };
    let phone = if kind.is_none() || kind == Some("phone") {
        "\nTelepon id hanya mirip format nomor asli; jangan dipakai untuk menghubungi siapa pun.\n"
    } else {
        ""
    };
    let example = if kind == Some("email") {
        "dt fake email --domain perusahaan.test -c 5".to_string()
    } else {
        format!("dt fake {} -c 5 --seed 7", kind.unwrap_or("name"))
    };
    let types_line = if kind.is_none() {
        format!("\nJenis: {}\n", TYPES.join(", "))
    } else {
        String::new()
    };
    format!(
        "dt fake {} [options]\n{types_line}\nOpsi umum:\n  -c, --count <n>   jumlah hasil, 1-1000 (default: 1)\n      --locale <l>  id atau en (default: en)\n      --seed <n>    hasil bisa diulang\n      --json        keluaran JSON\n  -h, --help        bantuan\n{special}{phone}\nContoh: {example}\n",
        kind.unwrap_or("<jenis>")
    )
}

/// JS `\s` (WhiteSpace + LineTerminator).
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// Apply the same UTS #46 mapping used by Node's domainToASCII.
fn domain_to_ascii(value: &str) -> Option<String> {
    let ascii = idna::domain_to_ascii(value).ok()?;
    ipv4_host(&ascii)
}

fn ipv4_number(part: &str) -> Option<u64> {
    if part.is_empty() {
        return None;
    }
    let (digits, radix) = if let Some(h) = part.strip_prefix("0x").or_else(|| part.strip_prefix("0X")) {
        (h, 16)
    } else if part.len() > 1 && part.starts_with('0') {
        (&part[1..], 8)
    } else {
        (part, 10)
    };
    if digits.is_empty() {
        return Some(0);
    }
    u64::from_str_radix(digits, radix)
        .ok()
        .or_else(|| digits.chars().all(|c| c.is_digit(radix)).then_some(u64::MAX))
}

/// WHATWG host parser step: a host ending in a number is an IPv4 address or invalid.
fn ipv4_host(host: &str) -> Option<String> {
    let mut parts: Vec<&str> = host.split('.').collect();
    if parts.last() == Some(&"") && parts.len() > 1 {
        parts.pop();
    }
    let last = parts.last().copied().unwrap_or("");
    let numeric = !last.is_empty()
        && (last.bytes().all(|b| b.is_ascii_digit())
            || last
                .strip_prefix("0x")
                .or_else(|| last.strip_prefix("0X"))
                .is_some_and(|h| h.bytes().all(|b| b.is_ascii_hexdigit())));
    if !numeric {
        return Some(host.to_string());
    }
    if parts.len() > 4 {
        return None;
    }
    let mut nums = Vec::new();
    for p in &parts {
        nums.push(ipv4_number(p)?);
    }
    let (&tail, head) = nums.split_last()?;
    if head.iter().any(|&n| n > 255) || tail >= 256u64.pow(5 - nums.len() as u32) {
        return None;
    }
    let mut addr = tail;
    for (i, &n) in head.iter().enumerate() {
        addr += n * 256u64.pow(3 - i as u32);
    }
    Some(format!(
        "{}.{}.{}.{}",
        addr >> 24,
        (addr >> 16) & 255,
        (addr >> 8) & 255,
        addr & 255
    ))
}

/// Normalize a DNS email domain without accepting URL/userinfo syntax or querying DNS.
fn parse_domain(value: &str) -> Result<String> {
    if value.chars().any(|c| is_js_space(c) || "@:/\\?#%[]".contains(c)) {
        return Err(usage("--domain harus domain saja, tanpa @, URL, port, atau path."));
    }
    let invalid = || usage(format!("Domain email tidak valid: {value}"));
    let domain = domain_to_ascii(value).unwrap_or_default().to_lowercase();
    let labels: Vec<&str> = domain.split('.').collect();
    let label_ok = |l: &str| {
        let b = l.as_bytes();
        !b.is_empty()
            && l.len() <= 63
            && b.iter()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
            && b[0] != b'-'
            && b[b.len() - 1] != b'-'
    };
    if domain.is_empty() || domain.len() > 253 || labels.len() < 2 || !labels.iter().all(|l| label_ok(l)) {
        return Err(invalid());
    }
    Ok(domain)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

fn digits(s: &[u8]) -> Option<i64> {
    if s.iter().all(u8::is_ascii_digit) {
        std::str::from_utf8(s).ok()?.parse().ok()
    } else {
        None
    }
}

/// Strict ISO 8601 date or date-time without fractional seconds; date-only and zone-less values are UTC.
fn parse_date(value: &str, option: &str) -> Result<i64> {
    parse_date_inner(value).ok_or_else(|| usage(format!("Nilai --{option} bukan tanggal ISO 8601 yang valid: {value}")))
}

fn parse_date_inner(value: &str) -> Option<i64> {
    let b = value.as_bytes();
    if !value.is_ascii() || b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (digits(&b[0..4])?, digits(&b[5..7])?, digits(&b[8..10])?);
    let (mut h, mut mi, mut s, mut offset) = (0, 0, 0, 0);
    let rest = &b[10..];
    if !rest.is_empty() {
        if (rest[0] != b'T' && rest[0] != b' ') || rest.len() < 6 || rest[3] != b':' {
            return None;
        }
        h = digits(&rest[1..3])?;
        mi = digits(&rest[4..6])?;
        let mut tail = &rest[6..];
        if tail.first() == Some(&b':') {
            if tail.len() < 3 {
                return None;
            }
            s = digits(&tail[1..3])?;
            tail = &tail[3..];
        }
        match tail {
            [] | [b'Z'] => {}
            [sign @ (b'+' | b'-'), a, b2, b':', c, d2] => {
                let (oh, om) = (digits(&[*a, *b2])?, digits(&[*c, *d2])?);
                if oh > 23 || om > 59 {
                    return None;
                }
                offset = (oh * 60 + om) * 60_000 * if *sign == b'+' { 1 } else { -1 };
            }
            _ => return None,
        }
    }
    // Date.parse accepts any day 1-31 and rolls it over; the legacy round trip (Date.UTC maps years 0-99 to
    // 1900-1999) then verifies the calendar day survives.
    let adjusted = if y < 100 { y + 1900 } else { y };
    let valid = (1..=12).contains(&mo)
        && (1..=31).contains(&d)
        && mi <= 59
        && s <= 59
        && h < 24
        && d <= days_in_month(adjusted, mo);
    if !valid {
        return None;
    }
    Some(days_from_civil(y, mo, d) * DAY_MS + h * 3_600_000 + mi * 60_000 + s * 1000 - offset)
}

fn signed_int(value: Option<&str>, fallback: i64, option: &str) -> Result<i64> {
    let Some(v) = value else { return Ok(fallback) };
    let (neg, digits_part) = match v.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, v),
    };
    let ok = !digits_part.is_empty() && digits_part.bytes().all(|c| c.is_ascii_digit());
    let trimmed = digits_part.trim_start_matches('0');
    let n = if !ok {
        None
    } else if trimmed.is_empty() {
        Some(0)
    } else {
        trimmed.parse::<u64>().ok()
    };
    match n {
        Some(n) if n <= 9_007_199_254_740_991 => Ok(if neg { -(n as i64) } else { n as i64 }),
        _ => Err(usage(format!("Nilai --{option} harus bilangan bulat: {v}"))),
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("fake", args)?;
    if a.positionals.len() > 1 {
        return Err(usage(format!("Argumen tidak dikenal: {}", a.positionals[1])));
    }
    let kind = a.positionals.first().map(String::as_str);
    if let Some(k) = kind
        && !TYPES.contains(&k)
    {
        return Err(usage(format!(
            "Jenis tidak dikenal: {k}. Pilihan: {}",
            TYPES.join(", ")
        )));
    }
    let only = |names: &[(&str, bool)], types: &[&str]| -> Result<()> {
        for (n, present) in names {
            if *present && kind.is_none_or(|k| !types.contains(&k)) {
                return Err(usage(format!("--{n} hanya berlaku untuk jenis: {}", types.join(", "))));
            }
        }
        Ok(())
    };
    only(&[("domain", a.domain.is_some())], &["email"])?;
    only(
        &[("from", a.from.is_some()), ("to", a.to.is_some())],
        &["date", "datetime"],
    )?;
    only(&[("min", a.min.is_some()), ("max", a.max.is_some())], &["number"])?;
    only(&[("length", a.length.is_some())], &["hex"])?;
    let mut stdout = std::io::stdout().lock();
    if a.help {
        let _ = write!(stdout, "{}", help_text(kind));
        return Ok(());
    }
    let Some(kind) = kind else {
        let list: Vec<String> = TYPES.iter().map(|t| format!("  {t}")).collect();
        let _ = writeln!(stdout, "Jenis yang tersedia:\n{}", list.join("\n"));
        return Ok(());
    };

    let count = integer(a.count.as_deref(), 1, 1, 1000, "count")? as usize;
    let locale = choice(a.locale.as_deref(), LOCALES, Locale::En, "locale")?;
    let now = now_ms();
    let from = match a.from.as_deref() {
        None => now - 10 * YEAR_MS,
        Some(v) => parse_date(v, "from")?,
    };
    let to = match a.to.as_deref() {
        None => now,
        Some(v) => parse_date(v, "to")?,
    };
    if from > to {
        return Err(usage("--from tidak boleh setelah --to."));
    }
    let min = signed_int(a.min.as_deref(), 0, "min")?;
    let max = signed_int(a.max.as_deref(), min.max(1000), "max")?;
    if min > max {
        return Err(usage("--min tidak boleh lebih besar dari --max."));
    }
    // the rng is 32-bit: wider spans would silently lose precision
    if max - min > MAX_SPAN {
        return Err(usage(format!("Selisih --max dan --min maksimal {MAX_SPAN} (2^32-1).")));
    }
    let rng = rng::from_seed(a.seed.as_deref())?;
    let email_domain = match a.domain.as_deref() {
        None => "example.com".to_string(),
        Some(d) => parse_domain(d)?,
    };
    let length = integer(a.length.as_deref(), 16, 1, 1024, "length")? as usize;
    let mut params = Params {
        rng,
        locale,
        email_domain,
        from,
        to,
        min,
        max,
        length,
    };
    let out: Vec<Value> = (0..count).map(|_| make(kind, &mut params)).collect();
    if a.json {
        drop(stdout);
        print_json(&serde_json::json!({
            "type": kind,
            "locale": locale.name(),
            "values": out.iter().map(Value::json).collect::<Vec<_>>(),
        }));
    } else {
        let lines: Vec<String> = out.iter().map(Value::text).collect();
        let _ = writeln!(stdout, "{}", lines.join("\n"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_follow_legacy_round_trip() {
        assert_eq!(parse_date_inner("2020-01-01"), Some(1_577_836_800_000));
        assert_eq!(parse_date_inner("2020-01-01T00:00:10Z"), Some(1_577_836_810_000));
        assert_eq!(parse_date_inner("2020-02-30"), None);
        assert_eq!(parse_date_inner("2020-01-01T00:00:00.125Z"), None);
        assert_eq!(
            parse_date_inner("2020-01-01 10:00+02:00"),
            Some(1_577_836_800_000 + 8 * 3_600_000)
        );
        assert_eq!(parse_date_inner("0000-02-29"), None);
    }

    #[test]
    fn signed_ints() {
        assert_eq!(signed_int(Some("-0"), 5, "min").ok(), Some(0));
        assert_eq!(signed_int(Some("007"), 5, "min").ok(), Some(7));
        assert!(signed_int(Some("9007199254740992"), 5, "min").is_err());
        assert!(signed_int(Some("1.5"), 5, "min").is_err());
    }

    #[test]
    fn domains() {
        assert_eq!(
            parse_domain("Mail.Perusahaan.TEST").ok().as_deref(),
            Some("mail.perusahaan.test")
        );
        assert_eq!(parse_domain("BÜCHER.test").ok().as_deref(), Some("xn--bcher-kva.test"));
        assert!(parse_domain("localhost").is_err());
        assert!(parse_domain("a_b.test").is_err());
    }
}
