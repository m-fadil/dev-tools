use crate::errors::{CliError, Result, usage};
use regex::Regex;
use std::cmp::Ordering;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Vec<String>,
    pub build: Vec<String>,
}

pub const RELEASES: [&str; 7] = [
    "major",
    "minor",
    "patch",
    "premajor",
    "preminor",
    "prepatch",
    "prerelease",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    Major,
    Minor,
    Patch,
    Premajor,
    Preminor,
    Prepatch,
    Prerelease,
}

impl Release {
    pub fn from_name(name: &str) -> Option<Release> {
        Some(match name {
            "major" => Release::Major,
            "minor" => Release::Minor,
            "patch" => Release::Patch,
            "premajor" => Release::Premajor,
            "preminor" => Release::Preminor,
            "prepatch" => Release::Prepatch,
            "prerelease" => Release::Prerelease,
            _ => return None,
        })
    }
}

const NUM: &str = "0|[1-9][0-9]*";
const PRE_ID: &str = "(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)";
const BUILD_ID: &str = "[0-9A-Za-z-]+";

/// Largest integer JS can represent exactly (Number.MAX_SAFE_INTEGER).
const MAX_SAFE: u64 = 9_007_199_254_740_991;

pub static PRE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("^{PRE_ID}(?:\\.{PRE_ID})*$")).expect("static regex"));
static SEMVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "^({NUM})\\.({NUM})\\.({NUM})(?:-({PRE_ID}(?:\\.{PRE_ID})*))?(?:\\+({BUILD_ID}(?:\\.{BUILD_ID})*))?$"
    ))
    .expect("static regex")
});

pub fn is_numeric(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

/// Mirrors `Number.isSafeInteger(Number(text))` for a digit string.
pub fn to_number(text: &str, what: &str) -> std::result::Result<u64, String> {
    let mut n: u64 = 0;
    for b in text.bytes() {
        n = n.saturating_mul(10).saturating_add(u64::from(b - b'0'));
        if n > MAX_SAFE {
            return Err(format!("Angka {what} terlalu besar: {text}"));
        }
    }
    Ok(n)
}

fn split_ids(s: Option<regex::Match<'_>>) -> Vec<String> {
    s.map(|m| m.as_str().split('.').map(str::to_string).collect())
        .unwrap_or_default()
}

/// Strict SemVer 2.0.0 parse; one leading `v` is stripped. Returns an error message instead of failing.
pub fn try_parse(text: &str) -> std::result::Result<Version, String> {
    let body = text.strip_prefix('v').unwrap_or(text);
    let Some(m) = SEMVER_RE.captures(body) else {
        return Err(format!("bukan SemVer 2.0.0 yang valid: {text}"));
    };
    Ok(Version {
        major: to_number(&m[1], "major")?,
        minor: to_number(&m[2], "minor")?,
        patch: to_number(&m[3], "patch")?,
        prerelease: split_ids(m.get(4)),
        build: split_ids(m.get(5)),
    })
}

pub fn parse_version(text: &str) -> Result<Version> {
    try_parse(text).map_err(|e| usage(format!("Versi tidak valid, {e}")))
}

pub fn format(v: &Version, with_build: bool) -> String {
    let mut s = format!("{}.{}.{}", v.major, v.minor, v.patch);
    if !v.prerelease.is_empty() {
        s.push('-');
        s.push_str(&v.prerelease.join("."));
    }
    if with_build && !v.build.is_empty() {
        s.push('+');
        s.push_str(&v.build.join("."));
    }
    s
}

fn compare_id(a: &str, b: &str) -> Ordering {
    match (is_numeric(a), is_numeric(b)) {
        // Numeric identifiers have no leading zeros, so length then lexical order equals numeric order (any size).
        (true, true) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.cmp(b),
    }
}

pub fn compare_prerelease(a: &[String], b: &[String]) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    for (x, y) in a.iter().zip(b) {
        let c = compare_id(x, y);
        if c != Ordering::Equal {
            return c;
        }
    }
    a.len().cmp(&b.len())
}

/// SemVer 2.0.0 section 11 precedence; build metadata is ignored.
pub fn compare(a: &Version, b: &Version) -> Ordering {
    a.major
        .cmp(&b.major)
        .then(a.minor.cmp(&b.minor))
        .then(a.patch.cmp(&b.patch))
        .then_with(|| compare_prerelease(&a.prerelease, &b.prerelease))
}

pub fn ordering_sign(o: Ordering) -> i32 {
    match o {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub fn check_preid(preid: &str) -> Result<()> {
    if !PRE_RE.is_match(preid) || preid.contains('.') || is_numeric(preid) {
        return Err(usage(format!(
            "--preid harus satu identifier alfanumerik (mengandung huruf atau -): {preid}"
        )));
    }
    Ok(())
}

/// Adds one to a decimal digit string (arbitrary size, like BigInt + 1n).
fn increment_decimal(s: &str) -> String {
    let mut digits: Vec<u8> = s.bytes().collect();
    let mut i = digits.len();
    loop {
        if i == 0 {
            digits.insert(0, b'1');
            break;
        }
        i -= 1;
        if digits[i] == b'9' {
            digits[i] = b'0';
        } else {
            digits[i] += 1;
            break;
        }
    }
    // Digits are ASCII by construction.
    String::from_utf8_lossy(&digits).into_owned()
}

/// Increments the prerelease list like npm: bump the last numeric identifier, else append 0; --preid resets the tag.
fn increment_pre(pre: &[String], preid: Option<&str>) -> Vec<String> {
    let mut next: Vec<String> = pre.to_vec();
    if next.is_empty() {
        next.push("0".into());
    } else {
        match next.iter().rposition(|id| is_numeric(id)) {
            Some(i) => next[i] = increment_decimal(&next[i]),
            None => next.push("0".into()),
        }
    }
    if let Some(preid) = preid {
        // Same tag keeps its counter when it has one; a different tag restarts at 0.
        let keep = next[0] == preid && next.len() > 1 && is_numeric(&next[1]);
        if !keep {
            next = vec![preid.to_string(), "0".into()];
        }
    }
    next
}

/// npm `semver.inc` semantics; the result never carries build metadata.
pub fn bump(v: &Version, release: Release, preid: Option<&str>) -> Result<Version> {
    let (mut major, mut minor, mut patch) = (v.major, v.minor, v.patch);
    let was_pre = !v.prerelease.is_empty();
    let pre: Vec<String>;
    match release {
        Release::Major => {
            if minor != 0 || patch != 0 || !was_pre {
                major += 1;
            }
            minor = 0;
            patch = 0;
            pre = vec![];
        }
        Release::Minor => {
            if patch != 0 || !was_pre {
                minor += 1;
            }
            patch = 0;
            pre = vec![];
        }
        Release::Patch => {
            if !was_pre {
                patch += 1;
            }
            pre = vec![];
        }
        Release::Premajor => {
            major += 1;
            minor = 0;
            patch = 0;
            pre = increment_pre(&[], preid);
        }
        Release::Preminor => {
            minor += 1;
            patch = 0;
            pre = increment_pre(&[], preid);
        }
        Release::Prepatch => {
            patch += 1;
            pre = increment_pre(&[], preid);
        }
        Release::Prerelease => {
            if !was_pre {
                patch += 1;
            }
            pre = increment_pre(&v.prerelease, preid);
        }
    }
    if [major, minor, patch].iter().any(|&n| n > MAX_SAFE) {
        return Err(CliError::Failed("Hasil bump melebihi bilangan bulat aman.".into()));
    }
    Ok(Version {
        major,
        minor,
        patch,
        prerelease: pre,
        build: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_increment() {
        assert_eq!(increment_decimal("9007199254740993"), "9007199254740994");
        assert_eq!(increment_decimal("999"), "1000");
        assert_eq!(increment_decimal("0"), "1");
    }

    #[test]
    fn safe_integer_limit() {
        assert!(to_number("9007199254740991", "major").is_ok());
        assert!(to_number("9007199254740992", "major").is_err());
        assert!(to_number("99999999999999999999999999999", "major").is_err());
    }
}
