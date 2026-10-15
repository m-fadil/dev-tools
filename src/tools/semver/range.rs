use super::version::{PRE_RE, Version, compare, to_number};
use crate::errors::{Result, usage};
use regex::Regex;
use std::cmp::Ordering;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

#[derive(Debug, Clone)]
pub struct Comparator {
    op: Op,
    version: Version,
}

/// A comparator set is an AND of comparators; an empty set matches any (non-prerelease) version.
pub type ComparatorSet = Vec<Comparator>;

struct PartialVersion {
    major: Option<u64>,
    minor: Option<u64>,
    patch: Option<u64>,
    prerelease: Vec<String>,
}

const X: &str = "([0-9]+|[xX*])";
static PARTIAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "^v?{X}(?:\\.{X}(?:\\.{X}(?:-([0-9A-Za-z.-]+))?(?:\\+[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?)?)?$"
    ))
    .expect("static regex")
});

fn ver(major: u64, minor: u64, patch: u64, prerelease: Vec<String>) -> Version {
    Version {
        major,
        minor,
        patch,
        prerelease,
        build: vec![],
    }
}

fn is_wild(p: &str) -> bool {
    matches!(p, "x" | "X" | "*")
}

fn parse_partial(text: &str, range: &str) -> Result<PartialVersion> {
    let bad = || usage(format!("Range tidak valid: \"{text}\" dalam \"{range}\""));
    let Some(m) = PARTIAL_RE.captures(text) else {
        return Err(bad());
    };
    let parts: Vec<Option<&str>> = (1..=3)
        .map(|i| m.get(i).map(|g| g.as_str()).filter(|p| !is_wild(p)))
        .collect();
    // After a wildcard nothing more specific may follow (1.x.3 is meaningless).
    for i in 1..3 {
        if parts[i].is_some() && parts[i - 1].is_none() {
            return Err(bad());
        }
    }
    for p in parts.iter().flatten() {
        if p.len() > 1 && p.starts_with('0') {
            return Err(bad());
        }
    }
    let pre = m.get(4).map(|g| g.as_str());
    if let Some(pre) = pre
        && (parts[2].is_none() || !PRE_RE.is_match(pre))
    {
        return Err(bad());
    }
    let names = ["major", "minor", "patch"];
    let mut nums = [None; 3];
    for i in 0..3 {
        if let Some(p) = parts[i] {
            nums[i] = Some(to_number(p, names[i]).map_err(usage)?);
        }
    }
    Ok(PartialVersion {
        major: nums[0],
        minor: nums[1],
        patch: nums[2],
        prerelease: pre
            .map(|p| p.split('.').map(str::to_string).collect())
            .unwrap_or_default(),
    })
}

fn cmp(op: Op, major: u64, minor: u64, patch: u64, prerelease: Vec<String>) -> Comparator {
    Comparator {
        op,
        version: ver(major, minor, patch, prerelease),
    }
}

/// Upper bound that excludes every prerelease of the given tuple.
fn below(major: u64, minor: u64, patch: u64) -> Comparator {
    cmp(Op::Lt, major, minor, patch, vec!["0".into()])
}

fn nothing() -> ComparatorSet {
    vec![below(0, 0, 0)]
}

fn caret(p: PartialVersion) -> ComparatorSet {
    let PartialVersion {
        major,
        minor,
        patch,
        prerelease,
    } = p;
    let Some(major) = major else { return vec![] };
    let Some(minor) = minor else {
        return vec![cmp(Op::Ge, major, 0, 0, vec![]), below(major + 1, 0, 0)];
    };
    let Some(patch) = patch else {
        return vec![
            cmp(Op::Ge, major, minor, 0, vec![]),
            if major > 0 {
                below(major + 1, 0, 0)
            } else {
                below(0, minor + 1, 0)
            },
        ];
    };
    let lower = cmp(Op::Ge, major, minor, patch, prerelease);
    if major > 0 {
        vec![lower, below(major + 1, 0, 0)]
    } else if minor > 0 {
        vec![lower, below(0, minor + 1, 0)]
    } else {
        vec![lower, below(0, 0, patch + 1)]
    }
}

fn tilde(p: PartialVersion) -> ComparatorSet {
    let PartialVersion {
        major,
        minor,
        patch,
        prerelease,
    } = p;
    let Some(major) = major else { return vec![] };
    let Some(minor) = minor else {
        return vec![cmp(Op::Ge, major, 0, 0, vec![]), below(major + 1, 0, 0)];
    };
    vec![
        cmp(Op::Ge, major, minor, patch.unwrap_or(0), prerelease),
        below(major, minor + 1, 0),
    ]
}

fn primitive(op: Op, p: PartialVersion) -> ComparatorSet {
    let PartialVersion {
        major,
        minor,
        patch,
        prerelease,
    } = p;
    let Some(major) = major else {
        return if op == Op::Gt || op == Op::Lt {
            nothing()
        } else {
            vec![]
        };
    };
    if let Some(patch) = patch {
        return vec![cmp(op, major, minor.unwrap_or(0), patch, prerelease)];
    }
    // Partial version: treat the missing parts as a wildcard.
    let lo = ver(major, minor.unwrap_or(0), 0, vec![]);
    let next = match minor {
        None => below(major + 1, 0, 0),
        Some(minor) => below(major, minor + 1, 0),
    };
    match op {
        Op::Eq => vec![
            Comparator {
                op: Op::Ge,
                version: lo,
            },
            next,
        ],
        Op::Gt => vec![Comparator {
            op: Op::Ge,
            version: Version {
                prerelease: vec![],
                ..next.version
            },
        }],
        Op::Ge => vec![Comparator {
            op: Op::Ge,
            version: lo,
        }],
        Op::Lt => vec![Comparator {
            op: Op::Lt,
            version: Version {
                prerelease: vec!["0".into()],
                ..lo
            },
        }],
        Op::Le => vec![next],
    }
}

fn hyphen(from: PartialVersion, to: PartialVersion) -> ComparatorSet {
    let mut set = Vec::new();
    if let Some(major) = from.major {
        set.push(cmp(
            Op::Ge,
            major,
            from.minor.unwrap_or(0),
            from.patch.unwrap_or(0),
            from.prerelease,
        ));
    }
    if let Some(major) = to.major {
        match (to.minor, to.patch) {
            (None, _) => set.push(below(major + 1, 0, 0)),
            (Some(minor), None) => set.push(below(major, minor + 1, 0)),
            (Some(minor), Some(patch)) => set.push(cmp(Op::Le, major, minor, patch, to.prerelease)),
        }
    }
    set
}

/// JavaScript `\s` (WhiteSpace + LineTerminator).
fn is_js_space(c: char) -> bool {
    c != '\u{85}' && (c.is_whitespace() || c == '\u{feff}')
}

const OPERATORS: [&str; 7] = ["^", "~>", "~", "<=", ">=", "<", ">"];

/// Splits a leading operator off a token (`^`, `~>?`, `<=`, `>=`, `<`, `>`, `=`).
fn split_operator(token: &str) -> (&str, &str) {
    for op in OPERATORS.iter().chain(["="].iter()) {
        if let Some(rest) = token.strip_prefix(op) {
            return (op, rest);
        }
    }
    ("", token)
}

fn parse_set(text: &str, range: &str) -> Result<ComparatorSet> {
    let tokens: Vec<&str> = text.split(is_js_space).filter(|t| !t.is_empty()).collect();
    let mut set = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let mut token = tokens[i].to_string();
        if tokens.get(i + 1) == Some(&"-") {
            if i + 2 >= tokens.len() {
                return Err(usage(format!("Range hyphen tidak lengkap: \"{range}\"")));
            }
            set.extend(hyphen(
                parse_partial(&token, range)?,
                parse_partial(tokens[i + 2], range)?,
            ));
            i += 3;
            continue;
        }
        // Allow a detached operator: ">= 1.2.3".
        if split_operator(&token).1.is_empty() && !token.is_empty() {
            if i + 1 >= tokens.len() {
                return Err(usage(format!("Operator tanpa versi dalam range: \"{range}\"")));
            }
            i += 1;
            token.push_str(tokens[i]);
        }
        let (op, rest) = split_operator(&token);
        let partial = parse_partial(rest, range)?;
        match op {
            "^" => set.extend(caret(partial)),
            "~" | "~>" => set.extend(tilde(partial)),
            _ => {
                let op = match op {
                    "<" => Op::Lt,
                    "<=" => Op::Le,
                    ">" => Op::Gt,
                    ">=" => Op::Ge,
                    _ => Op::Eq,
                };
                set.extend(primitive(op, partial));
            }
        }
        i += 1;
    }
    Ok(set)
}

/// Parses an npm-style range into OR-ed comparator sets.
pub fn parse_range(range: &str) -> Result<Vec<ComparatorSet>> {
    range.split("||").map(|part| parse_set(part, range)).collect()
}

fn test(c: &Comparator, v: &Version) -> bool {
    let r = compare(v, &c.version);
    match c.op {
        Op::Lt => r == Ordering::Less,
        Op::Le => r != Ordering::Greater,
        Op::Gt => r == Ordering::Greater,
        Op::Ge => r != Ordering::Less,
        Op::Eq => r == Ordering::Equal,
    }
}

fn set_matches(set: &ComparatorSet, v: &Version) -> bool {
    if !set.iter().all(|c| test(c, v)) {
        return false;
    }
    if v.prerelease.is_empty() {
        return true;
    }
    // A prerelease only matches a set that opts in with a prerelease on the same major.minor.patch.
    set.iter().any(|c| {
        !c.version.prerelease.is_empty()
            && c.version.major == v.major
            && c.version.minor == v.minor
            && c.version.patch == v.patch
    })
}

pub fn satisfies(v: &Version, sets: &[ComparatorSet]) -> bool {
    sets.iter().any(|set| set_matches(set, v))
}
