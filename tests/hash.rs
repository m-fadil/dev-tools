mod common;
use common::{dt, dt_in};

fn trimmed(o: &common::Out) -> &str {
    o.out.trim()
}

#[test]
fn known_sha256_and_hmac_vectors_stdin_verbatim() {
    assert_eq!(
        trimmed(&dt(&["hash", "abc"])),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        trimmed(&dt_in(&["hash", "-a", "md5"], b"abc")),
        "900150983cd24fb0d6963f7d28e17f72"
    );
    // RFC 4231 case 2
    assert_eq!(
        trimmed(&dt(&["hash", "-k", "Jefe", "what do ya want for nothing?"])),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
    // stdin keeps its trailing newline: not the hash of "abc"
    assert_ne!(trimmed(&dt_in(&["hash"], b"abc\n")), trimmed(&dt(&["hash", "abc"])));
}

#[test]
fn all_algorithms_match_known_digests() {
    let cases = [
        ("sha1", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        ("sha224", "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7"),
        (
            "sha384",
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
        ),
        (
            "sha3-256",
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
        ),
        (
            "blake2b512",
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
        ),
    ];
    for (alg, digest) in cases {
        assert_eq!(trimmed(&dt(&["hash", "-a", alg, "abc"])), digest, "{alg}");
    }
}

#[test]
fn crc32_vector_encodings_no_hmac() {
    assert_eq!(dt(&["hash", "-a", "crc32", "123456789"]).out, "cbf43926\n");
    assert_eq!(dt(&["hash", "-a", "crc32", ""]).out, "00000000\n");
    assert_eq!(
        dt(&["hash", "-a", "crc32", "-e", "base64", "123456789"]).out,
        "y/Q5Jg==\n"
    );
    assert_eq!(
        dt(&["hash", "-a", "crc32", "-e", "base64url", "123456789"]).out,
        "y_Q5Jg\n"
    );
    let r = dt(&["hash", "-a", "crc32", "-k", "x", "abc"]);
    assert_eq!(r.code, 2);
    assert_eq!(r.out, "");
}

#[test]
fn crc32_with_key_is_rejected_before_input_is_read() {
    // A missing file would exit 1 if the input were read first.
    let r = dt(&["hash", "-a", "crc32", "-k", "x", "-f", "/nonexistent/dt-input"]);
    assert_eq!(r.code, 2);
    assert_eq!(r.out, "");
}

#[test]
fn json_shape() {
    let r = dt(&["hash", "--json", "-a", "crc32", "123456789"]);
    let v: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"algorithm": "crc32", "encoding": "hex", "hmac": false, "digest": "cbf43926"})
    );
    let r = dt(&["hash", "--json", "-k", "Jefe", "what do ya want for nothing?"]);
    let v: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(v["hmac"], true);
    assert_eq!(v["algorithm"], "sha256");
    assert_eq!(
        v["digest"],
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

#[test]
fn unknown_algorithm_is_a_usage_error() {
    let r = dt(&["hash", "-a", "rot13", "x"]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("--algorithm tidak valid: rot13"), "{}", r.err);
}

#[test]
fn file_input_and_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::write(&p, "abc").unwrap();
    let r = dt(&["hash", "-f", p.to_str().unwrap()]);
    assert_eq!(
        trimmed(&r),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(dt(&["hash", "-f", p.to_str().unwrap(), "x"]).code, 2);
}

#[test]
fn every_hmac_algorithm_matches_legacy() {
    for (algorithm, expected) in [
        ("md5", "d2fe98063f876b03193afb49b4979591"),
        ("sha1", "4fd0b215276ef12f2b3e4c8ecac2811498b656fc"),
        ("sha224", "f524670b7e34f31467de0aa96593861cf65117d414fb2d86158d760e"),
        (
            "sha256",
            "9c196e32dc0175f86f4b1cb89289d6619de6bee699e4c378e68309ed97a1a6ab",
        ),
        (
            "sha384",
            "30ddb9c8f347cffbfb44e519d814f074cf4047a55d6f563324f1c6a33920e5edfb2a34bac60bdc96cd33a95623d7d638",
        ),
        (
            "sha512",
            "3926a207c8c42b0c41792cbd3e1a1aaaf5f7a25704f62dfc939c4987dd7ce060009c5bb1c2447355b3216f10b537e9afa7b64a4e5391b0d631172d07939e087a",
        ),
        (
            "sha3-256",
            "09b6dbab8d11795ca7c8d82f1cf91682013c7cb980abbb25473be4ae7f7b5683",
        ),
        (
            "sha3-512",
            "085e4e83503f40b82fef38438bc4905a55dbaa8c8878097a899db0b57ce7da57a368251c34474f60b3ebacb39b2edaca4b290456411c76ec7ab61944cfe2288e",
        ),
        (
            "blake2b512",
            "05cc4815438d5cfe68fff446b8df57828cc96189de4b4e928e3f06d815d64e5bc15124a02ffd39859b3e2476da03bc0235ca86df623af2a5631779809e9fd04a",
        ),
    ] {
        let r = dt(&["hash", "-a", algorithm, "-k", "key", "abc"]);
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(r.out.trim(), expected, "{algorithm}");
    }
}
