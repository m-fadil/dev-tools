mod common;
use common::{dt, dt_in};

#[test]
fn malformed_input_fails_with_exit_1() {
    assert_eq!(dt(&["decode", "-t", "hex", "abc"]).code, 1);
    assert_eq!(dt(&["decode", "-t", "base64", "a$b="]).code, 1);
    assert_eq!(dt(&["decode", "-t", "url", "%E0%A4%A"]).code, 1);
}

#[test]
fn url_decode_is_case_insensitive_and_plus_is_literal() {
    assert_eq!(dt_in(&["decode", "-t", "url"], b"%ff%FE").out_bytes, vec![0xff, 0xfe]);
    assert_eq!(dt(&["decode", "-t", "url", "a+b"]).out, "a+b");
}

#[test]
fn url_decode_requires_two_hex_digits_after_every_percent() {
    for bad in ["%", "%4", "%zz", "a%4g", "100%"] {
        assert_eq!(dt(&["decode", "-t", "url", bad]).code, 1, "{bad}");
    }
}

#[test]
fn base64_padding_and_trailing_bits_are_strict() {
    for ty in ["base64", "base64url"] {
        assert_eq!(dt(&["decode", "-t", ty, "YQ=="]).out, "a", "{ty}");
        assert_eq!(dt(&["decode", "-t", ty, "YQ"]).out, "a", "{ty}");
        assert_eq!(dt(&["decode", "-t", ty, "YWI="]).out, "ab", "{ty}");
        assert_eq!(dt(&["decode", "-t", ty, "YWI"]).out, "ab", "{ty}");
        for bad in ["YQ=", "YWJj=", "YWJj==", "YQ===", "Y", "=", "YR==", "YR", "YWJ=="] {
            assert_eq!(dt(&["decode", "-t", ty, bad]).code, 1, "{ty} {bad}");
        }
    }
    assert_eq!(dt(&["decode", "-t", "base64url", "Pz8-Pg=="]).out, "??>>");
    assert_eq!(dt(&["decode", "-t", "base64url", "Pz8-Pg"]).out, "??>>");
}

#[test]
fn whitespace_inside_base64_and_hex_is_ignored() {
    let raw = vec![7u8; 100];
    let b64 = {
        let r = dt_in(&["encode"], &raw);
        r.out.trim().to_string()
    };
    let wrapped: String = b64
        .as_bytes()
        .chunks(76)
        .map(|c| String::from_utf8_lossy(c).into_owned() + "\r\n")
        .collect();
    assert!(wrapped.contains("\r\n"));
    assert_eq!(dt_in(&["decode", "-t", "base64"], wrapped.as_bytes()).out_bytes, raw);
    assert_eq!(dt_in(&["decode", "-t", "base64"], b" YW Jj\n ZA = =\n").out, "abcd");
    assert_eq!(dt_in(&["decode", "-t", "base64url"], b" Pz 8-\n Pg = =\n").out, "??>>");
    assert_eq!(dt_in(&["decode", "-t", "hex"], b"68 61\n6c 6f\n").out, "halo");
    assert_eq!(dt_in(&["decode", "-t", "hex"], b"686 1").code, 0);
    assert_eq!(dt_in(&["decode", "-t", "hex"], b"6 8 6").code, 1);
}

#[test]
fn error_message_is_indonesian() {
    let r = dt(&["decode", "-t", "hex", "abc"]);
    assert!(r.err.contains("Input bukan hex yang valid."), "{}", r.err);
}
