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
