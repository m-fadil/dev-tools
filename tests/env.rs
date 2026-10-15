mod common;
use common::dt;
use serde_json::{Value, json};
use tempfile::TempDir;
fn files(a: &str, b: &str) -> (TempDir, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let pa = dir.path().join("a.env");
    let pb = dir.path().join("b.env");
    std::fs::write(&pa, a).unwrap();
    std::fs::write(&pb, b).unwrap();
    (
        dir,
        pa.to_string_lossy().into_owned(),
        pb.to_string_lossy().into_owned(),
    )
}
#[test]
fn identical_and_crlf() {
    for (a, b) in [("A=1\nB=2\n", "B=2\nA=1"), ("A=1\r\nB=2\r\n", "A=1\nB=2\n")] {
        let (_d, a, b) = files(a, b);
        let r = dt(&["env", &a, &b]);
        assert_eq!(r.code, 0);
        assert_eq!(r.out, "2 sama, 0 beda, 0 hanya a, 0 hanya b\n");
    }
}
#[test]
fn sorted_masked_diff() {
    let (_d, a, b) = files(
        "Z=1\nA=1\nSHARED=rahasia-a\nSAME=x\n",
        "Y=1\nB=1\nSHARED=rahasia-b\nSAME=x\n",
    );
    let r = dt(&["env", &a, &b]);
    assert_eq!(r.code, 1);
    assert_eq!(
        r.out,
        "Hanya di a:\n  A\n  Z\nHanya di b:\n  B\n  Y\nBeda nilai:\n  SHARED\n1 sama, 1 beda, 2 hanya a, 2 hanya b\n"
    );
    assert!(!format!("{}{}", r.out, r.err).contains("rahasia"));
}
#[test]
fn show_and_keys_only() {
    let (_d, a, b) = files("K=satu\nL=1", "K=dua\nL=1");
    let r = dt(&["env", "--show-values", &a, &b]);
    assert_eq!(r.code, 1);
    assert!(r.out.contains("K: satu -> dua"));
    let r = dt(&["env", "-k", &a, &b]);
    assert_eq!(r.code, 0);
    assert_eq!(r.out.trim(), "2 sama, 0 beda, 0 hanya a, 0 hanya b");
    let (_d, a, b) = files("K=1", "M=1");
    assert_eq!(dt(&["env", "-k", &a, &b]).code, 1);
}
#[test]
fn parser_export_comments_quotes_escapes_no_interpolation() {
    let (_d, a, b) = files(
        "# komentar\n\nexport A=1\nB=\"x # y\"\nC='lit\\n#z'\nD=plain # trailing\nE=\"baris1\\nbaris2 \\\"q\\\"\"\nF=${A}\nG=#fff\nH=",
        "A=1\nB=x # y\nC=lit\\n#z\nD=plain\nE=\"baris1\\nbaris2 \\\"q\\\"\" # c\nF=${A}\nG=#fff\nH=",
    );
    let r = dt(&["env", "-s", &a, &b]);
    assert_eq!(
        r.out,
        "Beda nilai:\n  B: x # y -> x\n7 sama, 1 beda, 0 hanya a, 0 hanya b\n"
    );
}
#[test]
fn duplicate_last_wins() {
    let (_d, a, b) = files("K=1\nK=2\n", "K=2\n");
    let r = dt(&["env", "-s", &a, &b]);
    assert_eq!(r.code, 0);
    assert!(r.err.starts_with("a:2: peringatan: K duplikat (baris 1)"));
    assert_eq!(r.out.trim(), "1 sama, 0 beda, 0 hanya a, 0 hanya b");
}
#[test]
fn invalid_lines_and_usage() {
    let (_d, a, b) = files("A=1\nbukan valid\nB=\"belum ditutup\n", "A=1\n");
    let r = dt(&["env", &a, &b]);
    assert_eq!(r.code, 1);
    for s in [
        "a:2: baris tidak valid",
        "a:3: baris tidak valid",
        "2 baris tidak valid",
    ] {
        assert!(r.err.contains(s));
    }
    assert_eq!(dt(&["env", &a]).code, 2);
    assert_eq!(dt(&["env"]).code, 2);
    assert_eq!(dt(&["env", &a, &b, &b]).code, 2);
    let r = dt(&["env", &format!("{a}.missing"), &b]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Tidak bisa membaca"));
}
#[test]
fn json_masks_values() {
    let (_d, a, b) = files("A=1\nB=2\nC=3\nS=s\n", "A=1\nB=9\nD=4\nS=s\n");
    let r = dt(&["env", "--json", &a, &b]);
    assert_eq!(r.code, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&r.out).unwrap(),
        json!({"onlyA":["C"],"onlyB":["D"],"different":[{"key":"B"}],"same":2,"invalidLines":[]})
    );
}
#[test]
fn json_values_invalid_lines_and_identical() {
    let (_d, a, b) = files("A=1\nbad line\n", "A=2\n");
    let r = dt(&["env", "--json", "-s", &a, &b]);
    assert_eq!(r.code, 1);
    let v: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(v["different"], json!([{"key":"A","a":"1","b":"2"}]));
    assert_eq!(v["invalidLines"], json!([{"file":a,"line":2}]));
    let (_d, a, b) = files("A=1\n", "A=1\n");
    let r = dt(&["env", "--json", &a, &b]);
    assert_eq!(r.code, 0);
    assert_eq!(serde_json::from_str::<Value>(&r.out).unwrap()["same"], 1);
}

#[test]
fn export_as_key_and_mid_line_separators_invalid() {
    let (_d, a, b) = files("export = 1\nexport  =1\nA=1\rB=2\nC=x\u{2028}y\n", "export=1\n");
    let r = dt(&["env", &a, &b]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("2 baris tidak valid"), "{}", r.err);
    for s in ["a:3:", "a:4:"] {
        assert!(r.err.contains(s), "{}", r.err);
    }
    assert!(
        !r.err.contains("a:1: baris") && !r.err.contains("a:2: baris"),
        "{}",
        r.err
    );
}
