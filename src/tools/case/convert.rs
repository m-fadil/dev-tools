use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Camel,
    Pascal,
    Snake,
    Screaming,
    Kebab,
    Train,
    Dot,
    Path,
    Title,
    Sentence,
    Lower,
    Upper,
}

pub const STYLES: [(&str, Style); 12] = [
    ("camel", Style::Camel),
    ("pascal", Style::Pascal),
    ("snake", Style::Snake),
    ("screaming", Style::Screaming),
    ("kebab", Style::Kebab),
    ("train", Style::Train),
    ("dot", Style::Dot),
    ("path", Style::Path),
    ("title", Style::Title),
    ("sentence", Style::Sentence),
    ("lower", Style::Lower),
    ("upper", Style::Upper),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Lower,
    Upper,
    Digit,
}

static NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{N}$").expect("valid regex"));
static UPPER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{Lu}\p{Lt}]$").expect("valid regex"));
static MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{M}$").expect("valid regex"));
static SEPARATORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\p{L}\p{M}\p{N}]+").expect("valid regex"));

fn is(re: &Regex, ch: char) -> bool {
    let mut buf = [0u8; 4];
    re.is_match(ch.encode_utf8(&mut buf))
}

fn kind_of(ch: char) -> Kind {
    if is(&NUMBER, ch) {
        Kind::Digit
    } else if is(&UPPER, ch) {
        Kind::Upper
    } else {
        Kind::Lower
    }
}

/// Splits identifiers and prose into words. Separators are anything that is not a letter, mark or
/// number (apostrophes are dropped instead); boundaries are lower->Upper, letter<->digit and
/// the end of an acronym (HTTPServer -> HTTP, Server). Case is preserved.
pub fn split_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let cleaned = text.replace(['\'', '’'], "");
    for chunk in SEPARATORS.split(&cleaned) {
        if chunk.is_empty() {
            continue;
        }
        let chars: Vec<char> = chunk.chars().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            if is(&MARK, chars[i]) {
                continue; // combining marks stick to their base
            }
            let prev = kind_of(chars[i - 1]);
            let cur = kind_of(chars[i]);
            let next = chars.get(i + 1).map(|&c| kind_of(c));
            let boundary = (prev == Kind::Digit) != (cur == Kind::Digit)
                || (prev == Kind::Lower && cur == Kind::Upper)
                || (prev == Kind::Upper && cur == Kind::Upper && next == Some(Kind::Lower));
            if boundary {
                words.push(chars[start..i].iter().collect());
                start = i;
            }
        }
        words.push(chars[start..].iter().collect());
    }
    words
}

fn capitalize(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut it = lower.chars();
    match it.next() {
        Some(first) => first.to_uppercase().chain(it).collect(),
        None => String::new(),
    }
}

pub fn convert(text: &str, style: Style) -> String {
    let words = split_words(text);
    let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    let caps = || words.iter().map(|w| capitalize(w));
    match style {
        Style::Camel => words
            .iter()
            .enumerate()
            .map(|(i, w)| if i == 0 { w.to_lowercase() } else { capitalize(w) })
            .collect(),
        Style::Pascal => caps().collect(),
        Style::Snake => lower.join("_"),
        Style::Screaming => lower.join("_").to_uppercase(),
        Style::Kebab => lower.join("-"),
        Style::Train => caps().collect::<Vec<_>>().join("-"),
        Style::Dot => lower.join("."),
        Style::Path => lower.join("/"),
        Style::Title => caps().collect::<Vec<_>>().join(" "),
        Style::Sentence => words
            .iter()
            .enumerate()
            .map(|(i, w)| if i == 0 { capitalize(w) } else { w.to_lowercase() })
            .collect::<Vec<_>>()
            .join(" "),
        Style::Lower => lower.join(" "),
        Style::Upper => lower.join(" ").to_uppercase(),
    }
}

/// Lines of the input; one trailing newline is not a line of its own.
pub fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let body = match text.strip_suffix('\n') {
        Some(t) => t.strip_suffix('\r').unwrap_or(t),
        None => text,
    };
    let pieces: Vec<&str> = body.split('\n').collect();
    let last = pieces.len() - 1;
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i < last {
                p.strip_suffix('\r').unwrap_or(p).to_string()
            } else {
                p.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines() {
        assert_eq!(split_lines(""), Vec::<String>::new());
        assert_eq!(split_lines("a\r\n\r\nb\n"), vec!["a", "", "b"]);
        assert_eq!(split_lines("a\n\n"), vec!["a", ""]);
    }

    #[test]
    fn words() {
        assert_eq!(split_words("parseURL2Json"), vec!["parse", "URL", "2", "Json"]);
        assert_eq!(convert("straße größe", Style::Pascal), "StraßeGröße");
    }
}
