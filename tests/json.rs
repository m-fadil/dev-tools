mod common;
use common::{dt, dt_in};
#[test]
fn formatting_indent_minify_and_file_bom() {
    assert_eq!(
        dt(&["json", "{\"a\":[1,2]}"]).out,
        "{\n  \"a\": [\n    1,\n    2\n  ]\n}\n"
    );
    assert_eq!(dt(&["json", "--indent", "4", "[1]"]).out, "[\n    1\n]\n");
    assert_eq!(dt(&["json", "--indent", "0", "[1, 2]"]).out, "[1,2]\n");
    assert_eq!(
        dt_in(&["json", "-m"], b"{ \"a\": 1,\n \"b\": [ true, null ] }\n").out,
        "{\"a\":1,\"b\":[true,null]}\n"
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.json");
    std::fs::write(&file, "\u{feff}{\"a\":1}").unwrap();
    assert_eq!(dt(&["json", "-m", "-f", file.to_str().unwrap()]).out, "{\"a\":1}\n");
}
#[test]
fn recursive_sort_preserves_arrays_and_proto() {
    for (input, expected) in [
        (
            "{\"b\":1,\"a\":[{\"d\":1,\"c\":2},0]}",
            "{\"a\":[{\"c\":2,\"d\":1},0],\"b\":1}\n",
        ),
        ("{\"b\":1,\"__proto__\":2}", "{\"__proto__\":2,\"b\":1}\n"),
    ] {
        assert_eq!(dt(&["json", "-m", "-s", input]).out, expected);
    }
}
#[test]
fn validation_exact_legacy_diagnostics() {
    let r = dt_in(&["json", "--check"], b"{\"a\":[1,2]}\n");
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (0, "valid\n", ""));
    for (input, error) in [
        (
            "[1,\n2,\nx]",
            "invalid: 3:1 Unexpected token 'x', \"[1,\n2,\nx]\" is not valid JSON\n",
        ),
        (
            "{\"a\":1",
            "invalid: 1:7 Expected ',' or '}' after property value in JSON at position 6\n",
        ),
        (
            "{\"a\":1} x",
            "invalid: 1:9 Unexpected non-whitespace character after JSON at position 8\n",
        ),
        ("", "invalid: 1:1 Unexpected end of JSON input\n"),
        (
            "{\"a\":}",
            "invalid: 1:6 Unexpected token '}', \"{\"a\":}\" is not valid JSON\n",
        ),
        (
            "[1,]",
            "invalid: 1:4 Unexpected token ']', \"[1,]\" is not valid JSON\n",
        ),
        ("01", "invalid: 1:2 Unexpected number in JSON at position 1\n"),
        (
            "1.",
            "invalid: 1:3 Unterminated fractional number in JSON at position 2\n",
        ),
        (
            "1e+",
            "invalid: 1:4 Exponent part is missing a number in JSON at position 3\n",
        ),
        ("\"abc", "invalid: 1:5 Unterminated string in JSON at position 4\n"),
        ("\"a\\q\"", "invalid: 1:4 Bad escaped character in JSON at position 3\n"),
        ("\"\\u12x4\"", "invalid: 1:6 Bad Unicode escape in JSON at position 5\n"),
    ] {
        let r = dt_in(&["json", "--check"], input.as_bytes());
        assert_eq!(r.code, 1, "{input}");
        assert_eq!(r.out, "");
        assert_eq!(r.err, error, "{input}");
    }
    let r = dt(&["json", "{\"a\":}"]);
    assert_eq!(r.code, 1);
    assert!(r.out.is_empty());
    assert!(r.err.starts_with("invalid: 1:6 "));
}
#[test]
fn query_paths() {
    let doc = "{\"a\":{\"b\":[10,{\"c\":\"x\"}]},\"k.d\":{\"e f\":true},\"n\":null}";
    for (path, raw, expected) in [
        ("a.b[0]", false, "10\n"),
        ("$.a.b[1].c", false, "\"x\"\n"),
        (".a.b[1].c", true, "x\n"),
        ("[\"k.d\"][\"e f\"]", false, "true\n"),
        ("n", false, "null\n"),
        ("a.b[0]", true, "10\n"),
    ] {
        let mut args = vec!["json", "-q", path];
        if raw {
            args.push("-r");
        }
        args.push(doc);
        assert_eq!(dt(&args).out, expected);
    }
    assert_eq!(dt(&["json", "-q", "$", doc]).out, dt(&["json", doc]).out);
    assert_eq!(dt(&["json", "-q", "a.b", "-m", doc]).out, "[10,{\"c\":\"x\"}]\n");
}
#[test]
fn missing_and_invalid_queries() {
    for path in ["a.zzz", "a.b[5]", "a.b.c", "a[0]", "toString"] {
        let r = dt(&["json", "-q", path, "{\"a\":{\"b\":[1]}}"]);
        assert_eq!(r.code, 1);
        assert_eq!(r.err.trim(), format!("path tidak ditemukan: {path}"));
    }
    for path in [
        "a[",
        "a[x]",
        "a[-1]",
        "a..b",
        "a[0]b",
        "[\"x]",
        "[01]",
        "[00]",
        "[000]",
        "[9007199254740992]",
        "[99999999999999999999]",
        "a[01]",
    ] {
        let r = dt(&["json", "-q", path, "[1,2]"]);
        assert_eq!(r.code, 2, "{path}");
        assert!(r.out.is_empty());
        assert!(r.err.contains("Path query tidak valid"));
    }
    assert_eq!(dt(&["json", "-q", "[9007199254740991]", "[1,2]"]).code, 1);
    assert_eq!(dt(&["json", "-m", "-q", "[0]", "[1,2]"]).out, "1\n");
    assert_eq!(dt(&["json", "-q", "[\"01\"]", "{\"01\":\"key\"}"]).out, "\"key\"\n");
}
#[test]
fn conflicting_options() {
    for args in [
        vec!["json", "--check", "--query", "a", "{}"],
        vec!["json", "--check", "-m", "{}"],
        vec!["json", "-m", "--indent", "4", "{}"],
        vec!["json", "--indent", "9", "{}"],
        vec!["json", "--indent", "-1", "{}"],
        vec!["json", "-r", "{}"],
        vec!["json", "--check", "-s", "{}"],
        vec!["json", "--check", "--indent", "2", "{}"],
    ] {
        assert_eq!(dt(&args).code, 2);
    }
}
#[test]
fn positions_never_mined_from_input() {
    for input in ["a at position 99999", "x end of JSON input", "\nx at position 99999"] {
        let r = dt(&["json", "--check", input]);
        assert_eq!(r.code, 1);
        assert!(r.out.is_empty());
        assert!(r.err.starts_with(if input.starts_with('\n') {
            "invalid: 2:1 "
        } else {
            "invalid: 1:1 "
        }));
    }
}
#[test]
fn excessive_depth_fails_cleanly_in_every_mode() {
    let deep = "[".repeat(200000);
    for (args, input) in [
        (vec!["json", "--check"], deep.clone()),
        (vec!["json", "--check"], format!("{deep}{}", "]".repeat(200000))),
        (vec!["json", "-s"], format!("{deep}{}", "]".repeat(200000))),
        (vec!["json"], format!("{deep}{}", "]".repeat(200000))),
        (vec!["json", "-m"], format!("{deep}{}", "]".repeat(200000))),
        (
            vec!["json", "-s", "-m"],
            format!("{}0{}", "{\"a\":".repeat(200000), "}".repeat(200000)),
        ),
    ] {
        let r = dt_in(&args, input.as_bytes());
        assert_eq!(r.code, 1);
        assert!(r.out.is_empty());
        assert_eq!(r.err.trim(), "JSON terlalu dalam (maks 2000 level)");
    }
}
#[test]
fn depth_limit_counts_containers_including_empty() {
    for (open, close) in [("[", "]"), ("{\"a\":", "}")] {
        for value in ["0", if open == "[" { "[]" } else { "{}" }] {
            let depth = if value == "0" { 2000 } else { 1999 };
            let input = format!("{}{value}{}", open.repeat(depth), close.repeat(depth));
            let r = dt_in(&["json", "--check"], input.as_bytes());
            assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (0, "valid\n", ""));
            let r = dt_in(&["json", "-s", "-m"], input.as_bytes());
            assert_eq!(r.code, 0, "{}", r.err);
            assert_eq!(r.out, format!("{input}\n"));
        }
        let input = format!("{}0{}", open.repeat(2001), close.repeat(2001));
        for args in [&["json", "--check"][..], &["json", "-s", "-m"][..], &["json", "-m"][..]] {
            let r = dt_in(args, input.as_bytes());
            assert_eq!(r.code, 1);
            assert_eq!(r.err.trim(), "JSON terlalu dalam (maks 2000 level)");
        }
    }
    let r = dt_in(&["json", "--check"], format!("{}x", "[".repeat(2000)).as_bytes());
    assert_eq!(r.code, 1);
    assert!(r.err.starts_with("invalid: 1:2001 "));
    let input = serde_json::to_string(&"[".repeat(2001)).unwrap();
    assert_eq!(dt_in(&["json", "--check"], input.as_bytes()).out, "valid\n");
}
#[test]
fn arbitrary_precision_integer_and_legacy_float_ordering() {
    assert_eq!(
        dt(&["json", "-m", "{\"z\":9007199254740993,\"2\":-0,\"1\":1e-7,\"a\":1e999}"]).out,
        "{\"1\":1e-7,\"2\":0,\"z\":9007199254740993,\"a\":null}\n"
    );
    assert_eq!(
        dt(&["json", "-m", "1234567890123456789012345678901234567890"]).out,
        "1234567890123456789012345678901234567890\n"
    );
}

#[test]
fn surrogate_and_escape_boundaries_preserve_real_unicode() {
    let input = r#"["\uFFFFd800;","\ud800","\ud83d\udca9","\ue000","\\ud800"]"#;
    let expected = format!(
        "[\"{}d800;\",\"\\ud800\",\"💩\",\"{}\",\"\\\\ud800\"]\n",
        '\u{ffff}', '\u{e000}'
    );
    let r = dt(&["json", "-m", input]);
    assert_eq!((r.code, r.out, r.err), (0, expected, String::new()));
    assert_eq!(dt(&["json", "--check", r#""\ud800""#]).out, "valid\n");
    assert_eq!(dt(&["json", "-q", "s", "-r", r#"{"s":"\ud800"}"#]).out, "�\n");
    let object = r#"{"\uffff":1,"\ud800":2,"\ue000":3}"#;
    assert_eq!(
        dt(&["json", "-s", "-m", object]).out,
        format!("{{\"\\ud800\":2,\"{}\":3,\"{}\":1}}\n", '\u{e000}', '\u{ffff}')
    );
    let key = format!("[\"{}\"]", '\u{ffff}');
    assert_eq!(dt(&["json", "-q", &key, object]).out, "1\n");
    let key = format!("[\"{}\"]", '\u{e000}');
    assert_eq!(dt(&["json", "-q", &key, object]).out, "3\n");
    // A literal marker followed by marker-like data must never decode as a surrogate.
    let object = format!("{{\"{}d800;\":\"{}{}\"}}", '\u{ffff}', '\u{ffff}', '\u{e001}');
    let key = format!("[\"{}d800;\"]", '\u{ffff}');
    assert_eq!(
        dt(&["json", "-q", &key, "-r", &object]).out,
        format!("{}{}\n", '\u{ffff}', '\u{e001}')
    );
}
