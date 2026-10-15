mod common;
use common::dt;
use std::collections::HashSet;

fn values(args: &[&str]) -> Vec<String> {
    let r = dt(args);
    assert_eq!(r.code, 0, "{}", r.err);
    r.out.lines().map(str::to_owned).collect()
}
fn matches(value: &str, pattern: &str) {
    assert!(regex::Regex::new(pattern).unwrap().is_match(value), "{value}");
}

#[test]
fn defaults_classes_and_boundaries() {
    let passwords = values(&["password", "-c", "50"]);
    assert_eq!(passwords.len(), 50);
    assert_eq!(passwords.iter().collect::<HashSet<_>>().len(), 50);
    for p in passwords {
        matches(&p, "^[A-Za-z0-9]{20}$");
    }
    for symbols in [false, true] {
        let mut args = vec!["password", "-l", "4", "-c", "300"];
        if symbols {
            args.push("-s");
        }
        for p in values(&args) {
            assert_eq!(p.len(), 4);
            for pattern in ["[a-z]", "[A-Z]", "[0-9]"] {
                matches(&p, pattern);
            }
            if symbols {
                assert!(p.chars().any(|c| "!@#$%^&*()-_=+[]{};:,.?".contains(c)));
            }
        }
    }
    assert_eq!(values(&["password", "-l", "1024"])[0].len(), 1024);
    assert_eq!(values(&["password", "-c", "1000"]).len(), 1000);
    for bad in ["3", "1025", "0", "-5", "1.5", "abc", ""] {
        assert_eq!(dt(&["password", "-l", bad]).code, 2);
    }
    for bad in ["0", "1001"] {
        assert_eq!(dt(&["password", "-c", bad]).code, 2);
    }
}
#[test]
fn restricted_alphabets_and_unambiguous() {
    for (set, pattern) in [
        ("lower", "^[a-z]{40}$"),
        ("upper", "^[A-Z]{40}$"),
        ("digits", "^[0-9]{40}$"),
        ("hex", "^[0-9a-f]{40}$"),
        ("alnum", "^[A-Za-z0-9]{40}$"),
    ] {
        for p in values(&["password", "--only", set, "-l", "40", "-c", "100"]) {
            matches(&p, pattern);
            if set == "alnum" {
                for pat in ["[a-z]", "[A-Z]", "[0-9]"] {
                    matches(&p, pat);
                }
            }
        }
    }
    for p in values(&["password", "-u", "-s", "-l", "200", "-c", "50"]) {
        assert!(!p.chars().any(|c| "0O1lI|".contains(c)));
    }
    for (set, pattern) in [("hex", "^[2-9a-f]{100}$"), ("digits", "^[2-9]{100}$")] {
        matches(&values(&["password", "-u", "--only", set, "-l", "100"])[0], pattern);
    }
    for args in [
        vec!["password", "--only", "hex", "-s"],
        vec!["password", "--only", "emoji"],
        vec!["password", "extra"],
    ] {
        let r = dt(&args);
        assert_eq!(r.code, 2);
        assert!(r.out.is_empty());
    }
    assert!(
        dt(&["password", "--only", "emoji"])
            .err
            .contains("--only tidak valid: emoji")
    );
}
#[test]
fn entropy_and_json() {
    for (extra, expected) in [
        (vec!["-e"], 119),
        (vec!["-e", "-s"], 128),
        (vec!["-e", "-u"], 117),
        (vec!["-e", "--only", "hex", "-l", "32"], 128),
    ] {
        let mut args = vec!["password"];
        args.extend(extra);
        let r = dt(&args);
        assert_eq!(r.code, 0);
        assert_eq!(r.err.trim(), format!("Entropi: ~{expected} bit"));
    }
    assert!(dt(&["password"]).err.is_empty());
    let r = dt(&["password", "--json", "-c", "3", "-l", "12"]);
    let j: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(j.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["values"]);
    assert_eq!(j["values"].as_array().unwrap().len(), 3);
    for v in j["values"].as_array().unwrap() {
        assert_eq!(v.as_str().unwrap().len(), 12);
    }
    let r = dt(&["password", "--json", "-e", "--only", "hex", "-l", "16"]);
    assert!(r.err.is_empty());
    let j: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(j["entropyBits"], 64);
}
