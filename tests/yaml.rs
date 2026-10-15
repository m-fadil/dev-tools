mod common;
use common::{Out, dt, dt_in};

fn yaml(args: &[&str], stdin: &str) -> Out {
    let mut full = vec!["yaml"];
    full.extend_from_slice(args);
    dt_in(&full, stdin.as_bytes())
}

fn ok(args: &[&str], stdin: &str) -> String {
    let r = yaml(args, stdin);
    assert_eq!((r.code, r.err.as_str()), (0, ""), "args {args:?}");
    r.out
}

fn fail(args: &[&str], stdin: &str, code: i32) -> String {
    let r = yaml(args, stdin);
    assert_eq!(r.code, code, "args {args:?}: {}", r.err);
    assert_eq!(r.out, "", "no stdout on failure: {args:?}");
    r.err
}

#[test]
fn format_normalizes_and_drops_comments() {
    let input = "# top\nb:   1   # trailing\na:\n    - x\n    -   y\nc: {k: v}\n";
    assert_eq!(ok(&[], input), "b: 1\na:\n- x\n- y\nc:\n  k: v\n");
    assert_eq!(ok(&["b: 1"], ""), "b: 1\n");
}

#[test]
fn strings_that_look_like_other_types_stay_quoted() {
    assert_eq!(
        ok(&[], "a: \"true\"\nb: '123'\nc: \"x\\ny\"\nd: ~\n"),
        "a: 'true'\nb: '123'\nc: |-\n  x\n  y\nd: null\n"
    );
}

#[test]
fn key_order_is_preserved_and_sort_keys_is_recursive() {
    let input = "zebra: 1\napple:\n  y: 1\n  x: [{b: 1, a: 2}, 0]\nmango: 3\n";
    assert_eq!(
        ok(&[], input),
        "zebra: 1\napple:\n  y: 1\n  x:\n  - b: 1\n    a: 2\n  - 0\nmango: 3\n"
    );
    assert_eq!(
        ok(&["--sort-keys"], input),
        "apple:\n  x:\n  - a: 2\n    b: 1\n  - 0\n  y: 1\nmango: 3\nzebra: 1\n"
    );
    assert_eq!(
        ok(&["--to", "json", "-m"], input),
        "{\"zebra\":1,\"apple\":{\"y\":1,\"x\":[{\"b\":1,\"a\":2},0]},\"mango\":3}\n"
    );
    // JSON integer-like keys keep their position too (no JavaScript reordering).
    assert_eq!(ok(&["--to", "json", "-m"], "b: 1\n2: x\n"), "{\"b\":1,\"2\":\"x\"}\n");
}

#[test]
fn validate_reports_ok_and_located_errors() {
    let r = yaml(&["--validate"], "a: 1\nb: [x, y]\n");
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (0, "valid\n", ""));

    let err = fail(&["--validate"], "a: [1\nb: 2\n", 1);
    assert!(err.starts_with("invalid: 2:2 "), "{err}");
    // The position is reported once, in the prefix, not again inside the message.
    assert!(!err.contains("at line 2 column 2"), "{err}");

    let err = fail(&[], "a: 1\n  b: 2\n", 1);
    assert!(err.starts_with("invalid: 2:4 mapping values are not allowed"), "{err}");

    let err = fail(&["--validate"], "a: 1\na: 2\n", 1);
    assert!(
        err.starts_with("invalid: ") && err.contains("duplicate entry with key \"a\""),
        "{err}"
    );

    let err = fail(&[], "a: *missing\n", 1);
    assert!(err.starts_with("invalid: 1:4 unknown anchor"), "{err}");
}

#[test]
fn only_first_document_unless_all_but_validate_checks_every_document() {
    let stream = "a: 1\n---\nb: 2\n---\nc: 1\n  d: 2\n";
    assert_eq!(ok(&[], stream), "a: 1\n");
    assert_eq!(ok(&["--to", "json", "-m"], stream), "{\"a\":1}\n");
    let err = fail(&["--validate"], stream, 1);
    assert!(err.starts_with("invalid: 6:4 mapping values are not allowed"), "{err}");
    assert_eq!(fail(&["--all"], stream, 1), err);
}

#[test]
fn all_streams_documents_as_yaml_or_json_array() {
    let stream = "a: 1\n---\n- x\n- y\n---\nplain\n";
    assert_eq!(ok(&["--all"], stream), "a: 1\n---\n- x\n- y\n---\nplain\n");
    assert_eq!(
        ok(&["--all", "--to", "json", "-m"], stream),
        "[{\"a\":1},[\"x\",\"y\"],\"plain\"]\n"
    );
    // A single document is still an array under --all.
    assert_eq!(ok(&["--all", "--to", "json", "-m"], "a: 1\n"), "[{\"a\":1}]\n");
    // An empty stream is one null document.
    assert_eq!(ok(&["--all", "--to", "json", "-m"], ""), "[null]\n");
    assert_eq!(
        ok(&["--all", "-q", "n", "--to", "json", "-m"], "n: x\n---\nn: y\n"),
        "[\"x\",\"y\"]\n"
    );
    assert_eq!(ok(&["--validate"], "a: 1\n---\nb: 2\n"), "valid\n");
}

#[test]
fn anchors_aliases_and_merge_keys_are_resolved() {
    let input = "base: &b\n  x: 1\n  y: [1, 2]\nchild:\n  <<: *b\n  x: 9\n  z: 3\nlist: *b\nname: &n bob\nagain: *n\n";
    assert_eq!(
        ok(&["--to", "json", "-m"], input),
        "{\"base\":{\"x\":1,\"y\":[1,2]},\"child\":{\"x\":9,\"z\":3,\"y\":[1,2]},\"list\":{\"x\":1,\"y\":[1,2]},\"name\":\"bob\",\"again\":\"bob\"}\n"
    );
    assert_eq!(ok(&[], "a: &v [1, 2]\nb: *v\n"), "a:\n- 1\n- 2\nb:\n- 1\n- 2\n");
    // Several sources: the explicit key wins, then the earlier source.
    assert_eq!(
        ok(
            &["--to", "json", "-m"],
            "a: &a {p: 1, q: 1}\nb: &b {q: 2, r: 2}\nc:\n  <<: [*a, *b]\n  s: 0\n  p: 9\n"
        ),
        "{\"a\":{\"p\":1,\"q\":1},\"b\":{\"q\":2,\"r\":2},\"c\":{\"s\":0,\"p\":9,\"q\":1,\"r\":2}}\n"
    );
    let err = fail(&[], "a: &x 1\nb:\n  <<: *x\n", 1);
    assert!(err.starts_with("invalid: ") && err.contains("merge key <<"), "{err}");
}

#[test]
fn to_json_pretty_and_minify() {
    assert_eq!(
        ok(&["--to", "json"], "a: [1, true, null, 'x']\nb:\n  c: 1.5\n"),
        "{\n  \"a\": [\n    1,\n    true,\n    null,\n    \"x\"\n  ],\n  \"b\": {\n    \"c\": 1.5\n  }\n}\n"
    );
    assert_eq!(
        ok(&["--to", "json", "-m"], "a: [1, true, null, 'x']\nb:\n  c: 1.5\n"),
        "{\"a\":[1,true,null,\"x\"],\"b\":{\"c\":1.5}}\n"
    );
    assert_eq!(ok(&["--to", "json", "-m"], "~\n"), "null\n");
    assert_eq!(ok(&["--to", "json", "-m"], "- 1\n- [2]\n"), "[1,[2]]\n");
    // Non-string scalar keys become strings, tags are dropped.
    assert_eq!(
        ok(
            &["--to", "json", "-m"],
            "1: a\ntrue: b\nnull: c\nk: !!str 12\nt: !x {a: 1}\n"
        ),
        "{\"1\":\"a\",\"true\":\"b\",\"null\":\"c\",\"k\":\"12\",\"t\":{\"a\":1}}\n"
    );
}

#[test]
fn from_json_produces_yaml_and_round_trips() {
    let json = "{\"z\":1,\"a\":{\"b\":[1,2.5,{\"c\":true}],\"k\":\"v\"},\"s\":\"true\",\"n\":null,\"e\":[],\"o\":{}}";
    let text = ok(&["--from", "json"], json);
    assert_eq!(
        text,
        "z: 1\na:\n  b:\n  - 1\n  - 2.5\n  - c: true\n  k: v\ns: 'true'\nn: null\ne: []\no: {}\n"
    );
    assert_eq!(ok(&["--to", "json", "-m"], &text), format!("{json}\n"));
    assert_eq!(
        ok(&["--from", "json", "-s"], "{\"b\":1,\"a\":{\"d\":1,\"c\":2}}"),
        "a:\n  c: 2\n  d: 1\nb: 1\n"
    );
    // Floats stay floats.
    assert_eq!(ok(&["--from", "json"], "{\"a\":1.0,\"b\":1e2}"), "a: 1.0\nb: 100.0\n");
}

#[test]
fn from_json_errors_are_located() {
    let err = fail(&["--from", "json"], "{\"a\":}", 1);
    assert!(err.starts_with("invalid: 1:6 "), "{err}");
    let err = fail(&["--from", "json"], "[1,\n2", 1);
    assert!(err.starts_with("invalid: 2:"), "{err}");
    let err = fail(&["--from", "json"], "", 1);
    assert!(err.starts_with("invalid: 1:1 "), "{err}");
}

#[test]
fn big_integers_stay_exact_within_64_bits() {
    let y = "max: 18446744073709551615\nmin: -9223372036854775808\nsafe: 9007199254740993\n";
    assert_eq!(
        ok(&["--to", "json", "-m"], y),
        "{\"max\":18446744073709551615,\"min\":-9223372036854775808,\"safe\":9007199254740993}\n"
    );
    assert_eq!(ok(&[], y), y);
    assert_eq!(
        ok(
            &["--from", "json"],
            "{\"max\":18446744073709551615,\"min\":-9223372036854775808,\"safe\":9007199254740993}"
        ),
        y
    );
}

#[test]
fn integers_beyond_64_bits_are_rejected_not_rounded() {
    let err = fail(&[], "id: 123456789012345678901234567890\n", 1);
    assert_eq!(
        err,
        "invalid: 1:5 id: bilangan bulat di luar jangkauan 64-bit (i64/u64): 123456789012345678901234567890\n"
    );
    let err = fail(&["--from", "json"], "{\"a\":{\"id\":18446744073709551616}}", 1);
    assert_eq!(err, "bilangan bulat di a.id di luar jangkauan: 18446744073709551616\n");
    let err = fail(&["--from", "json"], "[-9223372036854775809]", 1);
    assert_eq!(err, "bilangan bulat di [0] di luar jangkauan: -9223372036854775809\n");
}

#[test]
fn values_json_cannot_hold_name_their_path() {
    let err = fail(&["--to", "json"], "a:\n  - 1\n  - .nan\n", 1);
    assert_eq!(err, ".nan di a[1] tak bisa jadi JSON\n");
    let err = fail(&["--to", "json"], "? [1, 2]\n: x\n", 1);
    assert_eq!(err, "key di $ berupa urutan/mapping, JSON hanya menerima key string\n");
    let err = fail(&["--to", "json"], "o:\n  1: a\n  \"1\": b\n", 1);
    assert_eq!(err, "key ganda setelah dikonversi ke string JSON: o.1\n");
    // Plain YAML output has no such limit.
    assert_eq!(ok(&[], "a: .nan\n"), "a: .nan\n");
}

#[test]
fn query_selects_values() {
    let doc = "items:\n  - name: a\n    tags: [x, y]\n  - name: b\n'k.dot':\n  \"q r\": 7\n1: one\n";
    assert_eq!(ok(&["-q", "items[1].name"], doc), "b\n");
    assert_eq!(ok(&["-q", "$.items[0].tags"], doc), "- x\n- y\n");
    assert_eq!(
        ok(&["-q", "items[0]", "--to", "json", "-m"], doc),
        "{\"name\":\"a\",\"tags\":[\"x\",\"y\"]}\n"
    );
    assert_eq!(ok(&["-q", "[\"k.dot\"][\"q r\"]"], doc), "7\n");
    assert_eq!(ok(&["-q", "1"], doc), "one\n");
    assert_eq!(ok(&["-q", "items[0].name", "--raw"], doc), "a\n");
    assert_eq!(ok(&["-q", "items[0].tags", "--raw"], doc), "- x\n- y\n");
    assert_eq!(ok(&["-q", "items[0].name", "--to", "json"], doc), "\"a\"\n");
    assert_eq!(ok(&["-q", "items[0].name", "--to", "json", "-r"], doc), "a\n");
    assert_eq!(ok(&["-q", "$", "-m", "--to", "json"], "a: 1\n"), "{\"a\":1}\n");
    // Query on JSON input happens before conversion.
    assert_eq!(
        ok(&["--from", "json", "-q", "a.b[1]"], "{\"a\":{\"b\":[1,{\"c\":2}]}}"),
        "c: 2\n"
    );
    assert_eq!(ok(&["--from", "json", "-q", "a", "-r"], "{\"a\":\"txt\"}"), "txt\n");
    assert_eq!(ok(&["-q", "a", "-s"], "a: {z: 1, b: 2}\n"), "b: 2\nz: 1\n");
}

#[test]
fn query_errors() {
    assert_eq!(
        fail(&["-q", "items[5]"], "items: [1]\n", 1),
        "path tidak ditemukan: items[5]\n"
    );
    assert_eq!(fail(&["-q", "a.b"], "a: 1\n", 1), "path tidak ditemukan: a.b\n");
    assert_eq!(fail(&["-q", "0"], "- x\n", 1), "path tidak ditemukan: 0\n");
    assert_eq!(
        fail(&["--all", "-q", "a"], "a: 1\n---\nb: 1\n", 1),
        "path tidak ditemukan: a (dokumen 2)\n"
    );
    assert_eq!(
        fail(&["--from", "json", "-q", "x"], "{\"a\":1}", 1),
        "path tidak ditemukan: x\n"
    );
    let err = fail(&["-q", "a["], "a: 1\n", 2);
    assert!(err.starts_with("Path query tidak valid"), "{err}");
}

#[test]
fn empty_input_is_a_null_document() {
    assert_eq!(ok(&[], ""), "null\n");
    assert_eq!(ok(&["--to", "json"], "# only a comment\n"), "null\n");
    assert_eq!(ok(&["--validate"], ""), "valid\n");
}

#[test]
fn input_from_file_stdin_and_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.yml");
    std::fs::write(&file, "\u{feff}a:   1\nb: 2\n").unwrap();
    let path = file.to_str().unwrap();
    assert_eq!(ok(&["-f", path], ""), "a: 1\nb: 2\n");
    assert_eq!(
        ok(&["--file", path, "--to", "json", "-m"], "ignored stdin"),
        "{\"a\":1,\"b\":2}\n"
    );
    assert_eq!(ok(&["--validate", "--file", path], ""), "valid\n");
    assert_eq!(ok(&["a:   1"], "ignored: stdin\n"), "a: 1\n");
    assert_eq!(ok(&["a:", "1"], ""), "a: 1\n");

    let err = fail(&["a: 1", "-f", path], "", 2);
    assert!(err.starts_with("Gunakan teks atau --file, bukan keduanya."), "{err}");
    let missing = dir.path().join("none.yml");
    let err = fail(&["-f", missing.to_str().unwrap()], "", 1);
    assert!(err.starts_with("ENOENT: no such file or directory, open "), "{err}");
    std::fs::write(&file, [b'a', b':', b' ', 0xff, b'\n']).unwrap();
    let err = fail(&["-f", path], "", 1);
    assert_eq!(err, "invalid: input bukan UTF-8 yang valid (byte ke-4)\n");
}

#[test]
fn usage_errors_exit_2() {
    for (args, message) in [
        (vec!["--to", "xml"], "Nilai --to tidak valid: xml. Pilihan: json"),
        (vec!["--from", "toml"], "Nilai --from tidak valid: toml. Pilihan: json"),
        (
            vec!["--to", "json", "--from", "json"],
            "--to dan --from tidak bisa bersamaan.",
        ),
        (vec!["-m"], "--minify hanya berlaku dengan --to json."),
        (vec!["--from", "json", "-m"], "--minify hanya berlaku dengan --to json."),
        (vec!["-r"], "--raw hanya berlaku dengan --query."),
        (vec!["--all", "--from", "json"], "--all tidak bisa dengan --from."),
        (vec!["--all", "-q", "a", "-r"], "--raw tidak bisa dengan --all."),
        (vec!["--validate", "--to", "json"], "--validate tidak bisa dengan --to."),
        (vec!["--validate", "-q", "a"], "--validate tidak bisa dengan --query."),
        (vec!["--validate", "-s"], "--validate tidak bisa dengan --sort-keys."),
        (vec!["--validate", "--all"], "--validate tidak bisa dengan --all."),
    ] {
        let err = fail(&args, "a: 1\n", 2);
        assert!(err.starts_with(message), "{args:?}: {err}");
    }
    let err = fail(&["--bogus"], "", 2);
    assert!(err.contains("--bogus"), "{err}");
}

#[test]
fn help_lists_options() {
    let r = dt(&["yaml", "--help"]);
    assert_eq!(r.code, 0);
    for needle in [
        "dt yaml",
        "--validate",
        "--to json",
        "--from json",
        "--all",
        "--query",
        "Exit:",
    ] {
        assert!(r.out.contains(needle), "missing {needle}");
    }
    assert_eq!(dt(&["yaml", "-h"]).out, r.out);
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let deep = format!("{}1{}", "[".repeat(500), "]".repeat(500));
    let err = fail(&[], &deep, 1);
    assert!(
        err.starts_with("invalid: ") && err.contains("recursion limit exceeded"),
        "{err}"
    );
    // JSON input may nest up to 2000 levels (like dt json) and must convert without overflowing the stack.
    let json = format!("{}1{}", "[".repeat(1500), "]".repeat(1500));
    let r = yaml(&["--from", "json"], &json);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.out, format!("{}1\n", "- ".repeat(1500)));
    let json = format!("{}1{}", "[".repeat(2001), "]".repeat(2001));
    assert_eq!(
        fail(&["--from", "json"], &json, 1),
        "JSON terlalu dalam (maks 2000 level)\n"
    );
}
