use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

use super::normalize;

static WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{L}\p{M}\p{N}]+(?:['’-][\p{L}\p{M}\p{N}]+)*").expect("Pola kata statis valid"));
static NON_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\p{L}\p{M}\p{N}]+").expect("Pola pemisah statis valid"));
static LETTER_OR_DIGIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}]$").expect("Pola huruf dan angka statis valid"));

/// JavaScript `\s` / `String.prototype.trim` whitespace set.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

pub fn is_letter_or_number(c: char) -> bool {
    let mut buf = [0u8; 4];
    LETTER_OR_DIGIT.is_match(c.encode_utf8(&mut buf))
}

pub fn grapheme_count(text: &str) -> usize {
    text.graphemes(true).count()
}

/// Lines of the input; one trailing newline is not a line of its own.
pub fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts = body(text).split('\n').peekable();
    let mut lines = Vec::new();
    while let Some(line) = parts.next() {
        // Only CR immediately before an LF belongs to its terminator; a final lone CR is content.
        lines.push(
            if parts.peek().is_some() {
                line.strip_suffix('\r').unwrap_or(line)
            } else {
                line
            }
            .to_string(),
        );
    }
    lines
}

/// Input without its single trailing newline.
pub fn body(text: &str) -> &str {
    match text.strip_suffix('\n') {
        Some(t) => t.strip_suffix('\r').unwrap_or(t),
        None => text,
    }
}

pub fn words(text: &str) -> Vec<&str> {
    WORD.find_iter(text).map(|m| m.as_str()).collect()
}

pub struct LongestLine {
    pub number: usize,
    pub length: usize,
}

pub struct Stats {
    pub characters: usize,
    pub graphemes: usize,
    pub bytes: usize,
    pub words: usize,
    pub lines: usize,
    pub sentences: usize,
    pub longest_line: LongestLine,
    pub average_word_length: f64,
}

fn trailing_newline_bytes(buf: &[u8]) -> usize {
    if buf.last() == Some(&b'\n') {
        if buf.len() > 1 && buf[buf.len() - 2] == b'\r' {
            2
        } else {
            1
        }
    } else {
        0
    }
}

/// Single pass: a maximal run of . ! ? … followed by whitespace/end closes a segment; segments with a letter/digit count.
fn count_sentences(text: &str) -> usize {
    let is_end = |c: char| matches!(c, '.' | '!' | '?' | '…');
    let mut chars = text.chars().peekable();
    let mut count = 0;
    let mut has_content = false;
    while let Some(c) = chars.next() {
        if is_end(c) {
            while chars.peek().is_some_and(|&next| is_end(next)) {
                chars.next();
            }
            if chars.peek().is_none_or(|&next| is_js_space(next)) {
                if has_content {
                    count += 1;
                }
                has_content = false;
            }
        } else if !has_content && is_letter_or_number(c) {
            has_content = true;
        }
    }
    count + usize::from(has_content)
}

pub fn stats(raw: &str, input: &[u8]) -> Stats {
    let text = body(raw);
    let lines = split_lines(raw);
    let w = words(text);
    let mut longest = LongestLine { number: 0, length: 0 };
    for (i, line) in lines.iter().enumerate() {
        let length = grapheme_count(line);
        if length > longest.length {
            longest = LongestLine { number: i + 1, length };
        }
    }
    let letters: usize = w.iter().map(|word| grapheme_count(word)).sum();
    Stats {
        characters: text.chars().count(),
        graphemes: grapheme_count(text),
        bytes: input.len() - trailing_newline_bytes(input),
        words: w.len(),
        lines: lines.len(),
        sentences: count_sentences(text),
        longest_line: longest,
        average_word_length: if w.is_empty() {
            0.0
        } else {
            (letters as f64 / w.len() as f64 * 100.0).round() / 100.0
        },
    }
}

pub fn reverse_text(raw: &str, by_lines: bool) -> Vec<String> {
    let mut lines = split_lines(raw);
    if by_lines {
        lines.reverse();
        lines
    } else {
        lines
            .iter()
            .map(|l| l.graphemes(true).rev().collect::<String>())
            .collect()
    }
}

/// Strip U+0300–U+036F diacritics, recompose, and lowercase; preserve other scripts' combining marks.
pub fn slugify(text: &str, sep: &str) -> String {
    let stripped: String = text.chars().filter(|&c| c != '\'' && c != '’').collect();
    let kept: Vec<char> = normalize::nfd(&stripped)
        .into_iter()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .collect();
    let clean = normalize::compose(&kept).to_lowercase();
    NON_WORD
        .split(&clean)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}

pub struct FreqOptions {
    pub top: usize,
    pub case_sensitive: bool,
    pub min_length: usize,
}

pub struct Frequency {
    pub total: usize,
    pub unique: usize,
    pub top: Vec<(String, usize)>,
}

/// JS string `<` compares UTF-16 code units.
fn cmp_utf16(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

pub fn frequency(text: &str, o: &FreqOptions) -> Frequency {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total = 0;
    for raw in words(text) {
        let word = if o.case_sensitive {
            raw.to_string()
        } else {
            raw.to_lowercase()
        };
        if word.chars().count() < o.min_length {
            continue;
        }
        total += 1;
        *counts.entry(word).or_insert(0) += 1;
    }
    let unique = counts.len();
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| cmp_utf16(&a.0, &b.0)));
    sorted.truncate(o.top);
    Frequency {
        total,
        unique,
        top: sorted,
    }
}

pub struct LineOptions {
    pub trim: bool,
    pub unique: bool,
    pub sort: bool,
    pub reverse: bool,
}

/// Order of operations: trim, unique (first occurrence wins), sort (code point order), reverse.
pub fn process_lines(raw: &str, o: &LineOptions) -> Vec<String> {
    let mut lines = split_lines(raw);
    if o.trim {
        lines = lines.iter().map(|l| js_trim(l).to_string()).collect();
    }
    if o.unique {
        let mut seen = HashSet::new();
        lines.retain(|l| seen.insert(l.clone()));
    }
    if o.sort {
        lines.sort();
    }
    if o.reverse {
        lines.reverse();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_point_order_matches_legacy_sort() {
        let input = [
            "\u{1f600}",
            "\u{ffff}",
            "\u{ff21}",
            "z",
            "\u{10000}",
            "\u{10ffff}",
            "ab",
            "a",
            "a\u{1f600}",
            "a\u{ffff}",
        ]
        .join("\n");
        let out = process_lines(
            &format!("{input}\n"),
            &LineOptions {
                trim: false,
                unique: false,
                sort: true,
                reverse: false,
            },
        );
        assert_eq!(
            out,
            [
                "a",
                "ab",
                "a\u{ffff}",
                "a\u{1f600}",
                "z",
                "\u{ff21}",
                "\u{ffff}",
                "\u{10000}",
                "\u{1f600}",
                "\u{10ffff}"
            ]
        );
    }

    #[test]
    fn utf16_order_for_ties() {
        // U+10000 is units D800 DC00, which is below the single unit FFFF.
        assert_eq!(cmp_utf16("\u{10000}", "\u{ffff}"), Ordering::Less);
    }

    #[test]
    fn js_whitespace_set() {
        assert!(is_js_space('\u{feff}'));
        assert!(!is_js_space('\u{85}'));
        assert_eq!(js_trim("\u{feff} x \u{a0}"), "x");
    }

    #[test]
    fn crlf_and_lone_cr_preserve_legacy_line_boundaries() {
        assert_eq!(split_lines("x\r"), ["x\r"]);
        assert_eq!(split_lines("x\r\r\n"), ["x\r"]);
        assert_eq!(split_lines("x\r\n\r"), ["x", "\r"]);
        assert_eq!(split_lines("x\r\n\n"), ["x", ""]);
        assert_eq!(split_lines("\n"), [""]);
        assert!(split_lines("").is_empty());
        assert_eq!(body("x\r\n\n"), "x\r\n");
    }

    #[test]
    fn slug_handles_marks_and_scripts() {
        assert_eq!(slugify("Héllo Wörld", "-"), "hello-world");
        assert_eq!(slugify("ガパ", "-"), "ガパ");
        assert_eq!(slugify("カ\u{3099} ハ\u{309a}", "-"), "ガ-パ");
        assert_eq!(slugify("हिन्दी भाषा", "-"), "हिन्दी-भाषा");
        assert_eq!(slugify("a\u{1ab0}", "-"), "a\u{1ab0}");
    }
}
