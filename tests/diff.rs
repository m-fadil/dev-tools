mod common;

use common::dt;
use serde_json::{Value, json};

struct Files {
    _dir: tempfile::TempDir,
    a: String,
    b: String,
}

impl Files {
    fn new(a: impl AsRef<[u8]>, b: impl AsRef<[u8]>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pa = dir.path().join("a");
        let pb = dir.path().join("b");
        std::fs::write(&pa, a).unwrap();
        std::fs::write(&pb, b).unwrap();
        Self {
            a: pa.to_str().unwrap().into(),
            b: pb.to_str().unwrap().into(),
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> common::Out {
        let mut all = vec!["diff"];
        all.extend_from_slice(args);
        all.extend([self.a.as_str(), self.b.as_str()]);
        dt(&all)
    }
}

fn hunks(out: &str) -> &str {
    &out[out.find("@@").unwrap()..]
}

#[test]
fn identical_files_known_sha256_and_empty_files() {
    let r = Files::new("abc", "abc").run(&[]);
    assert_eq!(r.code, 0);
    assert_eq!(
        r.out.trim(),
        "Identik (sha256 ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad, 3 byte)"
    );
    let r = Files::new("", "").run(&[]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("Identik (sha256 e3b0c44298fc1c14"));
    assert!(r.out.ends_with(", 0 byte)\n"));
}

#[test]
fn minimal_unified_diff_and_summary() {
    let files = Files::new("satu\ndua\ntiga\n", "satu\nDUA\ntiga\n");
    let r = files.run(&[]);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "");
    assert!(r.out.contains(&format!("a: {}  14 byte  sha256 ", files.a)));
    assert!(r.out.contains(&format!("--- {}\n+++ {}\n", files.a, files.b)));
    assert_eq!(hunks(&r.out), "@@ -1,3 +1,3 @@\n satu\n-dua\n+DUA\n tiga\n");
}

#[test]
fn insertions_deletions_use_lcs() {
    let r = Files::new("a\nb\nc\nd\ne\n", "b\nc\nx\nd\ne\nf\n").run(&["-U", "0"]);
    assert_eq!(
        hunks(&r.out),
        "@@ -1,1 +0,0 @@\n-a\n@@ -3,0 +3,1 @@\n+x\n@@ -5,0 +6,1 @@\n+f\n"
    );
}

#[test]
fn context_splits_and_merges_hunks() {
    let lines: Vec<String> = (1..=20).map(|i| format!("l{i}")).collect();
    let mut changed = lines.clone();
    changed[1] = "X2".into();
    changed[17] = "X18".into();
    let files = Files::new(format!("{}\n", lines.join("\n")), format!("{}\n", changed.join("\n")));
    let small = files.run(&["-U", "1"]);
    let small = hunks(&small.out);
    assert_eq!(small.lines().filter(|l| l.starts_with("@@")).count(), 2);
    assert!(small.contains("@@ -1,3 +1,3 @@\n"));
    assert!(small.contains("@@ -17,3 +17,3 @@\n"));
    let wide = files.run(&["-U", "20"]);
    let wide = hunks(&wide.out);
    assert_eq!(wide.lines().filter(|l| l.starts_with("@@")).count(), 1);
    assert!(wide.starts_with("@@ -1,20 +1,20 @@\n"));
}

#[test]
fn missing_trailing_newline_and_empty_file_insertions() {
    let r = Files::new("x\ny", "x\ny\n").run(&[]);
    assert_eq!(r.code, 1);
    assert_eq!(
        hunks(&r.out),
        "@@ -1,2 +1,2 @@\n x\n-y\n\\ Tidak ada newline di akhir file\n+y\n"
    );
    let r = Files::new("", "baru\n").run(&[]);
    assert_eq!(r.code, 1);
    assert_eq!(hunks(&r.out), "@@ -0,0 +1,1 @@\n+baru\n");
}

#[test]
fn binary_first_byte_offsets_prefixes_and_multiple_blocks() {
    let r = Files::new([1, 2, 0, 4, 5], [1, 2, 0, 9, 5]).run(&[]);
    assert_eq!(r.code, 1);
    assert!(r.out.ends_with("Biner, berbeda di byte offset 3\n"));
    assert!(!r.out.contains("@@"));
    assert!(
        Files::new([0, 1, 2], [0, 1, 2, 3])
            .run(&[])
            .out
            .ends_with("byte offset 3\n")
    );
    let mut base = vec![7u8; 200_000];
    base[0] = 0;
    let mut other = base.clone();
    other[150_000] = 8;
    assert!(Files::new(base, other).run(&[]).out.ends_with("byte offset 150000\n"));
}

#[test]
fn over_five_mebibytes_and_quadratic_guard_produce_summary_only() {
    let large = Files::new(vec![b'a'; 5 * 1024 * 1024 + 1], vec![b'b'; 5 * 1024 * 1024 + 1]);
    let many = Files::new("a\n".repeat(6000), "b\n".repeat(6000));
    for files in [large, many] {
        let r = files.run(&[]);
        assert_eq!(r.code, 1);
        assert!(r.out.contains("git diff --no-index"));
        assert!(!r.out.contains("@@"));
        let r = files.run(&["--json"]);
        assert_eq!(r.code, 1);
        let doc: Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(doc["kind"], "summary-only");
        assert!(doc.get("hunks").is_none());
    }
}

#[test]
fn missing_file_and_directory_errors_name_path_and_have_no_stdout() {
    let files = Files::new("x", "y");
    let missing = format!("{}.missing", files.a);
    let r = dt(&["diff", &files.a, &missing]);
    assert_eq!(r.code, 1);
    assert_eq!(r.out, "");
    assert!(r.err.contains(&missing));
    assert!(r.err.contains("File tidak ditemukan:"));
    let r = dt(&["diff", "--json", &missing, &format!("{missing}2")]);
    assert_eq!(r.code, 1);
    assert_eq!(r.out, "");
    let directory = files._dir.path().to_str().unwrap();
    let r = dt(&["diff", directory, &files.a]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Bukan file (direktori):"));
}

#[test]
fn operand_count_and_context_usage_errors() {
    let f = Files::new("x", "x");
    assert_eq!(dt(&["diff", &f.a]).code, 2);
    assert_eq!(dt(&["diff", &f.a, &f.a, &f.a]).code, 2);
    let r = f.run(&["-U", "21"]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("--context"));
    assert_eq!(f.run(&["-U", "x"]).code, 2);
}

#[test]
fn json_identical_has_summaries_without_hunks() {
    let r = Files::new("x\n", "x\n").run(&["--json"]);
    assert_eq!(r.code, 0);
    let doc: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(doc["identical"], true);
    assert_eq!(doc["kind"], "identical");
    assert_eq!(doc["a"]["size"], 2);
    assert_eq!(doc["a"]["sha256"], doc["b"]["sha256"]);
    assert!(doc.get("hunks").is_none());
}

#[test]
fn json_text_and_binary_difference_shapes() {
    let r = Files::new("a\nb\nc\n", "a\nB\nc\n").run(&["--json", "-U", "0"]);
    assert_eq!(r.code, 1);
    let doc: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(doc["kind"], "text");
    assert_eq!(doc["identical"], false);
    assert_eq!(
        doc["hunks"],
        json!([{ "aStart": 2, "aLines": 1, "bStart": 2, "bLines": 1, "lines": [
        { "op": "-", "text": "b" }, { "op": "+", "text": "B" }
    ] }])
    );
    let r = Files::new([1, 0, 2], [1, 0, 3]).run(&["--json"]);
    assert_eq!(r.code, 1);
    let doc: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(doc["kind"], "binary");
    assert_eq!(doc["firstDifferingByte"], 2);
}

#[test]
fn invalid_utf8_stays_byte_distinct_and_cr_is_line_content() {
    let r = Files::new([0xff, b'\n'], [0xfe, b'\n']).run(&["--json"]);
    let doc: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(
        doc["hunks"][0]["lines"],
        json!([{ "op": "-", "text": "�" }, { "op": "+", "text": "�" }])
    );
    let r = Files::new("a\r\n", "a\n").run(&[]);
    assert_eq!(hunks(&r.out), "@@ -1,1 +1,1 @@\n-a\r\n+a\n");
}
