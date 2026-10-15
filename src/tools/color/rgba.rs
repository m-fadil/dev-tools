use super::names::{NAMES, TRANSPARENT};
use crate::errors::{CliError, Result, usage};
use crate::numfmt::js_number;
use regex::Regex;
use std::sync::LazyLock;

/// sRGB color: integer channels 0-255, alpha 0-1 rounded to 3 decimals at parse time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

/// ECMAScript `Math.round` (round half toward +Infinity).
fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// ECMAScript WhiteSpace + LineTerminator, as used by `String.prototype.trim` and `\s`.
pub fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

fn clamp(n: f64, min: f64, max: f64) -> Result<f64> {
    if !n.is_finite() {
        return Err(usage(format!(
            "Komponen warna harus berupa angka hingga: {}",
            js_number(n)
        )));
    }
    Ok(n.max(min).min(max))
}

fn byte(n: f64) -> Result<u8> {
    Ok(clamp(js_round(n), 0.0, 255.0)? as u8)
}

// ---- parsing ----

const NUM: &str = r"[+-]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)";
static NUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("^({NUM})(%?)$")).expect("valid regex"));
static HUE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("^({NUM})([A-Za-z]*)$")).expect("valid regex"));

fn bad(kind: &str, token: &str) -> CliError {
    usage(format!("{kind} tidak valid: {token}"))
}

fn shorten(token: &str) -> String {
    if token.chars().count() > 40 {
        format!("{}...", token.chars().take(40).collect::<String>())
    } else {
        token.to_string()
    }
}

struct Component {
    value: f64,
    percent: bool,
}

/// Number or percentage; `percent_scale` is the value a percentage of 100% maps to.
fn component(token: &str, percent_scale: f64, kind: &str) -> Result<Component> {
    let m = NUM_RE.captures(token).ok_or_else(|| bad(kind, token))?;
    let n: f64 = m[1].parse().map_err(|_| bad(kind, token))?;
    if !n.is_finite() {
        return Err(bad(kind, &shorten(token)));
    }
    let percent = !m[2].is_empty();
    let value = if percent { (n / 100.0) * percent_scale } else { n };
    if !value.is_finite() {
        return Err(bad(kind, &shorten(token)));
    }
    Ok(Component { value, percent })
}

fn percent_only(token: &str, kind: &str) -> Result<f64> {
    let c = component(token, 1.0, kind)?;
    if !c.percent {
        return Err(usage(format!("{kind} harus berupa persen (%): {token}")));
    }
    clamp(c.value, 0.0, 1.0)
}

fn alpha(token: Option<&str>) -> Result<f64> {
    let Some(token) = token else { return Ok(1.0) };
    // Canonical alpha is shared by formatting, hex byte conversion, and name matching.
    Ok(js_round(clamp(component(token, 1.0, "Alpha")?.value, 0.0, 1.0)? * 1000.0) / 1000.0)
}

fn hue(token: &str) -> Result<f64> {
    let m = HUE_RE.captures(token).ok_or_else(|| bad("Hue", token))?;
    let unit = m[2].to_ascii_lowercase();
    if !matches!(unit.as_str(), "" | "deg" | "grad" | "rad" | "turn") {
        return Err(bad("Hue", token));
    }
    let n: f64 = m[1].parse().map_err(|_| bad("Hue", token))?;
    if !n.is_finite() {
        return Err(bad("Hue", &shorten(token)));
    }
    let degrees = match unit.as_str() {
        "turn" => n * 360.0,
        "rad" => (n * 180.0) / std::f64::consts::PI,
        "grad" => n * 0.9,
        _ => n,
    };
    if !degrees.is_finite() {
        return Err(bad("Hue", &shorten(token)));
    }
    Ok(((degrees % 360.0) + 360.0) % 360.0)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    let f = |n: f64| {
        let k = (n + h / 30.0) % 12.0;
        l - s * l.min(1.0 - l) * (-1.0f64).max((k - 3.0).min(9.0 - k).min(1.0))
    };
    [f(0.0) * 255.0, f(8.0) * 255.0, f(4.0) * 255.0]
}

fn hwb_to_rgb(h: f64, w: f64, bl: f64) -> [f64; 3] {
    if w + bl >= 1.0 {
        let gray = (w / (w + bl)) * 255.0;
        return [gray, gray, gray];
    }
    hsl_to_rgb(h, 1.0, 0.5).map(|c| (c / 255.0) * (1.0 - w - bl) * 255.0 + w * 255.0)
}

fn parse_hex(digits: &str, has_hash: bool) -> Result<Rgba> {
    let len = digits.encode_utf16().count();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) || ![3, 4, 6, 8].contains(&len) {
        return Err(bad(
            "Warna hex",
            &format!("{}{digits}", if has_hash { "#" } else { "" }),
        ));
    }
    if !has_hash && len != 6 && len != 8 {
        return Err(usage(format!("Hex tanpa '#' harus 6 atau 8 digit: {digits}")));
    }
    let full: String = if len <= 4 {
        digits.chars().flat_map(|c| [c, c]).collect()
    } else {
        digits.to_string()
    };
    let at = |i: usize| -> u8 {
        if full.len() > i {
            u8::from_str_radix(&full[i..i + 2], 16).unwrap_or(255)
        } else {
            255
        }
    };
    Ok(Rgba {
        r: at(0),
        g: at(2),
        b: at(4),
        a: js_round((at(6) as f64 / 255.0) * 1000.0) / 1000.0,
    })
}

fn parse_function(func: &str, body: &str, original: &str) -> Result<Rgba> {
    let lower = func.to_ascii_lowercase();
    let family = lower.strip_suffix('a').unwrap_or(&lower);
    let parts: Vec<String>;
    let mut legacy = false;
    let mut alpha_token: Option<String> = None;
    if body.contains(',') {
        legacy = true;
        if family == "hwb" {
            return Err(usage(format!("hwb() tidak mendukung koma; pakai spasi: {original}")));
        }
        if body.contains('/') {
            return Err(usage(format!("Sintaks campuran koma dan '/': {original}")));
        }
        let list: Vec<String> = body.split(',').map(|s| js_trim(s).to_string()).collect();
        if list.len() != 3 && list.len() != 4 {
            return Err(usage(format!("{func}() butuh 3 atau 4 nilai: {original}")));
        }
        parts = list[..3].to_vec();
        alpha_token = list.get(3).cloned();
    } else {
        let slash: Vec<&str> = body.split('/').collect();
        if slash.len() > 2 {
            return Err(usage(format!("Terlalu banyak '/': {original}")));
        }
        let first = js_trim(slash[0]);
        parts = if first.is_empty() {
            vec![String::new()]
        } else {
            first
                .split(is_js_ws)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        };
        if slash.len() == 2 {
            let t = js_trim(slash[1]);
            if t.is_empty() || t.chars().any(is_js_ws) {
                return Err(bad("Alpha", if t.is_empty() { "(kosong)" } else { t }));
            }
            alpha_token = Some(t.to_string());
        }
        if parts.len() != 3 {
            return Err(usage(format!("{func}() butuh 3 nilai warna: {original}")));
        }
    }
    if parts.iter().any(|p| p.is_empty()) {
        return Err(usage(format!("Nilai kosong: {original}")));
    }
    let a = alpha(alpha_token.as_deref())?;
    if family == "rgb" {
        let cs = parts
            .iter()
            .map(|p| component(p, 255.0, "Kanal RGB"))
            .collect::<Result<Vec<_>>>()?;
        if legacy && cs.iter().any(|c| c.percent != cs[0].percent) {
            return Err(usage(format!(
                "Sintaks koma tidak boleh mencampur angka dan persen: {original}"
            )));
        }
        return Ok(Rgba {
            r: byte(cs[0].value)?,
            g: byte(cs[1].value)?,
            b: byte(cs[2].value)?,
            a,
        });
    }
    let h = hue(&parts[0])?;
    let [r, g, b] = if family == "hsl" {
        let s = percent_only(&parts[1], "Saturasi")?;
        let l = percent_only(&parts[2], "Lightness")?;
        hsl_to_rgb(h, s, l)
    } else {
        let w = percent_only(&parts[1], "Whiteness")?;
        let bl = percent_only(&parts[2], "Blackness")?;
        hwb_to_rgb(h, w, bl)
    };
    Ok(Rgba {
        r: byte(r)?,
        g: byte(g)?,
        b: byte(b)?,
        a,
    })
}

fn name_hex(lower: &str) -> Option<&'static str> {
    NAMES.iter().find(|(n, _)| *n == lower).map(|(_, h)| *h)
}

/// `^(rgba?|hsla?|hwb)\(\s*([^()]*?)\s*\)$` (case-insensitive): returns (function name, trimmed body).
fn split_function(s: &str) -> Option<(&str, &str)> {
    let inner = s.strip_suffix(')')?;
    let open = inner.find('(')?;
    let (name, rest) = (&inner[..open], &inner[open + 1..]);
    if !["rgb", "rgba", "hsl", "hsla", "hwb"]
        .iter()
        .any(|f| f.eq_ignore_ascii_case(name))
    {
        return None;
    }
    if rest.contains(['(', ')']) {
        return None;
    }
    Some((name, js_trim(rest)))
}

/// Parses hex (#rgb #rgba #rrggbb #rrggbbaa; '#' optional for 6/8 digits), rgb/hsl/hwb functions, and CSS names.
pub fn parse_color(text: &str) -> Result<Rgba> {
    let s = js_trim(text);
    if s.is_empty() {
        return Err(usage("Warna kosong."));
    }
    let lower = s.to_lowercase();
    if lower == TRANSPARENT {
        return Ok(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0.0,
        });
    }
    if let Some(hex) = name_hex(&lower) {
        return parse_hex(hex, false);
    }
    if let Some(rest) = s.strip_prefix('#') {
        return parse_hex(rest, true);
    }
    if let Some((func, body)) = split_function(s) {
        return parse_function(func, body, s);
    }
    if s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return parse_hex(s, false);
    }
    Err(usage(format!("Warna tidak dikenali: {s}")))
}

// ---- conversion & formatting ----

pub struct Hsl {
    pub h: i64,
    pub s: i64,
    pub l: i64,
}

pub struct Hwb {
    pub h: i64,
    pub w: i64,
    pub b: i64,
}

fn unit(c: Rgba) -> (f64, f64, f64, f64, f64) {
    let (r, g, b) = (c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0);
    (r, g, b, r.max(g).max(b), r.min(g).min(b))
}

pub fn to_hsl(c: Rgba) -> Hsl {
    let (r, g, b, max, min) = unit(c);
    let d = max - min;
    let l = (max + min) / 2.0;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    Hsl {
        h: js_round(hue_of(r, g, b, max, d)) as i64 % 360,
        s: js_round(s * 100.0) as i64,
        l: js_round(l * 100.0) as i64,
    }
}

pub fn to_hwb(c: Rgba) -> Hwb {
    let (r, g, b, max, min) = unit(c);
    Hwb {
        h: js_round(hue_of(r, g, b, max, max - min)) as i64 % 360,
        w: js_round(min * 100.0) as i64,
        b: js_round((1.0 - max) * 100.0) as i64,
    }
}

fn hue_of(r: f64, g: f64, b: f64, max: f64, d: f64) -> f64 {
    if d == 0.0 {
        return 0.0;
    }
    let sector = if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (sector * 60.0 + 360.0) % 360.0
}

pub fn to_hex(c: Rgba) -> String {
    let base = format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
    if c.a < 1.0 {
        format!("{base}{:02x}", js_round(c.a * 255.0) as u8)
    } else {
        base
    }
}

pub fn to_rgb_string(c: Rgba) -> String {
    if c.a < 1.0 {
        format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, js_number(c.a))
    } else {
        format!("rgb({}, {}, {})", c.r, c.g, c.b)
    }
}

pub fn to_hsl_string(c: Rgba) -> String {
    let Hsl { h, s, l } = to_hsl(c);
    if c.a < 1.0 {
        format!("hsla({h}, {s}%, {l}%, {})", js_number(c.a))
    } else {
        format!("hsl({h}, {s}%, {l}%)")
    }
}

pub fn to_hwb_string(c: Rgba) -> String {
    let Hwb { h, w, b } = to_hwb(c);
    let alpha = if c.a < 1.0 {
        format!(" / {}", js_number(c.a))
    } else {
        String::new()
    };
    format!("hwb({h} {w}% {b}%{alpha})")
}

// ---- names ----

// Legacy builds NAME_LIST once; do not reparse every named color for every lookup.
static NAME_LIST: LazyLock<Vec<(&'static str, [i64; 3])>> = LazyLock::new(|| {
    NAMES
        .iter()
        .map(|&(name, hex)| {
            let channel = |i| i64::from(u8::from_str_radix(&hex[i..i + 2], 16).expect("CSS table contains hex"));
            (name, [channel(0), channel(2), channel(4)])
        })
        .collect()
});

/// Exact CSS name (first alphabetically among synonyms) or the nearest by squared RGB distance; alpha is ignored.
/// Returns (name, exact).
pub fn name_of(c: Rgba) -> (&'static str, bool) {
    if c.a == 0.0 && c.r == 0 && c.g == 0 && c.b == 0 {
        return (TRANSPARENT, true);
    }
    let mut best = NAMES[0].0;
    let mut best_dist = i64::MAX;
    for &(name, [r, g, b]) in NAME_LIST.iter() {
        let (dr, dg, db) = (r - i64::from(c.r), g - i64::from(c.g), b - i64::from(c.b));
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best = name;
            best_dist = dist;
        }
    }
    (best, best_dist == 0)
}

// ---- WCAG 2.x ----

fn linear(v: u8) -> f64 {
    let s = v as f64 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance (alpha ignored).
pub fn luminance(c: Rgba) -> f64 {
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

pub struct Contrast {
    pub ratio: f64,
    pub aa_normal: bool,
    pub aaa_normal: bool,
    pub aa_large: bool,
    pub aaa_large: bool,
}

pub fn contrast(a: Rgba, b: Rgba) -> Contrast {
    let (l1, l2) = (luminance(a), luminance(b));
    let ratio = (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05);
    Contrast {
        ratio,
        aa_normal: ratio >= 4.5,
        aaa_normal: ratio >= 7.0,
        aa_large: ratio >= 3.0,
        aaa_large: ratio >= 4.5,
    }
}

/// Ratio truncated (never rounded up) to 2 decimals so a displayed value never overstates a passing grade.
pub fn ratio_value(ratio: f64) -> f64 {
    (ratio * 100.0 + 1e-9).floor() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_css_names_round_trip_in_table_order() {
        assert_eq!(NAMES.len(), 148);
        for &(name, hex) in NAMES {
            let color = parse_color(name).unwrap();
            assert_eq!(to_hex(color), format!("#{hex}"), "{name}");
            let canonical = NAMES.iter().find(|(_, value)| *value == hex).unwrap().0;
            assert_eq!(name_of(color), (canonical, true), "{name}");
        }
    }

    #[test]
    fn alpha_forms_round_to_three_decimals() {
        for (input, expected) in [
            ("rgb(1 2 3 / 99.99%)", 1.0),
            ("rgb(0 0 0 / 0.01%)", 0.0),
            ("hsla(0,0%,0%,0.0001)", 0.0),
            ("hsl(0 0% 0% / 0.9999)", 1.0),
            ("hwb(0 0% 100% / 0.0001)", 0.0),
            ("hwb(0 0% 100% / 0.9999)", 1.0),
            ("rgba(0,0,0,0.12349)", 0.123),
            ("#00000001", 0.004),
            ("#00000080", 0.502),
        ] {
            assert_eq!(parse_color(input).unwrap().a, expected, "{input}");
        }
    }

    #[test]
    fn non_finite_components_rejected() {
        let huge = "9".repeat(400);
        let finite = "9".repeat(308);
        for v in [
            format!("hsl({huge} 50% 50%)"),
            format!("hsl(-{huge}deg 50% 50%)"),
            format!("hsl({huge}grad 50% 50%)"),
            format!("hsl({huge}rad 50% 50%)"),
            format!("hsl({huge}turn 50% 50%)"),
            format!("hsl({finite}turn 50% 50%)"),
            format!("hsl({finite}rad 50% 50%)"),
            format!("hsl(0 {huge}% 50%)"),
            format!("hsl(0 50% {huge}%)"),
            format!("hwb({huge} 0% 0%)"),
            format!("hwb(0 {huge}% 0%)"),
            format!("hwb(0 0% {huge}%)"),
            format!("rgb({huge} 0 0)"),
            format!("rgb(0 {huge} 0)"),
            format!("rgb(0 0 -{huge})"),
            format!("rgb({huge}% 0% 0%)"),
            format!("rgb({finite}% 0% 0%)"),
            format!("rgb(0 0 0 / {huge})"),
            format!("rgb(0 0 0 / {huge}%)"),
            format!("rgba(0,0,0,-{huge})"),
            "rgb(NaN 0 0)".to_string(),
            "rgb(0 0 0 / Infinity)".to_string(),
        ] {
            assert!(
                matches!(parse_color(&v), Err(CliError::Usage(_))),
                "{}",
                &v[..v.len().min(40)]
            );
        }
        assert_eq!(
            parse_color(&format!("rgb({finite} -{finite} 0)")).unwrap(),
            Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 1.0
            }
        );
    }
}
