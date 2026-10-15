mod common;

use common::{dt, dt_in};
use serde_json::{Value, json};

fn text(args: &[&str], input: impl AsRef<[u8]>) -> common::Out {
    let mut all = vec!["text"];
    all.extend_from_slice(args);
    dt_in(&all, input.as_ref())
}

fn doc(args: &[&str], input: impl AsRef<[u8]>) -> Value {
    let mut all = args.to_vec();
    all.push("--json");
    let r = text(&all, input);
    assert_eq!(r.code, 0, "{}", r.err);
    serde_json::from_str(&r.out).unwrap()
}

#[test]
fn stats_unicode_counts_and_empty_input() {
    assert_eq!(
        doc(&["stats"], "Hello, wörld! 👨‍👩‍👧 Ok.\nsecond line here\n"),
        json!({
            "characters": 40, "graphemes": 36, "bytes": 54, "words": 6, "lines": 2, "sentences": 3,
            "longestLine": { "number": 1, "length": 19 }, "averageWordLength": 4.33
        })
    );
    assert_eq!(
        doc(&["stats"], ""),
        json!({
            "characters": 0, "graphemes": 0, "bytes": 0, "words": 0, "lines": 0, "sentences": 0,
            "longestLine": { "number": 0, "length": 0 }, "averageWordLength": 0
        })
    );
    assert!(
        text(&["stats", "a b"], "")
            .out
            .lines()
            .any(|l| l.split_whitespace().collect::<Vec<_>>() == ["kata", "2"])
    );
    assert_eq!(doc(&["stats", "Pi is 3.14. Yes"], "")["sentences"], 2);
}

#[test]
fn reverse_graphemes_and_line_order() {
    assert_eq!(text(&["reverse", "abc"], "").out, "cba\n");
    assert_eq!(text(&["reverse"], "ab\ncd\n").out, "ba\ndc\n");
    assert_eq!(text(&["reverse", "e\u{301}a👍🏽"], "").out, "👍🏽ae\u{301}\n");
    assert_eq!(text(&["reverse", "--lines"], "ab\ncd\nef\n").out, "ef\ncd\nab\n");
    assert_eq!(doc(&["reverse"], "ab\ncd"), json!({ "lines": ["ba", "dc"] }));
}

#[test]
fn slug_diacritics_separator_and_errors() {
    assert_eq!(
        text(&["slug", "  Héllo,   Wörld!! Ça va? "], "").out,
        "hello-world-ca-va\n"
    );
    assert_eq!(
        text(&["slug", "--sep", "_", "It's A Test--2"], "").out,
        "its_a_test_2\n"
    );
    assert_eq!(doc(&["slug", "A B"], ""), json!({ "slug": "a-b" }));
    let r = text(&["slug", "!!! ---"], "");
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Slug kosong"));
    for sep in ["ab", "a", "1", " ", "\u{feff}"] {
        assert_eq!(text(&["slug", "--sep", sep, "x"], "").code, 2);
    }
    assert_eq!(text(&["slug", "--sep", "😀", "A B"], "").out, "a😀b\n");
}

#[test]
fn frequency_ranking_folding_limits_and_json() {
    let input = "the cat The dog the Cat a bird";
    assert_eq!(text(&["freq"], input).out, "3  the\n2  cat\n1  a\n1  bird\n1  dog\n");
    assert_eq!(text(&["freq", "--top", "1"], input).out, "3  the\n");
    assert_eq!(
        text(&["freq", "--case-sensitive", "--top", "2"], input).out,
        "2  the\n1  Cat\n"
    );
    assert_eq!(text(&["freq", "--min-length", "4"], input).out, "1  bird\n");
    assert_eq!(
        doc(&["freq", "--top", "2"], input),
        json!({ "total": 8, "unique": 5, "top": [
        { "word": "the", "count": 3 }, { "word": "cat", "count": 2 }
    ] })
    );
    assert_eq!(text(&["freq", "--top", "0", "x"], "").code, 2);
    assert_eq!(text(&["freq", "--min-length", "abc", "x"], "").code, 2);
}

#[test]
fn line_processing_order_and_numbering() {
    let input = "  b \na\n\nb\n a\n";
    assert_eq!(text(&["lines", "--sort"], input).out, "\n  b \n a\na\nb\n");
    assert_eq!(text(&["lines", "--trim", "--unique"], input).out, "b\na\n\n");
    assert_eq!(
        text(&["lines", "--trim", "--unique", "--sort", "--reverse"], input).out,
        "b\na\n\n"
    );
    assert_eq!(
        text(&["lines", "--reverse", "--numbered"], "x\ny\n").out,
        "1  y\n2  x\n"
    );
    assert_eq!(
        doc(&["lines", "--unique"], "a\nb\na\n"),
        json!({ "count": 2, "lines": ["a", "b"] })
    );
    assert_eq!(text(&["lines"], "").out, "");
}

#[test]
fn usage_errors() {
    assert_eq!(text(&[], "").code, 2);
    assert_eq!(text(&["nope", "x"], "").code, 2);
    let r = text(&["stats", "--sort", "x"], "");
    assert_eq!(r.code, 2);
    assert!(r.err.contains("--sort tidak berlaku untuk \"stats\""));
    assert_eq!(text(&["lines", "--numbered", "--json"], "a").code, 2);
}

#[test]
fn long_punctuation_runs_and_sentence_boundaries() {
    assert_eq!(doc(&["stats"], format!("{}x", ".".repeat(200_000)))["sentences"], 1);
    for (input, expected) in [
        ("... !? …", 0),
        ("Hi... Yes! No?\n", 3),
        ("Hello...world! Next", 2),
        ("Pi is 3.14. Yes", 2),
        ("One?!\tTwo…\u{a0}Three.", 3),
        ("\u{10400}... １２３!", 2),
        ("Only words", 1),
        ("🙂... 🙂!", 0),
    ] {
        assert_eq!(doc(&["stats"], input)["sentences"], expected, "{input:?}");
    }
}

#[test]
fn slug_keeps_non_latin_marks() {
    for (input, expected) in [
        ("Héllo Wörld", "hello-world"),
        ("ガパ", "ガパ"),
        ("カ\u{3099} ハ\u{309a}", "ガ-パ"),
        ("हिन्दी भाषा", "हिन्दी-भाषा"),
        ("a\u{1ab0}", "a\u{1ab0}"),
    ] {
        assert_eq!(text(&["slug", input], "").out.trim(), expected);
    }
}

#[test]
fn stats_bytes_are_raw_not_replacement_encoded() {
    for (input, expected) in [
        (vec![0xff, 0xfe], 2),
        (vec![0xff, 0xfe, 0x0a], 2),
        (vec![0xff, 0xfe, 0x0d, 0x0a], 2),
        (vec![0xff, 0xfe, 0x0a, 0x0a], 3),
        (vec![0xff, 0xfe, 0x0d], 3),
    ] {
        assert_eq!(doc(&["stats"], input)["bytes"], expected);
    }
    assert_eq!(doc(&["stats"], "ab\n")["bytes"], 2);
    assert_eq!(doc(&["stats"], "é\r\n")["bytes"], 2);
}

#[test]
fn code_point_sort_differs_from_utf16_frequency_tie_order() {
    let lines = [
        "😀",
        "\u{ffff}",
        "Ａ",
        "z",
        "\u{10000}",
        "\u{10ffff}",
        "ab",
        "a",
        "a😀",
        "a\u{ffff}",
    ];
    assert_eq!(
        doc(&["lines", "--sort"], format!("{}\n", lines.join("\n")))["lines"],
        json!([
            "a",
            "ab",
            "a\u{ffff}",
            "a😀",
            "z",
            "Ａ",
            "\u{ffff}",
            "\u{10000}",
            "😀",
            "\u{10ffff}"
        ])
    );
    // Astral letters count as one code point for minimum length, not two UTF-16 units.
    assert_eq!(doc(&["freq", "--min-length", "2"], "𐐀 a")["total"], 0);
    assert_eq!(
        doc(&["freq", "--case-sensitive"], "𐐀 Ａ")["top"],
        json!([
            { "word": "𐐀", "count": 1 }, { "word": "Ａ", "count": 1 }
        ])
    );
}

#[test]
fn crlf_single_trailing_newline_and_lone_cr() {
    assert_eq!(
        doc(&["lines"], "a\r\nb\r\n"),
        json!({ "count": 2, "lines": ["a", "b"] })
    );
    assert_eq!(doc(&["lines"], "a\r"), json!({ "count": 1, "lines": ["a\r"] }));
    assert_eq!(doc(&["lines"], "a\r\r\n"), json!({ "count": 1, "lines": ["a\r"] }));
    assert_eq!(doc(&["lines"], "a\r\n\n"), json!({ "count": 2, "lines": ["a", ""] }));
    assert_eq!(doc(&["lines"], "\n"), json!({ "count": 1, "lines": [""] }));
    assert_eq!(
        doc(&["lines", "--trim"], "\u{feff}a\u{a0}\n\u{85}b\u{85}"),
        json!({ "count": 2, "lines": ["a", "\u{85}b\u{85}"] })
    );
}

#[test]
fn file_input_and_mutually_exclusive_sources() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.txt");
    std::fs::write(&path, "ab\r\ncd\r\n").unwrap();
    assert_eq!(dt(&["text", "reverse", "-f", path.to_str().unwrap()]).out, "ba\ndc\n");
    assert_eq!(dt(&["text", "reverse", "-f", path.to_str().unwrap(), "x"]).code, 2);
}

#[test]
fn lone_surrogates_at_cli_boundary_follow_utf8_replacement_rules() {
    // The legacy internal-only surrogate test has no Rust str equivalent. Both CLIs decode
    // surrogate UTF-8 bytes as replacement characters before processing Unicode code points.
    let input = b"\xf0\x90\x80\x80\n\xed\xb0\x80\n\xee\x80\x80\n\xed\xa0\x80\n";
    assert_eq!(
        doc(&["lines", "--sort"], input)["lines"],
        json!(["\u{e000}", "���", "���", "\u{10000}"])
    );
}

#[test]
fn repeated_options_report_first_inserted() {
    let r = text(&["stats", "--trim", "--sort", "--trim", "x"], "");
    assert_eq!(r.code, 2);
    assert!(r.err.contains("--trim tidak berlaku untuk \"stats\""), "{}", r.err);
}
