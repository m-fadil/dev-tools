use regex::{Regex, RegexBuilder};

pub struct MatchJob<'a> {
    pub text: &'a str,
    pub first: bool,
    pub replace: Option<&'a str>,
}

pub struct MatchRecord {
    /// Byte offset of the match.
    pub index: usize,
    pub text: String,
    pub groups: Vec<Option<String>>,
    pub named: Vec<(String, Option<String>)>,
}

pub struct MatchResult {
    pub matches: Vec<MatchRecord>,
    pub replaced: Option<String>,
}

const FLAG_CHARS: &str = "imsx";

/// Validates flag letters and returns them sorted and deduplicated.
pub fn normalize_flags(flags: &str) -> Result<String, String> {
    let mut letters: Vec<char> = Vec::new();
    for flag in flags.chars() {
        if !FLAG_CHARS.contains(flag) {
            return Err(format!("Flag tidak dikenal: {flag}. Pilihan: {FLAG_CHARS}"));
        }
        letters.push(flag);
    }
    letters.sort_unstable();
    letters.dedup();
    Ok(letters.into_iter().collect())
}

pub fn build(pattern: &str, flags: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .ignore_whitespace(flags.contains('x'))
        .build()
}

fn to_owned(m: Option<regex::Match<'_>>) -> Option<String> {
    m.map(|m| m.as_str().to_string())
}

pub fn run_match(re: &Regex, job: &MatchJob) -> MatchResult {
    let names: Vec<(usize, &str)> = re
        .capture_names()
        .enumerate()
        .filter_map(|(i, n)| n.map(|n| (i, n)))
        .collect();
    let mut matches = Vec::new();
    for caps in re.captures_iter(job.text) {
        let Some(whole) = caps.get(0) else { continue };
        matches.push(MatchRecord {
            index: whole.start(),
            text: whole.as_str().to_string(),
            groups: (1..caps.len()).map(|i| to_owned(caps.get(i))).collect(),
            named: names
                .iter()
                .map(|(i, n)| ((*n).to_string(), to_owned(caps.get(*i))))
                .collect(),
        });
        if job.first {
            break;
        }
    }
    let replaced = job.replace.map(|template| {
        let limit = if job.first { 1 } else { 0 };
        re.replacen(job.text, limit, template).into_owned()
    });
    MatchResult { matches, replaced }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job<'a>(_pattern: &'a str, text: &'a str) -> MatchJob<'a> {
        MatchJob {
            text,
            first: false,
            replace: None,
        }
    }

    #[test]
    fn flags_are_normalized() {
        assert_eq!(normalize_flags("six").as_deref(), Ok("isx"));
        assert_eq!(normalize_flags("ii").as_deref(), Ok("i"));
        assert_eq!(
            normalize_flags("g"),
            Err("Flag tidak dikenal: g. Pilihan: imsx".to_string())
        );
    }

    #[test]
    fn zero_length_matches_advance() {
        let re = build("x*", "").unwrap();
        let r = run_match(&re, &job("x*", "ab"));
        assert_eq!(r.matches.iter().map(|m| m.index).collect::<Vec<_>>(), vec![0, 1, 2]);
        let re = build("", "").unwrap();
        let r = run_match(&re, &job("", "😀a"));
        assert_eq!(r.matches.iter().map(|m| m.index).collect::<Vec<_>>(), vec![0, 4, 5]);
    }

    #[test]
    fn replace_respects_first() {
        let re = build(r"\d", "").unwrap();
        let mut j = job(r"\d", "1 2 3");
        j.replace = Some("[$0]");
        assert_eq!(run_match(&re, &j).replaced.as_deref(), Some("[1] [2] [3]"));
        j.first = true;
        assert_eq!(run_match(&re, &j).replaced.as_deref(), Some("[1] 2 3"));
    }
}
