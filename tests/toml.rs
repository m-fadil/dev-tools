mod common;
use common::{Out, dt, dt_in};

fn toml(args: &[&str], stdin: &str) -> Out {
    let mut full = vec!["toml"];
    full.extend_from_slice(args);
    dt_in(&full, stdin.as_bytes())
}

fn ok(args: &[&str], stdin: &str) -> String {
    let r = toml(args, stdin);
    assert_eq!((r.code, r.err.as_str()), (0, ""), "args {args:?}");
    r.out
}

fn fail(args: &[&str], stdin: &str, code: i32) -> String {
    let r = toml(args, stdin);
    assert_eq!(r.code, code, "args {args:?}: {}", r.err);
    assert_eq!(r.out, "", "no stdout on failure: {args:?}");
    r.err
}

const SAMPLE: &str = "# comment\ntitle = \"TOML\"\nbig = 9223372036854775807\nf = 1.50\nwhen = 1979-05-27T07:32:00Z\nday = 1979-05-27\n[owner]\nname = \"Tom\"\ntags = [\"a\", \"b\"]\n[[products]]\nname = \"Hammer\"\nsku = 738594937\n[[products]]\nname = \"Nail\"\n[owner.address]\ncity = \"X\"\n";

#[test]
fn format_normalizes_and_drops_comments() {
    assert_eq!(
        ok(&[], SAMPLE),
        "title = \"TOML\"\nbig = 9223372036854775807\nf = 1.5\nwhen = 1979-05-27T07:32:00Z\nday = 1979-05-27\n\n[owner]\nname = \"Tom\"\ntags = [\"a\", \"b\"]\n\n[owner.address]\ncity = \"X\"\n\n[[products]]\nname = \"Hammer\"\nsku = 738594937\n\n[[products]]\nname = \"Nail\"\n"
    );
    assert_eq!(ok(&["a   =   1 # c"], ""), "a = 1\n");
    assert_eq!(ok(&["a", "=", "1"], ""), "a = 1\n");
    assert_eq!(ok(&[], ""), "");
    assert_eq!(ok(&[], "# only a comment\n"), "");
}

#[test]
fn key_order_is_preserved_and_sort_keys_is_recursive() {
    let input = "zebra = 1\napple = 2\n[mid]\nz = 1\na = 2\n[[arr]]\nb = 1\na = 2\n";
    assert_eq!(
        ok(&[], input),
        "zebra = 1\napple = 2\n\n[mid]\nz = 1\na = 2\n\n[[arr]]\nb = 1\na = 2\n"
    );
    assert_eq!(
        ok(&["-s"], input),
        "apple = 2\nzebra = 1\n\n[[arr]]\na = 2\nb = 1\n\n[mid]\na = 2\nz = 1\n"
    );
    assert_eq!(
        ok(&["--to", "json", "-m"], input),
        "{\"zebra\":1,\"apple\":2,\"mid\":{\"z\":1,\"a\":2},\"arr\":[{\"b\":1,\"a\":2}]}\n"
    );
    assert_eq!(ok(&["--to", "json", "-m"], "b = 1\n\"2\" = 2\n"), "{\"b\":1,\"2\":2}\n");
}

#[test]
fn validate_reports_ok_and_located_errors() {
    let r = toml(&["--validate"], SAMPLE);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (0, "valid\n", ""));

    let err = fail(&["--validate"], "a = 1\nb = \nc = 2\n", 1);
    assert!(err.starts_with("invalid: 2:5 "), "{err}");

    let err = fail(&[], "a = [1\n", 1);
    assert!(err.starts_with("invalid: 1:7 unclosed array"), "{err}");

    let err = fail(&[], "a = 1\na = 2\n", 1);
    assert_eq!(err, "invalid: 2:1 duplicate key\n");

    let err = fail(&[], "[a]\nb = 1\n[a]\n", 1);
    assert_eq!(err, "invalid: 3:2 duplicate key\n");

    // Columns count characters, not bytes.
    let err = fail(&[], "é = \"é\"\nb = [\n1,\n", 1);
    assert!(err.starts_with("invalid: 3:3 "), "{err}");
    let err = fail(&[], "x = 1\ny = é\n", 1);
    assert!(err.starts_with("invalid: 2:5 "), "{err}");
}

#[test]
fn to_json_converts_types_and_datetimes_to_strings() {
    assert_eq!(
        ok(&["--to", "json", "-m"], SAMPLE),
        "{\"title\":\"TOML\",\"big\":9223372036854775807,\"f\":1.5,\"when\":\"1979-05-27T07:32:00Z\",\"day\":\"1979-05-27\",\"owner\":{\"name\":\"Tom\",\"tags\":[\"a\",\"b\"],\"address\":{\"city\":\"X\"}},\"products\":[{\"name\":\"Hammer\",\"sku\":738594937},{\"name\":\"Nail\"}]}\n"
    );
    assert_eq!(
        ok(
            &["--to", "json", "-m"],
            "t = 07:32:00\nu = 1979-05-27T00:32:00.5-07:00\nv = 1979-05-27T07:32:00\n"
        ),
        "{\"t\":\"07:32:00\",\"u\":\"1979-05-27T00:32:00.5-07:00\",\"v\":\"1979-05-27T07:32:00\"}\n"
    );
    assert_eq!(
        ok(&["--to", "json"], "a = [1, 2]\n[b]\nc = true\n"),
        "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {\n    \"c\": true\n  }\n}\n"
    );
    assert_eq!(ok(&["--to", "json"], ""), "{}\n");
    assert_eq!(
        ok(&["--to", "json", "-m"], "x = 0xff\ny = 1e3\nz = -0.5\n"),
        "{\"x\":255,\"y\":1000.0,\"z\":-0.5}\n"
    );
}

#[test]
fn from_json_produces_toml_and_round_trips() {
    let json =
        "{\"z\":1,\"top\":2.5,\"a\":{\"b\":[1,[2,3]],\"k\":\"v\"},\"arr\":[{\"x\":1},{\"x\":2}],\"s\":\"1979-05-27\"}";
    let text = ok(&["--from", "json"], json);
    assert_eq!(
        text,
        "z = 1\ntop = 2.5\ns = \"1979-05-27\"\n\n[a]\nb = [1, [2, 3]]\nk = \"v\"\n\n[[arr]]\nx = 1\n\n[[arr]]\nx = 2\n"
    );
    // TOML puts plain values before tables, so only that grouping may differ from the JSON order.
    assert_eq!(
        ok(&["--to", "json", "-m"], &text),
        "{\"z\":1,\"top\":2.5,\"s\":\"1979-05-27\",\"a\":{\"b\":[1,[2,3]],\"k\":\"v\"},\"arr\":[{\"x\":1},{\"x\":2}]}\n"
    );
    assert_eq!(ok(&["--from", "json"], "{}"), "");
    assert_eq!(
        ok(&["--from", "json", "-s"], "{\"b\":1,\"a\":{\"d\":1,\"c\":2}}"),
        "b = 1\n\n[a]\nc = 2\nd = 1\n"
    );
    assert_eq!(ok(&["--from", "json"], "{\"a\":1.0,\"b\":1e2}"), "a = 1.0\nb = 100.0\n");
    assert_eq!(
        ok(&["--from", "json"], "{\"a\":\"multi\\nline\\ttab\"}"),
        "a = \"\"\"\nmulti\nline\\ttab\"\"\"\n"
    );
}

#[test]
fn from_json_rejects_null_with_path() {
    assert_eq!(
        fail(&["--from", "json"], "{\"a\":null}", 1),
        "TOML tidak punya null: nilai null di a\n"
    );
    assert_eq!(
        fail(&["--from", "json"], "{\"a\":{\"b.c\":[1,null]},\"x\":1}", 1),
        "TOML tidak punya null: nilai null di a[\"b.c\"][1]\n"
    );
    assert_eq!(
        fail(&["--from", "json"], "{\"a\":[{\"b\":null}]}", 1),
        "TOML tidak punya null: nilai null di a[0].b\n"
    );
    // A query that avoids the null succeeds.
    assert_eq!(ok(&["--from", "json", "-q", "b"], "{\"a\":null,\"b\":1}"), "1\n");
}

#[test]
fn from_json_requires_a_table_at_top_level() {
    for (input, kind) in [
        ("[1]", "array"),
        ("\"s\"", "string"),
        ("3", "angka"),
        ("true", "boolean"),
        ("null", "null"),
    ] {
        assert_eq!(
            fail(&["--from", "json"], input, 1),
            format!("TOML harus berupa tabel (objek) di tingkat atas, bukan {kind}\n"),
            "{input}"
        );
    }
}

#[test]
fn big_integers_stay_exact_within_i64() {
    let t = "max = 9223372036854775807\nmin = -9223372036854775808\nsafe = 9007199254740993\n";
    assert_eq!(
        ok(&["--to", "json", "-m"], t),
        "{\"max\":9223372036854775807,\"min\":-9223372036854775808,\"safe\":9007199254740993}\n"
    );
    assert_eq!(ok(&[], t), t);
    assert_eq!(
        ok(
            &["--from", "json"],
            "{\"max\":9223372036854775807,\"min\":-9223372036854775808,\"safe\":9007199254740993}"
        ),
        t
    );
}

#[test]
fn integers_beyond_i64_are_rejected_not_rounded() {
    let err = fail(&[], "a = 9223372036854775808\n", 1);
    assert!(err.starts_with("invalid: 1:5 "), "{err}");
    let err = fail(&[], "a = 99999999999999999999\n", 1);
    assert_eq!(
        err,
        "invalid: 1:5 bilangan bulat di luar jangkauan 64-bit (i64): 99999999999999999999\n"
    );
    assert_eq!(
        fail(&["--from", "json"], "{\"a\":{\"id\":9223372036854775808}}", 1),
        "bilangan bulat di a.id di luar jangkauan TOML (i64): 9223372036854775808\n"
    );
    assert_eq!(
        fail(&["--from", "json"], "{\"a\":[-9223372036854775809]}", 1),
        "bilangan bulat di a[0] di luar jangkauan TOML (i64): -9223372036854775809\n"
    );
}

#[test]
fn non_finite_floats_cannot_become_json() {
    assert_eq!(ok(&[], "a = nan\nb = -inf\n"), "a = nan\nb = -inf\n");
    assert_eq!(
        fail(&["--to", "json"], "[x]\ny = [1.0, inf]\n", 1),
        "inf di x.y[1] tak bisa jadi JSON\n"
    );
    assert_eq!(fail(&["--to", "json"], "a = nan\n", 1), "NaN di a tak bisa jadi JSON\n");
}

#[test]
fn query_selects_values() {
    assert_eq!(ok(&["-q", "title"], SAMPLE), "\"TOML\"\n");
    assert_eq!(ok(&["-q", "title", "-r"], SAMPLE), "TOML\n");
    assert_eq!(ok(&["-q", "$.owner.tags"], SAMPLE), "[\"a\", \"b\"]\n");
    assert_eq!(ok(&["-q", "owner.tags[1]", "--to", "json"], SAMPLE), "\"b\"\n");
    assert_eq!(ok(&["-q", "products[1]"], SAMPLE), "name = \"Nail\"\n");
    assert_eq!(
        ok(&["-q", "owner", "-m", "--to", "json"], SAMPLE),
        "{\"name\":\"Tom\",\"tags\":[\"a\",\"b\"],\"address\":{\"city\":\"X\"}}\n"
    );
    assert_eq!(ok(&["-q", "big"], SAMPLE), "9223372036854775807\n");
    assert_eq!(ok(&["-q", "day"], SAMPLE), "1979-05-27\n");
    assert_eq!(ok(&["-q", "day", "--to", "json"], SAMPLE), "\"1979-05-27\"\n");
    assert_eq!(ok(&["-q", "[\"k.dot\"].x"], "[\"k.dot\"]\nx = 1\n"), "1\n");
    assert_eq!(ok(&["-q", "$", "-m", "--to", "json"], "a = 1\n"), "{\"a\":1}\n");
    assert_eq!(ok(&["-q", "a", "-s"], "[a]\nz = 1\nb = 2\n"), "b = 2\nz = 1\n");
    // Query on JSON input happens before conversion.
    assert_eq!(
        ok(&["--from", "json", "-q", "a.b"], "{\"a\":{\"b\":{\"c\":2}}}"),
        "c = 2\n"
    );
    assert_eq!(ok(&["--from", "json", "-q", "a", "-r"], "{\"a\":\"txt\"}"), "txt\n");
    assert_eq!(
        ok(&["--from", "json", "-q", "a"], "{\"a\":[1,{\"b\":2}]}"),
        "[1, { b = 2 }]\n"
    );
}

#[test]
fn query_errors() {
    assert_eq!(fail(&["-q", "nope"], SAMPLE, 1), "path tidak ditemukan: nope\n");
    assert_eq!(
        fail(&["-q", "products[2]"], SAMPLE, 1),
        "path tidak ditemukan: products[2]\n"
    );
    assert_eq!(fail(&["-q", "title.x"], SAMPLE, 1), "path tidak ditemukan: title.x\n");
    assert_eq!(fail(&["-q", "owner[0]"], SAMPLE, 1), "path tidak ditemukan: owner[0]\n");
    assert_eq!(
        fail(&["--from", "json", "-q", "x"], "{\"a\":1}", 1),
        "path tidak ditemukan: x\n"
    );
    // A selected null cannot be written as TOML; the path names it.
    assert_eq!(
        fail(&["--from", "json", "-q", "a.b"], "{\"a\":{\"b\":[null]}}", 1),
        "TOML tidak punya null: nilai null di a.b[0]\n"
    );
    let err = fail(&["-q", "a["], "a = 1\n", 2);
    assert!(err.starts_with("Path query tidak valid"), "{err}");
}

#[test]
fn input_from_file_stdin_and_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.toml");
    std::fs::write(&file, "\u{feff}a   = 1\n[b]\nc = 2\n").unwrap();
    let path = file.to_str().unwrap();
    assert_eq!(ok(&["-f", path], ""), "a = 1\n\n[b]\nc = 2\n");
    assert_eq!(
        ok(&["--file", path, "--to", "json", "-m"], "ignored stdin"),
        "{\"a\":1,\"b\":{\"c\":2}}\n"
    );
    assert_eq!(ok(&["--validate", "--file", path], ""), "valid\n");
    assert_eq!(ok(&[], "x   = 'y'\n"), "x = \"y\"\n");

    let err = fail(&["a = 1", "-f", path], "", 2);
    assert!(err.starts_with("Gunakan teks atau --file, bukan keduanya."), "{err}");
    let missing = dir.path().join("none.toml");
    let err = fail(&["-f", missing.to_str().unwrap()], "", 1);
    assert!(err.starts_with("ENOENT: no such file or directory, open "), "{err}");
    std::fs::write(&file, [b'a', b' ', b'=', b' ', b'"', 0xff, b'"', b'\n']).unwrap();
    let err = fail(&["-f", path], "", 1);
    assert_eq!(err, "invalid: input bukan UTF-8 yang valid (byte ke-6)\n");
}

#[test]
fn usage_errors_exit_2() {
    for (args, message) in [
        (vec!["--to", "yaml"], "Nilai --to tidak valid: yaml. Pilihan: json"),
        (vec!["--from", "yaml"], "Nilai --from tidak valid: yaml. Pilihan: json"),
        (
            vec!["--to", "json", "--from", "json"],
            "--to dan --from tidak bisa bersamaan.",
        ),
        (vec!["-m"], "--minify hanya berlaku dengan --to json."),
        (vec!["-r"], "--raw hanya berlaku dengan --query."),
        (vec!["--validate", "--to", "json"], "--validate tidak bisa dengan --to."),
        (
            vec!["--validate", "--from", "json"],
            "--validate tidak bisa dengan --from.",
        ),
        (vec!["--validate", "-q", "a"], "--validate tidak bisa dengan --query."),
        (vec!["--validate", "-s"], "--validate tidak bisa dengan --sort-keys."),
    ] {
        let err = fail(&args, "a = 1\n", 2);
        assert!(err.starts_with(message), "{args:?}: {err}");
    }
    // Multi-document streams are a YAML concept.
    let err = fail(&["--all"], "a = 1\n", 2);
    assert!(err.contains("--all"), "{err}");
}

#[test]
fn help_lists_options() {
    let r = dt(&["toml", "--help"]);
    assert_eq!(r.code, 0);
    for needle in ["dt toml", "--validate", "--to json", "--from json", "--query", "Exit:"] {
        assert!(r.out.contains(needle), "missing {needle}");
    }
    assert_eq!(dt(&["toml", "-h"]).out, r.out);
}

#[test]
fn deeply_nested_json_does_not_crash() {
    let json = format!("{}1{}", "{\"a\":".repeat(1500), "}".repeat(1500));
    let r = toml(&["--from", "json"], &json);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.out.starts_with("[a.a.a.a"), "{}", r.out);
}
