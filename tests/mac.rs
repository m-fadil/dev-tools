mod common;
use common::{dt, dt_in};
use serde_json::{Value, json};

fn flags(a: &str) -> Value {
    serde_json::from_str(&dt(&["mac", a, "--json"]).out).unwrap()
}

#[test]
fn all_input_forms_normalise_to_the_same_address() {
    for form in [
        "00:1A:2B:3C:4D:5E",
        "00-1a-2b-3c-4d-5e",
        "001a.2b3c.4d5e",
        "001A2B3C4D5E",
    ] {
        assert_eq!(dt(&["mac", "-f", "colon", form]).out, "00:1a:2b:3c:4d:5e\n", "{form}");
    }
}

#[test]
fn formats_upper_stdin() {
    assert_eq!(
        dt(&["mac", "-f", "hyphen", "aabb.ccdd.eeff"]).out,
        "aa-bb-cc-dd-ee-ff\n"
    );
    assert_eq!(
        dt(&["mac", "--format", "dot", "--upper", "aa:bb:cc:dd:ee:ff"]).out,
        "AABB.CCDD.EEFF\n"
    );
    assert_eq!(
        dt_in(&["mac", "-f", "plain"], b"00-1A-2B-3C-4D-5E\n").out,
        "001a2b3c4d5e\n"
    );
    assert_eq!(
        dt(&["mac", "00:1a:2b:3c:4d:5e"]).out,
        "colon   00:1a:2b:3c:4d:5e\nhyphen  00-1a-2b-3c-4d-5e\ndot     001a.2b3c.4d5e\nplain   001a2b3c4d5e\n\
         tipe    unicast\nadmin   universally administered\n"
    );
}

#[test]
fn flag_bits() {
    assert_eq!(flags("00:00:00:00:00:00")["multicast"], false);
    assert_eq!(flags("00:00:00:00:00:00")["locallyAdministered"], false);
    assert_eq!(flags("01:00:5e:00:00:01")["multicast"], true);
    assert_eq!(flags("02:00:00:00:00:00")["locallyAdministered"], true);
    assert_eq!(flags("ff:ff:ff:ff:ff:ff")["multicast"], true);
    let out = dt(&["mac", "03:00:00:00:00:00"]).out;
    assert!(out.contains("multicast") && out.contains("locally administered"));
}

#[test]
fn json_shape() {
    let d: Value =
        serde_json::from_str(&dt(&["mac", "-f", "dot", "--upper", "--json", "aa:bb:cc:dd:ee:ff"]).out).unwrap();
    assert_eq!(
        d,
        json!({
            "address": "AABB.CCDD.EEFF",
            "formats": {
                "colon": "AA:BB:CC:DD:EE:FF",
                "hyphen": "AA-BB-CC-DD-EE-FF",
                "dot": "AABB.CCDD.EEFF",
                "plain": "AABBCCDDEEFF",
            },
            "multicast": false,
            "locallyAdministered": true,
        })
    );
}

#[test]
fn invalid_addresses_are_usage_errors() {
    for bad in [
        "aa:bb:cc:dd:ee",
        "aa:bb:cc:dd:ee:gg",
        "aa-bb:cc-dd:ee-ff",
        "aabb.ccdd",
        "aabbccddeeff00",
        "a:b:c:d:e:f",
        "",
    ] {
        assert_eq!(dt(&["mac", bad]).code, 2, "argument {bad}");
        assert_eq!(dt_in(&["mac"], bad.as_bytes()).code, 2, "stdin {bad}");
    }
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    for padded in ["aa:bb:cc:dd:ee:ff\n", "  aa:bb:cc:dd:ee:ff \t", "\naabbccddeeff"] {
        let arg = dt(&["mac", "-f", "hyphen", padded]);
        assert_eq!(arg.code, 0, "{padded:?}");
        assert_eq!(arg.out, "aa-bb-cc-dd-ee-ff\n");
        let stdin = dt_in(&["mac", "-f", "hyphen"], padded.as_bytes());
        assert_eq!((stdin.code, stdin.out, stdin.err), (arg.code, arg.out, arg.err));
    }
}

#[test]
fn error_echoes_trimmed_text() {
    let input = " \tzz:bb:cc:dd:ee:ff\n";
    let r = dt(&["mac", input]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("zz:bb:cc:dd:ee:ff ("), "{}", r.err);
    let s = dt_in(&["mac"], input.as_bytes());
    assert_eq!((s.code, s.err), (r.code, r.err));
}

#[test]
fn usage_errors() {
    assert_eq!(dt(&["mac", "-f", "octal", "aabbccddeeff"]).code, 2);
    assert_eq!(dt(&["mac", "aabbccddeeff", "112233445566"]).code, 2);
    assert_eq!(dt(&["mac", "--random", "aabbccddeeff"]).code, 2);
}

#[test]
fn random_is_locally_administered_unicast() {
    for _ in 0..3 {
        let d: Value = serde_json::from_str(&dt(&["mac", "--random", "--json"]).out).unwrap();
        assert_eq!(d["multicast"], false);
        assert_eq!(d["locallyAdministered"], true);
        let a = d["address"].as_str().unwrap();
        assert_eq!(a.len(), 17);
        assert!(
            a.split(':')
                .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
        );
    }
    let out = dt(&["mac", "--random", "-f", "plain", "--upper"]).out;
    assert_eq!(out.len(), 13);
    assert!(out.trim_end().bytes().all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'F')));
}

#[test]
fn random_varies_and_fills_the_free_bits() {
    let mut seen = std::collections::HashSet::new();
    let (mut ones, mut zeros) = (0u64, 0u64);
    for _ in 0..64 {
        let out = dt(&["mac", "--random", "-f", "plain"]).out;
        let bits = u64::from_str_radix(out.trim(), 16).unwrap();
        assert_eq!((bits >> 40) & 0x03, 0x02, "U/L set, I/G clear");
        seen.insert(bits);
        ones |= bits;
        zeros |= !bits & 0xffff_ffff_ffff;
    }
    assert!(seen.len() > 60, "addresses must not repeat");
    let free = 0xffff_ffff_ffffu64 ^ (0x03u64 << 40);
    assert_eq!(ones & free, free);
    assert_eq!(zeros & free, free);
}
