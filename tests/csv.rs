mod common;

use common::{dt, dt_in};
use serde_json::{Value, json};

fn csv(args: &[&str], input: &str) -> common::Out {
    let mut all = vec!["csv"];
    all.extend_from_slice(args);
    dt_in(&all, input.as_bytes())
}

fn json_csv(args: &[&str], input: &str) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let r = csv(&all, input);
    assert_eq!(r.code, 0, "{}", r.err);
    serde_json::from_str(&r.out).unwrap()
}

#[test]
fn quotes_escaped_quotes_embedded_delimiters_newlines_crlf_bom() {
    assert_eq!(
        json_csv(
            &[],
            "\u{feff}name,note\r\n\"Smith, J\",\"say \"\"hi\"\"\nbye\"\r\nplain,\r\n"
        ),
        json!([
            { "name": "Smith, J", "note": "say \"hi\"\nbye" }, { "name": "plain", "note": "" }
        ])
    );
}

#[test]
fn strips_exactly_one_bom_with_and_without_detection() {
    for args in [vec![], vec!["--detect"]] {
        assert_eq!(
            json_csv(&args, "\u{feff}\u{feff}a,b\n1,2"),
            json!([{ "\u{feff}a": "1", "b": "2" }])
        );
    }
}

#[test]
fn trailing_delimiter_empty_field_and_blank_lines() {
    assert_eq!(
        json_csv(&["--no-header"], "a,\n\n,\nb,c"),
        json!([["a", ""], ["", ""], ["b", "c"]])
    );
    assert_eq!(json_csv(&[], "a\n\"\"\nx\n"), json!([{ "a": "" }, { "a": "x" }]));
    assert_eq!(json_csv(&["--no-header"], "a,"), json!([["a", ""]]));
    assert_eq!(json_csv(&[], ""), json!([]));
}

#[test]
fn malformed_quotes_report_record_number() {
    let r = csv(&[], "a,b\n1,2\n3,\"oops\n4,5\n");
    assert_eq!(r.code, 1);
    assert!(r.err.contains("tanda kutip tidak ditutup (mulai di baris 3)"));
    let r = csv(&[], "a,b\n\"x\"y,2\n");
    assert_eq!(r.code, 1);
    assert!(r.err.contains("baris 2"));
    assert!(csv(&[], "\"x\"😀").err.contains("\"\\ud83d\""));
}

#[test]
fn named_and_character_delimiters_and_detection() {
    for (d, input) in [("semicolon", "a;b\n1;2"), ("tab", "a\tb\n1\t2"), ("|", "a|b\n1|2")] {
        assert_eq!(json_csv(&["-d", d], input), json!([{ "a": "1", "b": "2" }]));
    }
    let r = csv(&["--json", "--detect"], "\"x,y,z\";b\n1;2\n");
    assert_eq!(
        serde_json::from_str::<Value>(&r.out).unwrap(),
        json!([{ "x,y,z": "1", "b": "2" }])
    );
    assert!(r.err.contains("Delimiter terdeteksi: ;"));
    for args in [
        vec!["-d", "ab"],
        vec!["-d", "\""],
        vec!["-d", "😀"],
        vec!["--detect", "-d", ","],
    ] {
        assert_eq!(csv(&args, "a").code, 2);
    }
    let tie = csv(&["--detect", "--json"], "a;b|c\nvalue");
    assert_eq!(tie.code, 0, "{}", tie.err);
    assert_eq!(
        serde_json::from_str::<Value>(&tie.out).unwrap(),
        json!([{ "a;b|c": "value" }])
    );
    assert!(tie.err.contains("Delimiter terdeteksi: ,"));
    assert_eq!(
        json_csv(&["--detect"], "5\" pipe,x\n1,2"),
        json!([{ "5\" pipe": "1", "x": "2" }])
    );
}

#[test]
fn table_alignment_and_truncation() {
    let r = csv(
        &["--max-width", "10"],
        "nama,kota\n日本,Tokyo\nab,\"a very long cell value that must be cut\"\n",
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(
        r.out,
        "nama | kota\n-----+-----------\n日本 | Tokyo\nab   | a very lo…\n"
    );
    assert_eq!(
        csv(&["--no-header"], "a,b\n1,2\n").out,
        "c1 | c2\n---+---\na  | b\n1  | 2\n"
    );
    assert_eq!(csv(&["--no-header", "-F", "csv"], "a,b\n1,2\n").out, "a,b\n1,2\n");
}

#[test]
fn controls_only_sanitized_in_tables() {
    let cell = "\x1b[2J\x1b]0;owned\x07\u{9b}31m\u{7f}\u{80}";
    let input = format!("a,b\n{cell},x");
    let table = csv(&["--max-width", "100"], &input);
    assert_eq!(table.code, 0, "{}", table.err);
    assert!(
        !table
            .out
            .chars()
            .any(|c| (c as u32 <= 0x1f && c != '\n') || (0x7f..=0x9f).contains(&(c as u32)))
    );
    assert!(table.out.contains("�[2J�]0;owned��31m��"));
    assert_eq!(json_csv(&[], &input), json!([{ "a": cell, "b": "x" }]));
    assert_eq!(csv(&["-F", "csv"], &input).out, format!("{input}\n"));
    assert_eq!(csv(&["-F", "tsv"], &input).out, format!("a\tb\n{cell}\tx\n"));
}

#[test]
fn tsv_escaping_and_csv_requoting() {
    assert_eq!(
        csv(&["-F", "tsv"], "a,b\n\"x\ty\",\"l1\nl2\"\n").out,
        "a\tb\nx\\ty\tl1\\nl2\n"
    );
    assert_eq!(
        csv(&["-F", "csv", "-d", "tab"], "a\tb\n\"1,2\"\t\"q\"\"r\"\n").out,
        "a,b\n\"1,2\",\"q\"\"r\"\n"
    );
}

#[test]
fn select_by_name_number_and_limit() {
    let input = "a,b,c\n1,2,3\n4,5,6\n";
    assert_eq!(
        json_csv(&["--select", "c,1", "--limit", "1"], input),
        json!([{ "c": "3", "a": "1" }])
    );
    let r = csv(&["--select", "zz"], input);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Kolom tidak ditemukan: zz"));
    assert_eq!(csv(&["--select", "4"], input).code, 1);
    assert_eq!(csv(&["--limit", "0"], input).code, 2);
    assert_eq!(csv(&["--select", "a,1"], input).code, 2);
    assert_eq!(csv(&["--select", "a,"], input).code, 2);
    assert_eq!(csv(&["--select", "\u{feff}"], input).code, 2);
    assert_eq!(json_csv(&["--select", " a "], " a ,b\nx,y"), json!([{ " a ": "x" }]));
}

#[test]
fn stats_count_full_selected_data_before_limit() {
    let input = "a,b\n1,\n,\n3,4\n";
    assert_eq!(
        json_csv(&["--stats"], input),
        json!({ "rows": 3, "columns": 2, "ragged": [], "columnStats": [
        { "index": 1, "name": "a", "empty": 1 }, { "index": 2, "name": "b", "empty": 2 }
    ] })
    );
    let text = csv(&["--stats"], input).out;
    assert!(text.contains("Baris data: 3"));
    assert!(text.contains("2 \"b\": 2"));
    assert_eq!(
        json_csv(&["--stats", "--select", "b", "--limit", "1"], input)["rows"],
        3
    );
}

#[test]
fn three_hundred_thousand_rows_work_in_all_formats() {
    let count = 300_000;
    let input = format!("a,b\n{}", "1,2\n".repeat(count));
    let stats = csv(&["--stats"], &input);
    assert_eq!(stats.code, 0, "{}", stats.err);
    assert!(stats.out.contains("Baris data: 300000"));
    assert!(stats.out.contains("Kolom: 2"));
    assert_eq!(json_csv(&["--stats"], &input)["rows"], count);
    let table = csv(&[], &input);
    assert_eq!(table.code, 0, "{}", table.err);
    assert_eq!(table.out.split('\n').count(), count + 3);
    assert!(table.out.ends_with("1 | 2\n"));
    let objects = json_csv(&[], &input);
    let data = objects.as_array().unwrap();
    assert_eq!(data.len(), count);
    assert_eq!(data[0], json!({ "a": "1", "b": "2" }));
    assert_eq!(data[count - 1], json!({ "a": "1", "b": "2" }));
}

#[test]
fn stats_handles_one_hundred_fifty_thousand_columns() {
    let input = (0..150_000).map(|i| format!("h{i}")).collect::<Vec<_>>().join(",");
    let stats = csv(&["--stats"], &input);
    assert_eq!(stats.code, 0, "{}", stats.err);
    assert!(stats.out.contains("Baris data: 0"));
    assert!(stats.out.contains("Kolom: 150000"));
    assert!(stats.out.ends_with("  150000 \"h149999\": 0\n"));
}

#[test]
fn format_conflicts_and_duplicate_headers() {
    assert_eq!(csv(&["--json", "-F", "tsv"], "a\n1").code, 2);
    assert_eq!(csv(&["--json", "-F", "json"], "a\n1").code, 0);
    let dup = csv(&["--json"], "a,a\n1,2");
    assert_eq!(dup.code, 1);
    assert!(dup.err.contains("Header duplikat \"a\" (kolom 1 dan 2)"));
    assert_eq!(csv(&["-F", "xml"], "a").code, 2);
}

#[test]
fn ragged_rows_warn_and_pad_and_generate_noncolliding_names() {
    let input = "a,b,c\n1,2\n1,2,3,4\nx,y,z\n";
    let r = csv(&["--json"], input);
    assert_eq!(r.code, 0);
    assert!(r.err.contains("baris 2 punya 2 kolom, diharapkan 3"));
    assert!(r.err.contains("baris 3 punya 4 kolom, diharapkan 3"));
    assert_eq!(
        serde_json::from_str::<Value>(&r.out).unwrap(),
        json!([
            { "a": "1", "b": "2", "c": "", "c4": "" }, { "a": "1", "b": "2", "c": "3", "c4": "4" }, { "a": "x", "b": "y", "c": "z", "c4": "" }
        ])
    );
    assert_eq!(
        json_csv(&[], "a,c3\n1,2,3,4"),
        json!([{ "a": "1", "c3": "2", "c4": "3", "c5": "4" }])
    );
    assert_eq!(
        json_csv(&[], "a,c3,c4\n1,2,3,4,5"),
        json!([{ "a": "1", "c3": "2", "c4": "3", "c5": "4", "c6": "5" }])
    );
    let strict = csv(&["--strict"], input);
    assert_eq!(strict.code, 1);
    assert!(strict.out.is_empty());
    assert!(strict.err.contains("Baris 2 punya 2 kolom, diharapkan 3"));
    let warnings = csv(&[], &format!("a,b\n{}", "x\n".repeat(23)));
    assert_eq!(warnings.err.lines().count(), 21);
    assert!(warnings.err.ends_with("peringatan: 3 baris tidak rata lainnya\n"));
}

#[test]
fn file_input_and_numeric_object_key_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.csv");
    std::fs::write(&path, "10,2,a,01,0\nx,y,z,q,p").unwrap();
    let r = dt(&["csv", "--json", "--file", path.to_str().unwrap()]);
    assert_eq!(r.code, 0, "{}", r.err);
    let keys = serde_json::from_str::<Value>(&r.out).unwrap()[0]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(keys, ["0", "2", "10", "a", "01"]);
    assert_eq!(dt(&["csv", "--file", path.to_str().unwrap(), "x"]).code, 2);
}
