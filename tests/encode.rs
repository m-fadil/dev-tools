mod common;
use common::{dt, dt_in};

#[test]
fn round_trip_for_every_type_including_non_ascii() {
    let text = "halo dunia & café ✓?";
    for ty in ["base64", "base64url", "hex", "url"] {
        let enc = dt(&["encode", "-t", ty, text]);
        assert_eq!(enc.code, 0, "{}", enc.err);
        let dec = dt_in(&["decode", "-t", ty], enc.out.as_bytes());
        assert_eq!(dec.out, text, "{ty}");
    }
    assert_eq!(dt(&["encode", "halo"]).out, "aGFsbw==\n");
    assert_eq!(dt(&["encode", "-t", "base64url", "??>>"]).out, "Pz8-Pg\n");
}

#[test]
fn json_shape() {
    let v: serde_json::Value = serde_json::from_str(&dt(&["encode", "--json", "halo"]).out).unwrap();
    assert_eq!(v, serde_json::json!({"type": "base64", "output": "aGFsbw=="}));
    let v: serde_json::Value = serde_json::from_str(&dt(&["encode", "--json", "-t", "hex", "hi"]).out).unwrap();
    assert_eq!(v, serde_json::json!({"type": "hex", "output": "6869"}));
}

#[test]
fn url_encoding_works_on_raw_bytes_so_binary_round_trips() {
    let enc = dt_in(&["encode", "-t", "url"], &[0xff, 0xfe]);
    assert_eq!(enc.out, "%FF%FE\n");
    let all: Vec<u8> = (0..=255u8).collect();
    let round = dt_in(&["encode", "-t", "url"], &all);
    assert_eq!(round.code, 0, "{}", round.err);
    let dec = dt_in(&["decode", "-t", "url"], &round.out_bytes);
    assert_eq!(dec.out_bytes, all);
    // unreserved characters stay literal; '+' is not a space
    assert_eq!(
        dt(&["encode", "-t", "url", "a-_.!~*'()Z9 +"]).out,
        "a-_.!~*'()Z9%20%2B\n"
    );
}

#[test]
fn bad_type_is_a_usage_error() {
    let r = dt(&["encode", "-t", "rot13", "x"]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("--type tidak valid: rot13"), "{}", r.err);
}

#[test]
fn help_matches_legacy() {
    let r = dt(&["encode", "--help"]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("dt encode [options] [teks]\n"));
}
