use std::sync::LazyLock;

use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

const WIDE: [(u32, u32); 12] = [
    (0x1100, 0x115f),
    (0x2e80, 0x303e),
    (0x3041, 0x33ff),
    (0x3400, 0x4dbf),
    (0x4e00, 0x9fff),
    (0xa000, 0xa4cf),
    (0xac00, 0xd7a3),
    (0xf900, 0xfaff),
    (0xfe30, 0xfe6f),
    (0xff00, 0xff60),
    (0xffe0, 0xffe6),
    (0x20000, 0x3fffd),
];

fn code_point_width(cp: u32) -> usize {
    if cp < 0x20 || (0x7f..0xa0).contains(&cp) {
        return 0;
    }
    if (0x300..=0x36f).contains(&cp)
        || (0x200b..=0x200f).contains(&cp)
        || (0x20d0..=0x20ff).contains(&cp)
        || (0xfe00..=0xfe0f).contains(&cp)
    {
        return 0;
    }
    if WIDE.iter().any(|&(lo, hi)| cp >= lo && cp <= hi) {
        2
    } else {
        1
    }
}

static DEFAULT_EMOJI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{Emoji_Presentation}").expect("Pola presentasi emoji statis valid"));
static EMOJI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{Extended_Pictographic}\p{Emoji}]").expect("Pola emoji statis valid"));

fn grapheme_width(grapheme: &str) -> usize {
    // VS15 requests text presentation; VS16 only widens an emoji-capable cluster.
    if !grapheme.is_ascii() && !grapheme.contains('\u{fe0e}') {
        let default_emoji = DEFAULT_EMOJI.is_match(grapheme);
        let vs16_emoji = grapheme.contains('\u{fe0f}') && EMOJI.is_match(grapheme);
        if default_emoji || vs16_emoji {
            return 2;
        }
    }
    grapheme.chars().map(|c| code_point_width(c as u32)).max().unwrap_or(0)
}

/// Terminal columns taken by `text`, counting grapheme clusters (CJK and emoji are 2 wide).
pub fn display_width(text: &str) -> usize {
    text.graphemes(true).map(grapheme_width).sum()
}

/// Cuts `text` to at most `max` columns, ending with an ellipsis when something was removed.
pub fn truncate(text: &str, max: usize) -> String {
    if display_width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for segment in text.graphemes(true) {
        let w = grapheme_width(segment);
        if width + w > max.saturating_sub(1) {
            break;
        }
        out.push_str(segment);
        width += w;
    }
    out.push('…');
    out
}

/// Makes a cell safe for one-line display: line breaks become ↵, tab a space, other C0/C1 controls U+FFFD.
pub fn sanitize_cell(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut chars = cell.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('↵');
            }
            '\n' => out.push('↵'),
            '\t' => out.push(' '),
            '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' => out.push('\u{fffd}'),
            _ => out.push(c),
        }
    }
    out
}

pub fn render_table(header: &[String], rows: &[Vec<String>], max_width: usize) -> String {
    let cell = |c: &String| truncate(&sanitize_cell(c), max_width);
    let head: Vec<String> = header.iter().map(cell).collect();
    let body: Vec<Vec<String>> = rows.iter().map(|r| r.iter().map(cell).collect()).collect();
    let mut widths = vec![1usize; header.len()];
    for r in std::iter::once(&head).chain(body.iter()) {
        for (i, w) in widths.iter_mut().enumerate() {
            *w = (*w).max(r.get(i).map_or(0, |c| display_width(c)));
        }
    }
    let last = widths.len().saturating_sub(1);
    let line = |r: &Vec<String>| -> String {
        r.iter()
            .enumerate()
            .map(|(i, c)| {
                if i == last {
                    c.clone()
                } else {
                    format!("{c}{}", " ".repeat(widths[i].saturating_sub(display_width(c))))
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let separator = widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("-+-");
    let mut out = vec![line(&head), separator];
    out.extend(body.iter().map(line));
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_presentation_and_flags_are_two_wide() {
        for cell in ["🇮🇩", "⌚", "⌛", "✅", "☀️", "♟️", "🫠", "👨‍👩‍👧‍👦", "1️⃣"]
        {
            assert_eq!(display_width(cell), 2, "{cell}");
            assert_eq!(truncate(&format!("{cell}ab"), 3), format!("{cell}…"), "{cell}");
        }
        for cell in ["☀", "♟", "⌚︎", "x️", "e\u{301}"] {
            assert_eq!(display_width(cell), 1, "{cell}");
        }
        assert_eq!(display_width("日本"), 4);
    }

    #[test]
    fn sanitizes_every_control() {
        for cp in 0u32..=0x9f {
            if cp > 0x1f && cp < 0x7f {
                continue;
            }
            let c = char::from_u32(cp).unwrap();
            let expected = match cp {
                9 => " ",
                10 | 13 => "↵",
                _ => "\u{fffd}",
            };
            assert_eq!(sanitize_cell(&c.to_string()), expected, "{cp}");
            assert_eq!(display_width(expected), 1);
        }
        assert_eq!(sanitize_cell("a\r\nb\tc"), "a↵b c");
    }

    #[test]
    fn render_table_has_no_raw_controls() {
        let out = render_table(&["\x1b[2J\x07".to_string()], &[vec!["x".to_string()]], 100);
        assert!(
            !out.chars()
                .any(|c| (c as u32) < 0x20 && c != '\n' || (0x7f..0xa0).contains(&(c as u32)))
        );
    }
}
