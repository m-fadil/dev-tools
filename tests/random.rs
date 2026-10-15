mod common;
use common::dt;
use std::collections::HashSet;
fn lines(args: &[&str]) -> Vec<String> {
    let r = dt(args);
    assert_eq!(r.code, 0, "{}", r.err);
    r.out.lines().map(str::to_owned).collect()
}
fn check(s: &str, p: &str) {
    assert!(regex::Regex::new(p).unwrap().is_match(s), "{s}");
}
fn ticks(s: &str) -> u128 {
    let p: Vec<_> = s.split('-').collect();
    u128::from_str_radix(&format!("{}{}{}", &p[2][1..], p[1], p[0]), 16).unwrap()
}
fn now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
#[test]
fn uuid_layout_uniqueness_and_order() {
    for (version, count) in [("4", "3"), ("7", "1000"), ("1", "500")] {
        let before = now();
        let a = lines(&["random", "uuid", "-v", version, "-c", count]);
        let after = now();
        assert_eq!(a.len(), count.parse::<usize>().unwrap());
        assert_eq!(a.iter().collect::<HashSet<_>>().len(), a.len());
        let pattern =
            format!("^[0-9a-f]{{8}}-[0-9a-f]{{4}}-{version}[0-9a-f]{{3}}-[89ab][0-9a-f]{{3}}-[0-9a-f]{{12}}$");
        for s in &a {
            check(s, &pattern);
        }
        if version == "7" {
            for pair in a.windows(2) {
                assert!(pair[0] < pair[1]);
            }
            let ms = u128::from_str_radix(&a[0].replace('-', ""), 16).unwrap() >> 80;
            assert!(ms >= before - 1000 && ms <= after + 1000);
        }
        if version == "1" {
            assert_eq!(a.iter().map(|s| &s[19..]).collect::<HashSet<_>>().len(), 1);
            assert_eq!(u8::from_str_radix(&a[0][24..26], 16).unwrap() & 1, 1);
            for pair in a.windows(2) {
                assert!(ticks(&pair[0]) < ticks(&pair[1]));
            }
            let ms = ticks(&a[0]) / 10000 - 12_219_292_800_000;
            assert!(ms.abs_diff(after) < 60_000);
        }
    }
    for s in lines(&["random", "uuid", "-c", "3"]) {
        assert_eq!(&s[14..15], "4");
    }
}
#[test]
fn tokens_sizes_formats_and_count() {
    for (extra, pattern) in [
        (vec![], "^[0-9a-f]{64}$"),
        (vec!["-b", "24", "-f", "base64url"], "^[A-Za-z0-9_-]{32}$"),
        (vec!["--bits", "128"], "^[0-9a-f]{32}$"),
        (vec!["-l", "7"], "^[0-9a-f]{7}$"),
        (vec!["-l", "50", "-f", "base64"], "^[A-Za-z0-9+/]{50}$"),
        (vec!["-l", "50", "-f", "base64url"], "^[A-Za-z0-9_-]{50}$"),
        (vec!["-f", "alnum"], "^[A-Za-z0-9]{32}$"),
        (vec!["-l", "5", "-f", "alnum"], "^[A-Za-z0-9]{5}$"),
    ] {
        let mut args = vec!["random", "token"];
        args.extend(extra);
        check(&lines(&args)[0], pattern);
    }
    assert_ne!(
        lines(&["random", "token", "-l", "40", "-f", "alnum"]),
        lines(&["random", "token", "-l", "40", "-f", "alnum"])
    );
    let r = dt(&["random", "token", "-l", "20", "-f", "alnum", "-c", "3", "--json"]);
    let j: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(j["type"], "token");
    assert_eq!(j["values"].as_array().unwrap().len(), 3);
    for s in j["values"].as_array().unwrap() {
        check(s.as_str().unwrap(), "^[A-Za-z0-9]{20}$");
    }
}
#[test]
fn json_shapes_and_usage_errors() {
    for (args, version, count) in [
        (vec!["random", "uuid", "--json", "-v", "7", "-c", "2"], 7, 2),
        (vec!["random", "uuid", "--json"], 4, 1),
    ] {
        let j: serde_json::Value = serde_json::from_str(&dt(&args).out).unwrap();
        assert_eq!(j["type"], "uuid");
        assert_eq!(j["version"], version);
        assert_eq!(j["values"].as_array().unwrap().len(), count);
    }
    let j: serde_json::Value =
        serde_json::from_str(&dt(&["random", "token", "--json", "-b", "4", "-f", "hex"]).out).unwrap();
    assert_eq!(j["format"], "hex");
    assert!(j.get("version").is_none());
    check(j["values"][0].as_str().unwrap(), "^[0-9a-f]{8}$");
    for extra in [
        vec!["uuid", "-b", "8"],
        vec!["uuid", "--bits", "128"],
        vec!["uuid", "--length", "8"],
        vec!["uuid", "--format", "hex"],
        vec!["uuid", "--token"],
        vec!["uuid", "-t"],
        vec!["uuid", "-v", "5"],
        vec!["uuid", "-c", "1001"],
        vec!["uuid", "text"],
        vec!["token", "-b", "0"],
        vec!["token", "--bits", "12"],
        vec!["token", "-l", "5", "-b", "5"],
        vec!["token", "--bits", "128", "--bytes", "16"],
        vec!["token", "-f", "alnum", "-b", "5"],
        vec!["token", "-f", "nope"],
        vec!["token", "-v", "4"],
        vec!["token", "--token"],
        vec!["token", "-c", "1001"],
        vec!["token", "text"],
        vec!["--bits", "128"],
        vec!["unknown"],
    ] {
        let mut args = vec!["random"];
        args.extend(extra);
        let r = dt(&args);
        assert_eq!(r.code, 2, "{args:?}");
        assert!(r.out.is_empty());
    }
    assert_eq!(dt(&["uuid"]).code, 2);
}

const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn ulid_ms(s: &str) -> u128 {
    s[..10]
        .chars()
        .fold(0u128, |acc, c| acc * 32 + CROCKFORD.find(c).unwrap() as u128)
}

#[test]
fn ulid_format_timestamp_and_monotonic_order() {
    let before = now();
    let v = lines(&["random", "ulid", "-c", "1000"]);
    let after = now();
    assert_eq!(v.len(), 1000);
    for s in &v {
        check(s, "^[0-9A-HJKMNP-TV-Z]{26}$");
        assert!(
            s.as_bytes()[0] <= b'7',
            "48-bit timestamp must not overflow 128 bits: {s}"
        );
    }
    assert!(ulid_ms(&v[0]) >= before - 1 && ulid_ms(&v[999]) <= after + 1000);
    assert!(v.windows(2).all(|w| w[0] < w[1]), "must be strictly increasing");
    assert_eq!(lines(&["random", "ulid"]).len(), 1);
}

#[test]
fn ulid_lowercase_and_json() {
    for s in lines(&["random", "ulid", "-c", "5", "--lowercase"]) {
        check(&s, "^[0-9a-hjkmnp-tv-z]{26}$");
    }
    let j: serde_json::Value = serde_json::from_str(&dt(&["random", "ulid", "--json", "-c", "3"]).out).unwrap();
    assert_eq!(j["type"], "ulid");
    assert_eq!(j.as_object().unwrap().len(), 2);
    let values = j["values"].as_array().unwrap();
    assert_eq!(values.len(), 3);
    check(values[0].as_str().unwrap(), "^[0-9A-Z]{26}$");
}

#[test]
fn nanoid_defaults_and_custom_alphabet() {
    let s = lines(&["random", "nanoid"]);
    assert_eq!(s.len(), 1);
    check(&s[0], "^[A-Za-z0-9_-]{21}$");
    let v = lines(&["random", "nanoid", "-l", "256", "-c", "3"]);
    assert_eq!(v.len(), 3);
    assert!(v.iter().all(|x| x.len() == 256));
    assert_ne!(v[0], v[1]);
    for s in lines(&["random", "nanoid", "-l", "40", "-c", "20", "--alphabet", "xyz!"]) {
        check(&s, "^[xyz!]{40}$");
    }
    // Largest possible alphabet: all 94 printable ASCII characters.
    let all: String = (0x21u8..=0x7e).map(char::from).collect();
    let r = dt(&["random", "nanoid", "-l", "200", "--alphabet", &all]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.out.trim_end().chars().all(|c| all.contains(c)));
}

#[test]
fn nanoid_covers_whole_alphabet() {
    // 3 symbols is a non-power-of-two size: masking+rejection must still reach every symbol.
    let out = lines(&["random", "nanoid", "-l", "256", "-c", "40", "--alphabet", "abc"]).concat();
    let mut counts = [0usize; 3];
    for c in out.chars() {
        counts[(c as u8 - b'a') as usize] += 1;
    }
    let total = out.len() as f64;
    for n in counts {
        let share = n as f64 / total;
        assert!((share - 1.0 / 3.0).abs() < 0.05, "{counts:?}");
    }
}

#[test]
fn nanoid_json_shape() {
    let j: serde_json::Value =
        serde_json::from_str(&dt(&["random", "nanoid", "--json", "-l", "8", "-c", "2"]).out).unwrap();
    assert_eq!(j["type"], "nanoid");
    assert_eq!(j.as_object().unwrap().len(), 2);
    assert_eq!(j["values"].as_array().unwrap().len(), 2);
    check(j["values"][1].as_str().unwrap(), "^[A-Za-z0-9_-]{8}$");
}

#[test]
fn ulid_nanoid_usage_errors_and_help() {
    for extra in [
        vec!["ulid", "-c", "0"],
        vec!["ulid", "-c", "1001"],
        vec!["ulid", "text"],
        vec!["ulid", "-l", "5"],
        vec!["nanoid", "-c", "0"],
        vec!["nanoid", "-c", "1001"],
        vec!["nanoid", "-l", "0"],
        vec!["nanoid", "-l", "257"],
        vec!["nanoid", "--alphabet", "a"],
        vec!["nanoid", "--alphabet", "aab"],
        vec!["nanoid", "--alphabet", "a b"],
        vec!["nanoid", "--alphabet", "aé"],
        vec!["nanoid", "--alphabet", ""],
        vec!["nanoid", "text"],
    ] {
        let mut args = vec!["random"];
        args.extend(extra);
        let r = dt(&args);
        assert_eq!(r.code, 2, "{args:?}");
        assert!(r.out.is_empty());
    }
    for sub in ["ulid", "nanoid"] {
        let r = dt(&["random", sub, "-h"]);
        assert_eq!(r.code, 0);
        assert!(r.out.starts_with(&format!("dt random {sub}")));
    }
    let top = dt(&["random", "-h"]).out;
    assert!(top.contains("ulid") && top.contains("nanoid"));
}
