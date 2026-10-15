mod common;
use common::{dt, dt_in};

fn enc(mode: &str, text: &str) -> String {
    let o = dt(&["escape", mode, text]);
    assert_eq!(o.code, 0, "{}", o.err);
    o.out
}

fn dec(mode: &str, text: &str) -> String {
    let o = dt(&["escape", mode, "-d", text]);
    assert_eq!(o.code, 0, "{}", o.err);
    o.out
}

fn dec_err(mode: &str, text: &str) -> String {
    let o = dt(&["escape", mode, "-d", text]);
    assert_eq!(o.code, 1, "stdout: {}", o.out);
    assert_eq!(o.out, "");
    o.err
}

fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("dt-escape-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn json_encode_basics() {
    assert_eq!(enc("json", r#"a "b" \ c"#), "a \\\"b\\\" \\\\ c\n");
    assert_eq!(enc("json", "l1\nl2\r\t\u{8}\u{c}"), "l1\\nl2\\r\\t\\b\\f\n");
    assert_eq!(enc("json", "\u{1}\u{1f}/"), "\\u0001\\u001f/\n");
    assert_eq!(enc("json", "héllo 😀"), "héllo 😀\n");
}

#[test]
fn json_encode_ascii_and_quotes() {
    let o = dt(&["escape", "json", "--ascii", "é😀a"]);
    assert_eq!(o.out, "\\u00e9\\ud83d\\ude00a\n");
    let o = dt(&["escape", "json", "--quotes", "x\"y"]);
    assert_eq!(o.out, "\"x\\\"y\"\n");
    let o = dt(&["escape", "json", "--quotes", ""]);
    assert_eq!(o.out, "\"\"\n");
}

#[test]
fn json_decode_with_and_without_quotes() {
    assert_eq!(dec("json", r#"a\"b\\c\/d\n"#), "a\"b\\c/d\n\n");
    assert_eq!(dec("json", r#""a\tb""#), "a\tb\n");
    assert_eq!(dec("json", r"\u00e9\u00E9"), "éé\n");
    assert_eq!(dec("json", r"\ud83d\ude00"), "😀\n");
    assert_eq!(dec("json", ""), "\n");
}

#[test]
fn json_round_trip() {
    for s in [
        "",
        "plain",
        "q\"uote \\ back",
        "tab\t nl\n cr\r",
        "ünï 😀 \u{7f}\u{1}",
        "\\u0041",
    ] {
        for flags in [&[][..], &["--ascii"][..], &["--quotes"][..]] {
            let mut args = vec!["escape", "json"];
            args.extend_from_slice(flags);
            let e = dt_in(&args, s.as_bytes());
            assert_eq!(e.code, 0, "{}", e.err);
            let escaped = e.out.strip_suffix('\n').unwrap().to_string();
            let d = dt_in(&["escape", "json", "-d"], escaped.as_bytes());
            assert_eq!(d.code, 0, "{}: {}", escaped, d.err);
            assert_eq!(d.out, format!("{s}\n"));
        }
    }
}

#[test]
fn json_decode_errors() {
    assert!(dec_err("json", r"\x").contains("Escape tidak dikenal"));
    assert!(dec_err("json", r"abc\").contains("Backslash"));
    assert!(dec_err("json", r"\u12").contains("\\u"));
    assert!(dec_err("json", r"\u12g4").contains("\\u"));
    assert!(dec_err("json", r"\ud83d").contains("Surrogate tinggi"));
    assert!(dec_err("json", r"\ud83dx").contains("Surrogate tinggi"));
    assert!(dec_err("json", r"\ud83d\u0041").contains("Surrogate tinggi"));
    assert!(dec_err("json", r"\ude00").contains("Surrogate rendah"));
    assert!(dec_err("json", r#"a"b"#).contains("tanpa escape"));
    assert!(dec_err("json", "a\u{1}b").contains("kontrol"));
    assert!(dec_err("json", "\"").contains("tanpa escape"));
}

#[test]
fn html_encode_and_decode() {
    assert_eq!(
        enc("html", r#"<a href="x" t='y'>&</a>"#),
        "&lt;a href=&quot;x&quot; t=&#39;y&#39;&gt;&amp;&lt;/a&gt;\n"
    );
    assert_eq!(enc("html", "é😀"), "é😀\n");
    assert_eq!(
        dec("html", "&lt;b&gt; &amp;amp; &quot;q&quot; &apos;&#39;"),
        "<b> &amp; \"q\" ''\n"
    );
    assert_eq!(dec("html", "&#65;&#x42;&#X43;&#x1F600;"), "ABC😀\n");
    assert_eq!(dec("html", "&nbsp;&copy;&reg;&hellip;&mdash;&ndash;"), "\u{a0}©®…—–\n");
}

#[test]
fn html_decode_leaves_unknown_and_invalid_untouched() {
    for s in [
        "&bogus;",
        "&amp",
        "&",
        "a & b",
        "&#;",
        "&#x;",
        "&#xZZ;",
        "&#0;",
        "&#xD800;",
        "&#x110000;",
        "&#99999999999;",
        "&AMP;",
        "&#-1;",
    ] {
        assert_eq!(dec("html", s), format!("{s}\n"), "input {s}");
    }
    assert_eq!(dec("html", "&&lt;"), "&<\n");
}

#[test]
fn html_round_trip() {
    let s = "<p class=\"a\">Tom & 'Jerry' &amp; &lt;</p> 😀";
    let e = dt_in(&["escape", "html"], s.as_bytes());
    let d = dt_in(&["escape", "html", "-d"], e.out.as_bytes());
    assert_eq!(d.out, format!("{s}\n"));
}

#[test]
fn url_encode_and_decode() {
    assert_eq!(enc("url", "AZaz09-._~"), "AZaz09-._~\n");
    assert_eq!(
        enc("url", "a b&c=d/é😀+%"),
        "a%20b%26c%3Dd%2F%C3%A9%F0%9F%98%80%2B%25\n"
    );
    assert_eq!(dec("url", "a%20b%26%C3%A9%f0%9f%98%80"), "a b&é😀\n");
    assert_eq!(dec("url", "a+b"), "a+b\n");
    let o = dt(&["escape", "url", "-d", "--plus", "a+b%2Bc"]);
    assert_eq!(o.out, "a b+c\n");
    assert_eq!(dec("url", ""), "\n");
}

#[test]
fn url_decode_errors() {
    assert!(dec_err("url", "100%").contains("persen"));
    assert!(dec_err("url", "%zz").contains("persen"));
    assert!(dec_err("url", "a%2").contains("persen"));
    assert!(dec_err("url", "%+1").contains("persen"));
    assert!(dec_err("url", "%FF").contains("UTF-8"));
    assert!(dec_err("url", "%C3").contains("UTF-8"));
}

#[test]
fn url_round_trip() {
    let s = "k=v&x y/é😀~-._ %41+";
    let e = dt_in(&["escape", "url"], s.as_bytes());
    let d = dt_in(&["escape", "url", "-d"], e.out.as_bytes());
    assert_eq!(d.out, format!("{s}\n"));
}

#[test]
fn regex_escape() {
    assert_eq!(
        enc("regex", "a.b*c+d?(e)[f]{g}|h^i$j\\k"),
        "a\\.b\\*c\\+d\\?\\(e\\)\\[f\\]\\{g\\}\\|h\\^i\\$j\\\\k\n"
    );
    assert_eq!(enc("regex", "plain é😀 -#&~"), "plain é😀 \\-\\#\\&\\~\n");
    assert_eq!(enc("regex", ""), "\n");
}

#[test]
fn shell_quote() {
    assert_eq!(enc("shell", "hello world"), "'hello world'\n");
    assert_eq!(enc("shell", "it's"), "'it'\\''s'\n");
    assert_eq!(enc("shell", "''"), "''\\'''\\'''\n");
    assert_eq!(enc("shell", "$HOME `x` \"y\" \\ é"), "'$HOME `x` \"y\" \\ é'\n");
    assert_eq!(enc("shell", ""), "''\n");
}

#[test]
fn unsupported_decode_is_usage_error() {
    for mode in ["regex", "shell"] {
        let o = dt(&["escape", mode, "-d", "x"]);
        assert_eq!(o.code, 2);
        assert!(o.err.contains("tidak mendukung --decode"), "{}", o.err);
        assert_eq!(o.out, "");
    }
}

#[test]
fn usage_errors() {
    let o = dt(&["escape"]);
    assert_eq!(o.code, 2);
    assert!(o.err.contains("Mode wajib"));
    let o = dt(&["escape", "base64", "x"]);
    assert_eq!(o.code, 2);
    assert!(o.err.contains("Mode tidak dikenal"));
    assert_eq!(dt(&["escape", "html", "--ascii", "x"]).code, 2);
    assert_eq!(dt(&["escape", "json", "-d", "--quotes", "x"]).code, 2);
    assert_eq!(dt(&["escape", "url", "--plus", "x"]).code, 2);
    assert_eq!(dt(&["escape", "json", "--bogus", "x"]).code, 2);
}

#[test]
fn help_exits_zero() {
    let o = dt(&["escape", "--help"]);
    assert_eq!(o.code, 0);
    assert!(o.out.contains("dt escape <mode>"));
}

#[test]
fn stdin_input_and_trailing_newline() {
    let o = dt_in(&["escape", "json"], b"a\"b");
    assert_eq!(o.out, "a\\\"b\n");
    // Encoding keeps the input newline as data.
    let o = dt_in(&["escape", "json"], b"a\n");
    assert_eq!(o.out, "a\\n\n");
    // Decoding drops one trailing line ending (echo-style input).
    let o = dt_in(&["escape", "url", "-d"], b"a%20b\n");
    assert_eq!(o.out, "a b\n");
    let o = dt_in(&["escape", "url", "-d"], b"a%20b\r\n");
    assert_eq!(o.out, "a b\n");
}

#[test]
fn file_input() {
    let p = temp_file("in.txt", "tab\t\"q\" é".as_bytes());
    let o = dt(&["escape", "json", "--file", p.to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.err);
    assert_eq!(o.out, "tab\\t\\\"q\\\" é\n");
    let p = temp_file("dec.txt", b"a%20b\n");
    let o = dt(&["escape", "url", "-d", "-f", p.to_str().unwrap()]);
    assert_eq!(o.out, "a b\n");
}

#[test]
fn file_and_text_together_is_usage_error() {
    let p = temp_file("both.txt", b"x");
    let o = dt(&["escape", "json", "-f", p.to_str().unwrap(), "text"]);
    assert_eq!(o.code, 2);
}

#[test]
fn missing_file_fails() {
    let o = dt(&["escape", "json", "-f", "definitely-missing-file.txt"]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("ENOENT"), "{}", o.err);
}

#[test]
fn invalid_utf8_input_fails() {
    let o = dt_in(&["escape", "json"], &[0x61, 0xff, 0xfe]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("UTF-8"));
    let p = temp_file("bin.dat", &[0xc3, 0x28]);
    let o = dt(&["escape", "html", "-f", p.to_str().unwrap()]);
    assert_eq!(o.code, 1);
    assert!(o.err.contains("UTF-8"));
}

#[test]
fn json_output_shape() {
    let o = dt(&["escape", "json", "--json", "a\"b"]);
    let v: serde_json::Value = serde_json::from_str(&o.out).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"mode": "json", "direction": "encode", "output": "a\\\"b"})
    );
    let o = dt(&["escape", "url", "-d", "--json", "a%20b"]);
    let v: serde_json::Value = serde_json::from_str(&o.out).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"mode": "url", "direction": "decode", "output": "a b"})
    );
}

#[test]
fn json_flag_does_not_print_on_error() {
    let o = dt(&["escape", "url", "-d", "--json", "%zz"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.out, "");
}
