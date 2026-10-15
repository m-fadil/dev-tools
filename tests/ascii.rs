mod common;
use common::dt;
use serde_json::{Value, json};

fn ascii(args: &[&str]) -> common::Out {
    let mut v = vec!["ascii"];
    v.extend_from_slice(args);
    dt(&v)
}

fn json_of(args: &[&str]) -> Value {
    serde_json::from_str(&ascii(args).out).unwrap()
}

#[test]
fn lookup_by_char_decimal_hex() {
    for v in ["A", "65", "0x41"] {
        assert_eq!(
            json_of(&["--json", v]),
            json!({
                "decimal": 65, "hex": "0x41", "octal": "101", "binary": "01000001", "unicode": "U+0041",
                "utf8": ["41"], "ascii": true, "character": "A", "name": null, "description": null,
            })
        );
    }
    assert_eq!(
        ascii(&["65"]).out,
        "desimal   65\nhex       0x41\noktal     101\nbiner     01000001\nkarakter  A\n"
    );
}

#[test]
fn control_characters() {
    let lf = json_of(&["--json", "0x0a"]);
    assert_eq!(lf["name"], "LF");
    assert_eq!(lf["character"], Value::Null);
    assert!(lf["description"].as_str().unwrap().contains("Line Feed"));
    assert_eq!(json_of(&["--json", "0x00"])["name"], "NUL");
    assert_eq!(json_of(&["--json", "127"])["name"], "DEL");
    assert_eq!(json_of(&["--json", " "])["name"], "SP");
    assert!(ascii(&["10"]).out.contains("nama      LF"));
}

#[test]
fn non_ascii() {
    let e = json_of(&["--json", "é"]);
    assert_eq!(e["decimal"], 233);
    assert_eq!(e["unicode"], "U+00E9");
    assert_eq!(e["utf8"], json!(["C3", "A9"]));
    assert_eq!(e["ascii"], false);
    assert_eq!(json_of(&["--json", "😀"])["utf8"], json!(["F0", "9F", "98", "80"]));
    assert!(
        ascii(&["0x20AC"])
            .out
            .contains("unicode   U+20AC\nutf-8     E2 82 AC\nkarakter  €")
    );
    assert_eq!(json_of(&["--json", "0x85"])["character"], Value::Null);
}

#[test]
fn table() {
    let r = ascii(&[]);
    assert_eq!(r.code, 0);
    let rows: Vec<&str> = r.out.trim_end().split('\n').collect();
    assert_eq!(rows.len(), 34);
    assert_eq!(
        rows[2],
        "  0 00 000 NUL |  32 20 040 SP  |  64 40 100 @   |  96 60 140 `"
    );
    let t = json_of(&["--json"]);
    assert_eq!(t["table"].as_array().unwrap().len(), 128);
    assert_eq!(t["table"][127]["name"], "DEL");
    assert_eq!(ascii(&["--table"]).out, r.out);
}

#[test]
fn errors() {
    for args in [
        vec!["abc"],
        vec!["0x110000"],
        vec!["0xD800"],
        vec!["1", "2"],
        vec!["--table", "A"],
        vec!["99999999999999999999"],
    ] {
        assert_ne!(ascii(&args).code, 0, "{args:?}");
    }
    assert_eq!(ascii(&["1", "2"]).code, 2);
    assert_eq!(ascii(&["--table", "A"]).code, 2);
    assert_eq!(ascii(&["abc"]).code, 1);
}
