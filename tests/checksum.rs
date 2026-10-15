mod common;
use common::{Out, dt};
use serde_json::{Value, json};
use std::path::Path;
const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const MD5: &str = "900150983cd24fb0d6963f7d28e17f72";
fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("sub")).unwrap();
    for (p, s) in [("abc.txt", "abc"), ("empty.txt", ""), ("sub/spasi nama.txt", "abc")] {
        std::fs::write(d.path().join(p), s).unwrap();
    }
    d
}
fn check(d: &Path, text: &str, options: &[&str]) -> Out {
    let path = d.join("list.sums");
    std::fs::write(&path, text).unwrap();
    let mut args = vec!["checksum", "-c", path.to_str().unwrap()];
    args.extend_from_slice(options);
    dt(&args)
}
fn doc(r: &Out) -> Value {
    serde_json::from_str(&r.out).unwrap()
}
#[test]
fn known_vectors_argument_order_and_json() {
    let d = fixture();
    let a = d.path().join("abc.txt");
    let e = d.path().join("empty.txt");
    let (a, e) = (a.to_str().unwrap(), e.to_str().unwrap());
    let r = dt(&["checksum", a, e]);
    assert_eq!(r.code, 0);
    assert_eq!(r.out, format!("{ABC}  {a}\n{EMPTY}  {e}\n"));
    assert_eq!(dt(&["checksum", "-a", "md5", a]).out, format!("{MD5}  {a}\n"));
    assert_eq!(
        doc(&dt(&["checksum", "--json", "-a", "md5", a])),
        json!({"algorithm":"md5","files":[{"path":a,"digest":MD5}],"errors":[]})
    );
}
#[test]
fn invalid_arguments_and_file_errors() {
    let d = fixture();
    let sub = d.path().join("sub");
    let missing = d.path().join("nope.txt");
    assert_eq!(dt(&["checksum"]).code, 2);
    assert_eq!(dt(&["checksum", "-a", "rot13", "x"]).code, 2);
    for p in [&sub, &missing] {
        let r = dt(&["checksum", p.to_str().unwrap()]);
        assert_eq!(r.code, 1);
        assert!(r.err.contains(p.to_str().unwrap()));
    }
    assert_eq!(dt(&["checksum", "-c", "x", "abc.txt"]).code, 2);
    let r = dt(&["checksum", "-c", missing.to_str().unwrap()]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("nope.txt: tidak dapat dibaca ("));
    let r = dt(&["checksum", "--json", "-c", sub.to_str().unwrap()]);
    assert_eq!(r.code, 1);
    assert!(r.out.is_empty());
    assert!(r.err.contains("tidak dapat dibaca ("));
}
#[test]
fn all_ok_comments_binary_uppercase_spaces_crlf_and_no_newline() {
    let d = fixture();
    let r = check(
        d.path(),
        &format!(
            "# komentar\n\n{ABC}  abc.txt\n{} *abc.txt\n{EMPTY}  empty.txt\r\n{ABC}  sub/spasi nama.txt\n",
            ABC.to_uppercase()
        ),
        &[],
    );
    assert_eq!(r.code, 0);
    assert_eq!(
        r.out,
        "abc.txt: OK\nabc.txt: OK\nempty.txt: OK\nsub/spasi nama.txt: OK\n"
    );
    assert!(
        r.err
            .contains("4 OK, 0 GAGAL, 0 TIDAK ADA, 0 ERROR, 0 baris tidak valid")
    );
    assert_eq!(check(d.path(), &format!("{ABC}  abc.txt"), &[]).out, "abc.txt: OK\n");
    for s in ["", "# saja\n"] {
        assert_eq!(check(d.path(), s, &[]).code, 1);
    }
    assert_eq!(check(d.path(), &format!("{MD5}  abc.txt\n"), &[]).code, 1);
    assert_eq!(
        check(d.path(), &format!("{MD5}  abc.txt\n"), &["-a", "md5"]).out,
        "abc.txt: OK\n"
    );
}
#[test]
fn mismatch_missing_malformed_and_json_statuses() {
    let d = fixture();
    let wrong = format!("0{}", &ABC[1..]);
    let r = check(
        d.path(),
        &format!("{wrong}  abc.txt\n{ABC}  hilang.txt\nbukan-hex  abc.txt\n{MD5}  abc.txt\n{ABC}  empty.txt\n"),
        &[],
    );
    assert_eq!(r.code, 1);
    assert_eq!(r.out, "abc.txt: GAGAL\nhilang.txt: TIDAK ADA\nempty.txt: GAGAL\n");
    for s in [
        "baris 3 tidak valid",
        "baris 4 tidak valid",
        "2 GAGAL",
        "1 TIDAK ADA, 0 ERROR, 2 baris tidak valid",
    ] {
        assert!(r.err.contains(s));
    }
    let r = check(
        d.path(),
        &format!("{ABC}  abc.txt\n{wrong}  empty.txt\n{ABC}  hilang.txt\nxx  abc.txt\n"),
        &["--json"],
    );
    assert_eq!(r.code, 1);
    assert_eq!(
        doc(&r),
        json!({"algorithm":"sha256","results":[{"path":"abc.txt","status":"ok"},{"path":"empty.txt","status":"failed"},{"path":"hilang.txt","status":"missing"}],"malformed":[4],"ok":1,"failed":1,"missing":1,"errors":0,"passed":false})
    );
    let r = check(d.path(), &format!("{ABC}  abc.txt\n"), &["--json"]);
    assert_eq!(r.code, 0);
    assert_eq!(doc(&r)["passed"], true);
}
// "abc.txt/x" is ENOTDIR (an error) on Unix but a plain "not found" on Windows.
#[cfg(unix)]
#[test]
fn error_entries_do_not_hide_other_results() {
    let d = fixture();
    let text = format!("{ABC}  sub\n{ABC}  abc.txt/x\n{ABC}  abc.txt\n{ABC}  empty.txt\n");
    let r = check(d.path(), &text, &[]);
    assert_eq!(r.code, 1);
    for s in [
        "sub: ERROR (",
        "abc.txt/x: ERROR (",
        "abc.txt: OK\n",
        "empty.txt: GAGAL\n",
    ] {
        assert!(r.out.contains(s), "{}", r.out);
    }
    assert!(r.err.contains("2 ERROR"));
    let r = check(d.path(), &text, &["--json"]);
    let v = doc(&r);
    assert_eq!(r.code, 1);
    assert_eq!(v["errors"], 2);
    assert_eq!(v["ok"], 1);
    assert_eq!(v["failed"], 1);
    assert_eq!(v["passed"], false);
    for entry in &v["results"].as_array().unwrap()[..2] {
        assert_eq!(entry["status"], "error");
        assert!(entry["error"].is_string());
    }
    let a = d.path().join("abc.txt");
    let sub = d.path().join("sub");
    let e = d.path().join("empty.txt");
    let (a, sub, e) = (a.to_str().unwrap(), sub.to_str().unwrap(), e.to_str().unwrap());
    let r = dt(&["checksum", a, sub, e]);
    assert_eq!(r.code, 1);
    assert_eq!(r.out, format!("{ABC}  {a}\n{EMPTY}  {e}\n"));
    assert!(r.err.contains(sub));
    let r = dt(&["checksum", "--json", a, sub, e]);
    assert_eq!(r.code, 1);
    assert_eq!(
        doc(&r),
        json!({"algorithm":"sha256","files":[{"path":a,"digest":ABC},{"path":e,"digest":EMPTY}],"errors":[{"path":sub,"message":"direktori"}]})
    );
}
#[cfg(unix)]
#[test]
fn nonregular_files_are_rejected() {
    if Path::new("/dev/zero").exists() {
        let r = dt(&["checksum", "/dev/zero"]);
        assert_eq!(r.code, 1);
        assert!(r.err.contains("bukan file biasa"));
    }
}
