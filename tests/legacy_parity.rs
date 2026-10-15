mod common;
use common::{dt, dt_in};

#[test]
fn repeated_boolean_flags_are_accepted() {
    for args in [
        vec!["case", "--json", "--json", "ab"],
        vec!["lorem", "--classic", "--classic", "-w", "3", "--seed", "1"],
        vec!["hash", "--json", "--json", "abc"],
    ] {
        let r = dt(&args);
        assert_eq!(r.code, 0, "{args:?}: {}", r.err);
    }
}

#[test]
fn unreadable_file_reports_node_style_error() {
    let missing = dt(&["hash", "-f", "/nonexistent/dt-file"]);
    assert_eq!(missing.code, 1);
    assert_eq!(
        missing.err.trim(),
        "ENOENT: no such file or directory, open '/nonexistent/dt-file'"
    );
    let dir = tempfile::tempdir().unwrap();
    let r = dt(&["hash", "-f", dir.path().to_str().unwrap()]);
    assert_eq!(r.code, 1);
    assert_eq!(r.err.trim(), "EISDIR: illegal operation on a directory, read");
}

#[test]
fn decode_uses_javascript_whitespace() {
    // U+FEFF is JS whitespace; U+0085 is not.
    let bom = dt_in(&["decode"], "\u{feff}aGk=\n".as_bytes());
    assert_eq!((bom.code, bom.out.as_str()), (0, "hi"), "{}", bom.err);
    let nel = dt_in(&["decode"], "aG\u{85}k=".as_bytes());
    assert_eq!(nel.code, 1);
}
