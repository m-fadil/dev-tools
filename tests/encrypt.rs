mod common;
use common::{dt, dt_env, dt_in};

/// Token produced by the legacy TypeScript build: `dt encrypt -p secret halo`.
const LEGACY_TOKEN: &str = "AZrAz_pf74lH5HjwMWQfNZ8q91ajTF81XqBMJtTUdCkvtWLoTAcnAg0nBfhd5q8lWQ";

fn b64url_decode(s: &str) -> Vec<u8> {
    dt_in(&["decode", "-t", "base64url"], s.as_bytes()).out_bytes
}

fn b64url_encode(raw: &[u8]) -> String {
    dt_in(&["encode", "-t", "base64url"], raw).out.trim().to_string()
}

#[test]
fn decrypts_a_token_made_by_the_legacy_build() {
    let r = dt_in(&["encrypt", "-d", "-p", "secret"], LEGACY_TOKEN.as_bytes());
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.out, "halo");
}

#[test]
fn round_trip_random_per_run_wrong_password_and_tampering_rejected() {
    let a = dt(&["encrypt", "-p", "rahasia", "halo"]);
    assert_eq!(a.code, 0, "{}", a.err);
    assert_ne!(a.out, dt(&["encrypt", "-p", "rahasia", "halo"]).out);
    assert_eq!(dt_in(&["encrypt", "-d", "-p", "rahasia"], a.out.as_bytes()).out, "halo");

    let wrong = dt_in(&["encrypt", "-d", "-p", "salah"], a.out.as_bytes());
    assert_eq!(wrong.code, 1);
    assert!(wrong.err.contains("password salah atau data rusak"), "{}", wrong.err);

    // Flip one byte of the decoded token (version | salt 16 | iv 12 | tag 16 | ciphertext), then re-encode.
    let raw = b64url_decode(a.out.trim());
    for (name, offset) in [("salt", 1), ("iv", 17), ("tag", 29), ("ciphertext", raw.len() - 1)] {
        let mut tampered = raw.clone();
        tampered[offset] ^= 1;
        let r = dt_in(&["encrypt", "-d", "-p", "rahasia"], b64url_encode(&tampered).as_bytes());
        assert_eq!(r.code, 1, "{name}");
        assert!(r.err.contains("password salah atau data rusak"), "{name}: {}", r.err);
    }
}

#[test]
fn truncated_token_and_unknown_version_byte_are_rejected() {
    let raw = b64url_decode(dt(&["encrypt", "-p", "rahasia", "halo"]).out.trim());
    let decrypt_with = |bytes: &[u8]| dt_in(&["encrypt", "-d", "-p", "rahasia"], b64url_encode(bytes).as_bytes());
    assert_eq!(decrypt_with(&raw[..40]).code, 1);
    assert_eq!(decrypt_with(&raw[..1]).code, 1);
    assert_eq!(decrypt_with(&[]).code, 1);
    // dropping the last ciphertext byte breaks the auth tag
    assert_eq!(decrypt_with(&raw[..raw.len() - 1]).code, 1);
    for version in [0u8, 2, 255] {
        let mut wrong = raw.clone();
        wrong[0] = version;
        assert_eq!(decrypt_with(&wrong).code, 1, "version {version}");
    }
    let r = decrypt_with(&raw[..40]);
    assert!(r.err.contains("Format data terenkripsi tidak dikenal."), "{}", r.err);
}

#[test]
fn explicit_empty_password_is_a_usage_error_even_when_env_has_one() {
    let r = dt_env(&["encrypt", "-p", "", "halo"], b"", &[("DEV_TOOL_PASSWORD", "pw")]);
    assert_eq!(r.code, 2);
    assert_eq!(r.out, "");
    assert_eq!(
        dt_env(&["encrypt", "-d", "-p", ""], b"x", &[("DEV_TOOL_PASSWORD", "pw")]).code,
        2
    );
}

#[test]
fn password_from_env_missing_password_without_terminal_is_usage_error() {
    let a = dt_env(&["encrypt", "halo"], b"", &[("DEV_TOOL_PASSWORD", "pw")]);
    assert_eq!(
        dt_env(&["encrypt", "-d"], a.out.as_bytes(), &[("DEV_TOOL_PASSWORD", "pw")]).out,
        "halo"
    );
    let missing = dt(&["encrypt", "halo"]);
    assert_eq!(missing.code, 2);
    assert!(missing.err.contains("Password diperlukan"), "{}", missing.err);
}

#[test]
fn token_layout_and_version_byte_as_aad() {
    let raw = b64url_decode(LEGACY_TOKEN);
    assert_eq!(raw[0], 1);
    assert_eq!(raw.len(), 1 + 16 + 12 + 16 + 4);
    let emitted = b64url_decode(dt(&["encrypt", "-p", "secret", "halo"]).out.trim());
    assert_eq!(emitted.len(), raw.len());
    assert_eq!(emitted[0], 1);
}

#[test]
fn fixed_node_crypto_vectors_bind_version_as_aad() {
    // Node crypto: fixed salt [1;16], IV [2;12], scrypt N=16384/r=8/p=1.
    let with_aad = "AQEBAQEBAQEBAQEBAQEBAQECAgICAgICAgICAgIgQ6-3UrV6DfrV6bfq1hjVfc4jVA";
    let without_aad = "AQEBAQEBAQEBAQEBAQEBAQECAgICAgICAgICAgKmp2Jnxwr2HYrA4prnN4D0fc4jVA";
    let r = dt(&["encrypt", "-d", "-p", "rahasia", with_aad]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.out, "halo");
    assert_eq!(dt(&["encrypt", "-d", "-p", "rahasia", without_aad]).code, 1);
}
