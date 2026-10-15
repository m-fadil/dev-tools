mod common;
use common::{dt, dt_in};
use serde_json::{Value, json};

#[test]
fn groups_and_unmatched_groups() {
    let r = dt(&["regex", r"(\d+)-(?<word>[a-z]+)?", "12-ab 7-"]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(
        r.out,
        "#1 index=0 \"12-ab\"\n  $1 \"12\"\n  $2 \"ab\"\n  <word> \"ab\"\n#2 index=6 \"7-\"\n  $1 \"7\"\n  $2 -\n  <word> -\n"
    );
}
#[test]
fn verbatim_stdin_first_and_byte_offsets() {
    assert_eq!(
        dt_in(&["regex", "-f", "i", "--first", "A"], b"bab\nbAb\n").out,
        "#1 index=1 \"a\"\n"
    );
    assert_eq!(dt_in(&["regex", "$"], b"x\n").out, "#1 index=2 \"\"\n");
    for (pattern, text, expected) in [("x*", "ab", vec![0, 1, 2]), ("", "😀a", vec![0, 4, 5])] {
        let r = dt(&["regex", "--json", pattern, text]);
        assert_eq!(r.code, 0, "{}", r.err);
        let v: Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(
            v["matches"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["index"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
#[test]
fn rust_replacement_and_first() {
    assert!(
        dt(&["regex", "-r", "${b}-$1", r"(?<b>\d)(x)", "1x 2x"])
            .out
            .ends_with("Hasil ganti:\n1-1 2-2\n")
    );
    let r = dt(&["regex", "--json", "--first", "-r", "[$0]", r"\d", "1 2 3"]);
    let v: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(v["replaced"], "[1] 2 3");
    assert!(
        dt_in(&["regex", "-r", "$0!", r"\."], b"a.b\n")
            .out
            .ends_with("Hasil ganti:\na.!b\n\n")
    );
    assert!(dt(&["regex", "-r", "$$", "a", "a"]).out.ends_with("Hasil ganti:\n$\n"));
}
#[test]
fn json_shape_and_sorted_unique_flags() {
    let r = dt(&["regex", "--json", "-f", "ii", "(a)|(?<n>b)", "Ab"]);
    assert_eq!(
        serde_json::from_str::<Value>(&r.out).unwrap(),
        json!({"pattern":"(a)|(?<n>b)","flags":"i","matches":[{"index":0,"match":"A","groups":["A",null],"named":{"n":null}},{"index":1,"match":"b","groups":[null,"b"],"named":{"n":"b"}}]})
    );
    let r = dt(&["regex", "--json", "-f", "xsmiix", "z", "a"]);
    let v: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(v["flags"], "imsx");
    assert!(v.get("replaced").is_none());
}
#[test]
fn test_mode_and_usage_errors() {
    let yes = dt(&["regex", "--test", "b", "abc"]);
    assert_eq!((yes.code, yes.out.as_str(), yes.err.as_str()), (0, "", ""));
    let no = dt(&["regex", "--test", "z", "abc"]);
    assert_eq!((no.code, no.out.as_str(), no.err.as_str()), (1, "", ""));
    for p in ["(", "(?=a)a", r"(a)\1"] {
        let r = dt(&["regex", p, "x"]);
        assert_eq!(r.code, 2);
        assert!(r.err.contains("Regex tidak valid:"));
    }
    for f in ["q", "g", "u", "y"] {
        let r = dt(&["regex", "-f", f, "a", "a"]);
        assert_eq!(r.code, 2);
        assert!(r.err.contains(&format!("Flag tidak dikenal: {f}. Pilihan: imsx")));
    }
    for args in [
        vec!["regex"],
        vec!["regex", "--timeout", "0", "a", "a"],
        vec!["regex", "--test", "--json", "a", "a"],
        vec!["regex", "--test", "-r", "x", "a", "a"],
    ] {
        assert_eq!(dt(&args).code, 2);
    }
    assert_eq!(
        dt(&["regex", "--test", "^(a+)+$", &format!("{}!", "a".repeat(4000))]).code,
        1
    );
}
#[test]
fn flags_multiline_dotall_extended_and_files() {
    assert_eq!(dt_in(&["regex", "--test", "-f", "m", "^b$"], b"a\nb\nc").code, 0);
    assert_eq!(dt_in(&["regex", "--test", "-f", "s", "a.b"], b"a\nb").code, 0);
    assert_eq!(dt(&["regex", "--test", "-f", "x", "a # comment\n b", "ab"]).code, 0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input");
    std::fs::write(&path, "abc").unwrap();
    assert_eq!(dt(&["regex", "--test", "--file", path.to_str().unwrap(), "b"]).code, 0);
    assert_eq!(dt(&["regex", "--file", path.to_str().unwrap(), "b", "abc"]).code, 2);
}
