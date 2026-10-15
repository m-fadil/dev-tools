mod common;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{dt, dt_env, dt_in};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::{Sha256, Sha384, Sha512};
const SAMPLE: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
const NOW: &str = "2026-01-01T00:00:00Z";
const EPOCH: i64 = 1767225600;
fn b64(v: &Value) -> String {
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(v).unwrap())
}
fn raw_sign(payload: &str, key: &str, header: Value) -> String {
    let input = format!("{}.{}", b64(&header), URL_SAFE_NO_PAD.encode(payload));
    macro_rules! digest {
        ($h:ty) => {{
            let mut h = Hmac::<$h>::new_from_slice(key.as_bytes()).unwrap();
            h.update(input.as_bytes());
            h.finalize().into_bytes().to_vec()
        }};
    }
    let sig = match header["alg"].as_str().unwrap_or("") {
        "HS384" => digest!(Sha384),
        "HS512" => digest!(Sha512),
        _ => digest!(Sha256),
    };
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(sig))
}
fn sign(payload: Value, key: &str) -> String {
    raw_sign(&payload.to_string(), key, json!({"alg":"HS256","typ":"JWT"}))
}
fn doc(args: &[&str]) -> Value {
    let r = dt(args);
    assert_eq!(r.code, 0, "{}", r.err);
    serde_json::from_str(&r.out).unwrap()
}
#[test]
fn decode_sample_text_and_bearer() {
    let r = dt(&["jwt", "--now", NOW, SAMPLE]);
    assert_eq!(r.code, 0);
    for s in [
        "Header:\n{\n  \"alg\": \"HS256\",\n  \"typ\": \"JWT\"\n}",
        "Payload:\n{\n  \"sub\": \"1234567890\",\n  \"name\": \"John Doe\",\n  \"iat\": 1516239022\n}",
        "iat: 2018-01-18T01:30:22.000Z (",
        "hari lalu)",
        "TIDAK diverifikasi",
    ] {
        assert!(r.out.contains(s), "{s}: {}", r.out);
    }
    let a = dt_in(&["jwt", "--now", NOW], format!("  Bearer {SAMPLE}\n").as_bytes());
    assert_eq!(a.code, 0);
    assert_eq!(a.out, r.out);
    assert_eq!(dt(&["jwt", "--now", NOW, "Bearer", SAMPLE]).out, a.out);
}
#[test]
fn time_claim_statuses() {
    let expired = sign(json!({"exp":EPOCH-3*3600-10}), "k");
    let r = dt(&["jwt", "--now", &EPOCH.to_string(), &expired]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains("exp: 2025-12-31T20:59:50.000Z (kedaluwarsa 3 jam lalu)"));
    let token = sign(json!({"exp":EPOCH+2*86400+5,"nbf":EPOCH+60}), "k");
    let r = dt(&["jwt", "--now", NOW, &token]);
    assert!(r.out.contains("(berlaku 2 hari lagi)"));
    assert!(
        r.out
            .contains("nbf: 2026-01-01T00:01:00.000Z (belum berlaku, 1 menit lagi)")
    );
    let token = sign(json!({"nbf":EPOCH-1}), "k");
    assert!(dt(&["jwt", "--now", NOW, &token]).out.contains("(sudah berlaku)"));
}
#[test]
fn mistyped_claims_unverified_warn_but_succeed() {
    let token = sign(json!({"exp":"1","nbf":null}), "k");
    for json in [false, true] {
        let mut a = vec!["jwt", "--now", NOW];
        if json {
            a.push("--json");
        }
        a.push(&token);
        let r = dt(&a);
        assert_eq!(r.code, 0);
        assert!(r.err.contains("Peringatan: Klaim exp, nbf harus berupa angka"));
        if json {
            let j: Value = serde_json::from_str(&r.out).unwrap();
            assert_eq!(j["payload"], json!({"exp":"1","nbf":null}));
            assert_eq!(j["times"], json!([]));
            assert_eq!(j["valid"], Value::Null);
        } else {
            assert!(!r.out.contains("exp:"));
            assert!(r.out.contains("TIDAK diverifikasi"));
        }
    }
}
#[test]
fn malformed_tokens_no_stdout() {
    let h = b64(&json!({"alg":"HS256"}));
    let p = b64(&json!({}));
    for (token, message) in [
        ("abc.def".into(), "3 segmen"),
        ("a.b.c.d".into(), "3 segmen"),
        (format!("{h}.!!!.x"), "Payload bukan base64url"),
        (format!("{h}.{p}=.x"), "Payload bukan base64url"),
        (format!("{h}.{p}.a+b"), "Signature bukan base64url"),
        (
            format!("{}.{p}.", URL_SAFE_NO_PAD.encode("not json")),
            "Header bukan JSON",
        ),
        (format!("{}.{p}.", b64(&json!([1]))), "Header harus berupa objek"),
        (format!("{p}.{}.", b64(&json!("str"))), "Payload harus berupa objek"),
    ] {
        let r = dt(&["jwt", &token]);
        assert_eq!(r.code, 1);
        assert!(r.err.contains(message));
        assert!(r.out.is_empty());
    }
    assert_eq!(dt_in(&["jwt"], b"  \n").code, 1);
    assert_eq!(dt(&["jwt", "--now", "kemarin", SAMPLE]).code, 2);
}
#[test]
fn now_is_utc_and_rejects_rollover() {
    let token = sign(json!({"exp":EPOCH+1,"nbf":EPOCH}), "k");
    let expected = dt(&["jwt", "-k", "k", "--json", "--now", NOW, &token]);
    assert_eq!(expected.code, 0);
    for now in ["2026-01-01", "2026-01-01T00:00:00", "2026-01-01 00:00:00"] {
        let r = dt_env(
            &["jwt", "-k", "k", "--json", "--now", now, &token],
            b"",
            &[("TZ", "Asia/Jakarta")],
        );
        assert_eq!(
            (r.code, r.out, r.err),
            (expected.code, expected.out.clone(), expected.err.clone())
        );
    }
    for now in [
        "2026-02-30",
        "2026-02-29 00:00:00",
        "2026-02-30T00:00:00Z",
        "2026-04-31T00:00:00+07:00",
        "2026-00-01",
        "2026-13-01",
        "2026-01-00",
        "2026-01-01T24:00:00",
        "2026-01-01T00:60:00",
        "2026-01-01T00:00:60",
        "2026-01-01T00:00:00+24:00",
    ] {
        let r = dt(&["jwt", "--now", now, SAMPLE]);
        assert_eq!(r.code, 2, "{now}");
        assert!(r.out.is_empty());
        assert!(r.err.contains("--now tidak valid"));
    }
    for now in [
        "2024-02-29T00:00:00Z",
        "2024-02-29 00:00:00",
        "2024-02-29T07:00:00+07:00",
    ] {
        assert_eq!(
            doc(&["jwt", "-s", "-k", "k", "--now", now, "--iat", "--json", "{}"])["payload"]["iat"],
            1709164800
        );
    }
    let token = sign(json!({"exp":EPOCH as f64+0.5}), "k");
    assert_eq!(
        dt(&["jwt", "-k", "k", "--now", "2026-01-01 00:00:00.25", &token]).code,
        0
    );
    assert_eq!(
        dt(&["jwt", "-k", "k", "--now", "2026-01-01T00:00:00.5Z", &token]).code,
        1
    );
}
#[test]
fn sample_verification_and_exact_signing_vector() {
    let r = dt(&["jwt", "-k", "your-256-bit-secret", "--now", NOW, SAMPLE]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains("Signature: valid"));
    assert!(!r.out.contains("TIDAK"));
    let r = dt(&[
        "jwt",
        "--sign",
        "-k",
        "your-256-bit-secret",
        "{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022}",
    ]);
    assert_eq!(r.code, 0);
    assert_eq!(r.out, format!("{SAMPLE}\n"));
}
// Generated by node legacy/dist/src/cli.js jwt -s -k rahasia --alg ALG '{"a":1}'.
// The legacy verifier also accepted all three constants. Exact signing equality
// proves Rust-to-legacy compatibility without requiring Node at test runtime.
const LEGACY: [(&str, &str); 3] = [
    (
        "HS256",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhIjoxfQ.9gAvxdJb4Y7i7bJyeIHAzM5tezTUIkUtB6rDnTs98sw",
    ),
    (
        "HS384",
        "eyJhbGciOiJIUzM4NCIsInR5cCI6IkpXVCJ9.eyJhIjoxfQ.Jklrm5jvT7NDZEcLCqK0YZdDbFEigGoqY8RKkuoEy672Q1SWX5ZocV9Lq4bzx57j",
    ),
    (
        "HS512",
        "eyJhbGciOiJIUzUxMiIsInR5cCI6IkpXVCJ9.eyJhIjoxfQ.XhrikDwij8_L73NGi9WlVSetx37wNRXSIc8LS6TvBmyGa2a1V7sYKBC4IZCwM-h3g8n9jMpG8u4XpZYja_1xkQ",
    ),
];
#[test]
fn every_algorithm_legacy_interoperability_and_round_trip() {
    for (alg, token) in LEGACY {
        let r = dt(&["jwt", "-s", "-k", "rahasia", "--alg", alg, "{\"a\":1}"]);
        assert_eq!(r.code, 0);
        assert_eq!(r.out.trim(), token);
        let ok = dt(&["jwt", "-k", "rahasia", token]);
        assert_eq!(ok.code, 0);
        assert!(ok.out.contains("Signature: valid"));
        assert!(ok.out.contains(&format!("\"alg\": \"{alg}\"")));
        assert_eq!(dt(&["jwt", "-k", "salah", token]).code, 1);
    }
}
#[test]
fn wrong_key_tampering_and_signature_lengths() {
    let r = dt(&["jwt", "-k", "salah", SAMPLE]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Signature tidak valid"));
    assert!(r.out.contains("John Doe"));
    let p: Vec<_> = SAMPLE.split('.').collect();
    let tampered = format!(
        "{}.{}.{}",
        p[0],
        b64(&json!({"sub":"1234567890","name":"Admin","iat":1516239022})),
        p[2]
    );
    for token in [
        tampered,
        format!("{}.{}.", p[0], p[1]),
        SAMPLE[..SAMPLE.len() - 4].into(),
    ] {
        assert_eq!(dt(&["jwt", "-k", "your-256-bit-secret", &token]).code, 1);
    }
}
#[test]
fn unsupported_algorithms_and_missing_alg() {
    for header in [
        json!({"alg":"none"}),
        json!({"alg":"None"}),
        json!({"alg":"RS256"}),
        json!({"alg":"ES256"}),
        json!({"typ":"JWT"}),
        json!({"alg":"constructor"}),
    ] {
        let token = format!("{}.{}.", b64(&header), b64(&json!({"sub":"x"})));
        let r = dt(&["jwt", "-k", "k", &token]);
        assert_eq!(r.code, 1);
        assert!(r.err.contains("Algoritma tidak didukung"));
    }
}
#[test]
fn expired_early_and_boundary_claims_print_first() {
    for (payload, message) in [
        (json!({"exp":EPOCH-10}), "kedaluwarsa"),
        (json!({"nbf":EPOCH+10}), "belum berlaku"),
        (json!({"exp":EPOCH}), "kedaluwarsa"),
    ] {
        let token = sign(payload, "k");
        let r = dt(&["jwt", "-k", "k", "--now", NOW, &token]);
        assert_eq!(r.code, 1);
        assert!(r.out.contains("Signature: valid"));
        assert!(r.err.contains(message));
    }
    assert_eq!(
        dt(&["jwt", "-k", "k", "--now", NOW, &sign(json!({"exp":EPOCH+1}), "k")]).code,
        0
    );
    let r = dt(&["jwt", "-k", "salah", "--now", NOW, &sign(json!({"exp":1}), "k")]);
    assert!(r.err.contains("Signature tidak valid"));
    assert_eq!(dt(&["jwt", "-k", "", SAMPLE]).code, 2);
}
#[test]
fn present_claims_must_be_numeric_with_key() {
    for name in ["exp", "nbf"] {
        for value in [json!("1"), Value::Null, json!(true), json!([]), json!({})] {
            let mut payload = json!({});
            payload[name] = value;
            let token = sign(payload, "k");
            for json in [false, true] {
                let mut args = vec!["jwt", "-k", "k", "--now", NOW];
                if json {
                    args.push("--json");
                }
                args.push(&token);
                let r = dt(&args);
                assert_eq!(r.code, 1);
                assert!(r.err.contains(&format!("Klaim {name} harus berupa angka")));
                if json {
                    let j: Value = serde_json::from_str(&r.out).unwrap();
                    assert_eq!(j["signature"]["verified"], true);
                    assert_eq!(j["valid"], false);
                } else {
                    assert!(r.out.contains("Payload:"));
                    assert!(r.out.contains("Signature: valid"));
                }
            }
        }
    }
    assert_eq!(dt(&["jwt", "-k", "k", &sign(json!({}), "k")]).code, 0);
}
#[test]
fn infinite_numeric_dates_and_null_iso() {
    for (payload, valid, status) in [
        ("{\"exp\":-1e999}", false, "kedaluwarsa"),
        ("{\"exp\":1e999}", true, "tidak pernah kedaluwarsa"),
        ("{\"nbf\":1e999}", false, "belum berlaku"),
        ("{\"nbf\":-1e999}", true, "sudah berlaku"),
        ("{\"exp\":1e999,\"nbf\":-1e999,\"iat\":1e999}", true, "di masa depan"),
    ] {
        let token = raw_sign(payload, "k", json!({"alg":"HS256"}));
        let r = dt(&["jwt", "-k", "k", "--now", NOW, "--json", &token]);
        assert_eq!(r.code, if valid { 0 } else { 1 });
        let j: Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(j["signature"]["verified"], true);
        assert_eq!(j["valid"], valid);
        assert!(j["times"].as_array().unwrap().iter().all(|t| t["iso"].is_null()));
        assert!(
            j["times"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["status"].as_str().unwrap().contains(status))
        );
        if !valid {
            assert!(r.err.contains(status));
        }
        let r = dt(&["jwt", "-k", "k", "--now", NOW, &token]);
        assert_eq!(r.code, if valid { 0 } else { 1 });
        assert!(r.out.contains(status));
        assert!(doc(&["jwt", "--now", NOW, "--json", &token])["valid"].is_null());
    }
}
#[test]
fn expiry_and_iat_override_in_payload_order() {
    let j = doc(&[
        "jwt",
        "-s",
        "-k",
        "k",
        "--now",
        NOW,
        "--exp",
        "2w",
        "--iat",
        "--json",
        "{\"exp\":5,\"iat\":6,\"x\":\"y\"}",
    ]);
    assert_eq!(j["payload"], json!({"exp":EPOCH+14*86400,"iat":EPOCH,"x":"y"}));
    assert_eq!(j["header"], json!({"alg":"HS256","typ":"JWT"}));
    assert_eq!(
        dt(&["jwt", "-k", "k", "--now", NOW, j["token"].as_str().unwrap()]).code,
        0
    );
    for (value, expected) in [
        ("30s", EPOCH + 30),
        ("15m", EPOCH + 900),
        ("1h", EPOCH + 3600),
        ("7d", EPOCH + 7 * 86400),
        ("1767229200", 1767229200),
    ] {
        assert_eq!(
            doc(&["jwt", "-s", "-k", "k", "--now", NOW, "--exp", value, "--json", "{}"])["payload"]["exp"],
            expected
        );
    }
    let token = dt_in(
        &["jwt", "-s", "-k", "k", "--now", NOW, "--exp", "1m"],
        b"{\"sub\":\"1\"}\n",
    )
    .out;
    assert_eq!(
        dt(&["jwt", "-k", "k", "--now", "2026-01-01T00:00:30Z", token.trim()]).code,
        0
    );
    assert_eq!(
        dt(&["jwt", "-k", "k", "--now", "2026-01-01T00:02:00Z", token.trim()]).code,
        1
    );
}
#[test]
fn signing_usage_errors() {
    for args in [
        vec!["-s", "{}"],
        vec!["-s", "-k", "k", "[1]"],
        vec!["-s", "-k", "k", "\"str\""],
        vec!["-s", "-k", "k", "bukan json"],
        vec!["-s", "-k", "k", "--alg", "RS256", "{}"],
        vec!["-s", "-k", "k", "--exp", "1x", "{}"],
        vec!["-s", "-k", "k", "--exp", "-5m", "{}"],
        vec!["-s", "-k", "k", "--exp", "1.5h", "{}"],
        vec!["-k", "k", "--exp", "1h", SAMPLE],
        vec!["--iat", SAMPLE],
        vec!["--alg", "HS256", SAMPLE],
    ] {
        let mut a = vec!["jwt"];
        a.extend(args);
        let r = dt(&a);
        assert_eq!(r.code, 2);
        assert!(r.out.is_empty());
    }
}
#[test]
fn json_output_shapes_and_failures() {
    let j = doc(&["jwt", "--json", "--now", NOW, SAMPLE]);
    assert_eq!(j["header"], json!({"alg":"HS256","typ":"JWT"}));
    assert_eq!(j["payload"]["name"], "John Doe");
    assert_eq!(j["times"].as_array().unwrap().len(), 1);
    assert_eq!(j["times"][0]["claim"], "iat");
    assert_eq!(j["times"][0]["iso"], "2018-01-18T01:30:22.000Z");
    assert!(j["times"][0]["status"].is_string());
    assert_eq!(j["signature"], json!({"verified":null}));
    assert!(j["valid"].is_null());
    let j = doc(&["jwt", "--json", "-k", "your-256-bit-secret", "--now", NOW, SAMPLE]);
    assert_eq!(j["signature"], json!({"verified":true,"algorithm":"HS256"}));
    assert_eq!(j["valid"], true);
    let expired = sign(json!({"exp":EPOCH-10}), "k");
    let none = format!("{}.{}.", b64(&json!({"alg":"none"})), b64(&json!({})));
    for (key, token, verified) in [
        ("salah", SAMPLE, false),
        ("k", expired.as_str(), true),
        ("k", none.as_str(), false),
    ] {
        let r = dt(&["jwt", "--json", "-k", key, "--now", NOW, token]);
        assert_eq!(r.code, 1);
        let j: Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(j["valid"], false);
        assert_eq!(j["signature"]["verified"], verified);
        if verified {
            assert_eq!(j["times"][0]["claim"], "exp");
        }
    }
    let r = dt(&["jwt", "--json", "abc"]);
    assert_eq!(r.code, 1);
    assert!(r.out.is_empty());
}

#[test]
fn surrogate_payload_and_real_unicode_signing_preserve_wire_bytes() {
    let payload = format!(
        "{{\"s\":\"\\ud800\",\"real\":\"{}d800;\",\"pua\":\"{}\"}}",
        '\u{ffff}', '\u{e000}'
    );
    let expected = raw_sign(&payload, "k", json!({"alg":"HS256","typ":"JWT"}));
    let r = dt(&["jwt", "-s", "-k", "k", &payload]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.out.trim(), expected);
    let r = dt(&["jwt", "-k", "k", &expected]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.out.contains("\"s\": \"\\ud800\""));
    assert!(r.out.contains(&format!("\"real\": \"{}d800;\"", '\u{ffff}')));
}

#[test]
fn jwt_above_json_depth_limit_matches_legacy_sign_and_decode() {
    use sha2::Digest;
    // Legacy Node CLI accepted both operations with 2001 nested arrays.
    // SHA-256 fixtures cover the complete stdout, including JSON indentation.
    let payload = format!("{{\"a\":{}0{}}}", "[".repeat(2001), "]".repeat(2001));
    let signed = dt_in(&["jwt", "-s", "-k", "k"], payload.as_bytes());
    assert_eq!(signed.code, 0, "{}", signed.err);
    assert_eq!(
        hex::encode(Sha256::digest(signed.out.as_bytes())),
        "33585aae207cfb201e16ae52b777c35fb526f34b2d9cd9967ecbabe7bc55caab"
    );
    let decoded = dt_in(&["jwt", "-k", "k", "--json", "--now", NOW], signed.out.as_bytes());
    assert_eq!(decoded.code, 0, "{}", decoded.err);
    assert_eq!(decoded.out.len(), 8_032_205);
    assert_eq!(
        hex::encode(Sha256::digest(decoded.out.as_bytes())),
        "acad5fd64500359f043ae736faf6cf8ebdfd6add60fde942d7d5dc3b5f5f911a"
    );
    // JSON's own container limit remains independent and unchanged.
    let checked = dt_in(&["json", "--check"], payload.as_bytes());
    assert_eq!(checked.code, 1);
    assert_eq!(checked.err.trim(), "JSON terlalu dalam (maks 2000 level)");
}
