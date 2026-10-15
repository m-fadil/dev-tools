mod common;
use common::{Out, dt, dt_env};
use serde_json::{Value, json};
fn run(expr: &[&str], from: &str, n: &str) -> Out {
    let mut a = vec!["cron", "--utc", "--from", from, "-n", n];
    a.extend(expr);
    dt(&a)
}
fn times(out: &str) -> Vec<&str> {
    out.lines().filter(|s| s.as_bytes().get(4) == Some(&b'-')).collect()
}
#[test]
fn steps_ranges_names_and_explanation() {
    let r = run(&["*/15 9-10 * * mon-fri"], "2025-01-03T10:30:00Z", "4");
    assert_eq!(r.code, 0, "{}", r.err);
    for line in [
        "menit: 0, 15, 30, 45",
        "jam: 9, 10",
        "hari: Senin, Selasa, Rabu, Kamis, Jumat",
    ] {
        assert!(r.out.lines().any(|l| l == line));
    }
    assert_eq!(
        times(&r.out),
        [
            "2025-01-03T10:45:00Z",
            "2025-01-06T09:00:00Z",
            "2025-01-06T09:15:00Z",
            "2025-01-06T09:30:00Z"
        ]
    );
}
#[test]
fn separate_fields_case_and_macros() {
    let a = run(&["0", "12", "1", "JAN", "*"], "2025-01-01T00:00:00Z", "2");
    let b = run(&["0 12 1 jan *"], "2025-01-01T00:00:00Z", "2");
    assert_eq!(a.out, b.out);
    assert_eq!(times(&a.out), ["2025-01-01T12:00:00Z", "2026-01-01T12:00:00Z"]);
    assert_eq!(
        times(&run(&["@HOURLY"], "2025-01-01T00:00:00Z", "2").out),
        ["2025-01-01T01:00:00Z", "2025-01-01T02:00:00Z"]
    );
    assert_eq!(
        times(&run(&["@monthly"], "2025-01-31T23:59:59Z", "2").out),
        ["2025-02-01T00:00:00Z", "2025-03-01T00:00:00Z"]
    );
}
#[test]
fn exclusive_start_sunday_and_open_steps() {
    assert_eq!(
        times(&run(&["*/20 10 * * *"], "2025-01-01T10:00:30Z", "2").out),
        ["2025-01-01T10:20:00Z", "2025-01-01T10:40:00Z"]
    );
    assert_eq!(
        times(&run(&["0 10 * * *"], "2025-01-01T10:00:00Z", "1").out),
        ["2025-01-02T10:00:00Z"]
    );
    let a = run(&["0 0 * * 0"], "2025-01-01T00:00:00Z", "2");
    assert_eq!(times(&a.out), ["2025-01-05T00:00:00Z", "2025-01-12T00:00:00Z"]);
    assert_eq!(a.out, run(&["0 0 * * 7"], "2025-01-01T00:00:00Z", "2").out);
    assert_eq!(
        times(&run(&["50/5 0 1 1 *"], "2025-01-01T00:00:00Z", "5").out),
        [
            "2025-01-01T00:50:00Z",
            "2025-01-01T00:55:00Z",
            "2026-01-01T00:50:00Z",
            "2026-01-01T00:55:00Z",
            "2027-01-01T00:50:00Z"
        ]
    );
}
#[test]
fn day_field_or_rule() {
    assert_eq!(
        times(&run(&["0 0 13 * fri"], "2025-06-01T00:00:00Z", "4").out),
        [
            "2025-06-06T00:00:00Z",
            "2025-06-13T00:00:00Z",
            "2025-06-20T00:00:00Z",
            "2025-06-27T00:00:00Z"
        ]
    );
    assert_eq!(
        times(&run(&["0 0 13 * *"], "2025-06-01T00:00:00Z", "2").out),
        ["2025-06-13T00:00:00Z", "2025-07-13T00:00:00Z"]
    );
}
#[test]
fn leap_day_horizon_and_impossible_date() {
    assert_eq!(
        times(&run(&["0 0 29 2 *"], "2096-01-01T00:00:00Z", "3").out),
        ["2096-02-29T00:00:00Z", "2104-02-29T00:00:00Z", "2108-02-29T00:00:00Z"]
    );
    assert_eq!(
        times(&run(&["0 0 29 2 *"], "2100-01-01T00:00:00Z", "1").out),
        ["2104-02-29T00:00:00Z"]
    );
    assert_eq!(
        times(&run(&["0 0 29 2 *"], "2096-03-01T00:00:00Z", "2").out),
        ["2104-02-29T00:00:00Z", "2108-02-29T00:00:00Z"]
    );
    let r = run(&["0 0 31 2 *"], "2025-01-01T00:00:00Z", "5");
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Tidak ada waktu yang cocok dalam 8 tahun"));
    assert!(times(&r.out).is_empty());
}
#[test]
fn invalid_fields_and_options() {
    for (expr, field, reason) in [
        ("60 * * * *", "menit", ""),
        ("* 24 * * *", "jam", ""),
        ("* * 0 * *", "tanggal", ""),
        ("* * * 13 *", "bulan", ""),
        ("* * * * 8", "hari", ""),
        ("* * * foo *", "bulan", "foo"),
        ("5-1 * * * *", "menit", "terbalik"),
        ("*/0 * * * *", "menit", "step"),
        ("1,,2 * * * *", "menit", "kosong"),
        ("1-2-3 * * * *", "menit", ""),
        ("* * * * fri-sun", "hari", "terbalik"),
    ] {
        let r = dt(&["cron", expr]);
        assert_eq!(r.code, 2, "{expr}");
        assert!(
            r.err.contains(&format!("Field {field} tidak valid")) && r.err.contains(reason),
            "{}",
            r.err
        );
    }
    for expr in ["* * * *", "* * * * * *"] {
        assert!(dt(&["cron", expr]).err.contains("harus 5 field"));
    }
    assert!(dt(&["cron", "@reboot"]).err.contains("Makro tidak dikenal"));
    assert_eq!(dt(&["cron"]).code, 2);
    for n in ["0", "51"] {
        assert_eq!(dt(&["cron", "-n", n, "* * * * *"]).code, 2);
    }
    for from in ["2025-02-30", "besok", "2025-01-01T24:00:00Z"] {
        let r = dt(&["cron", "--from", from, "* * * * *"]);
        assert_eq!(r.code, 2);
        assert!(r.err.contains("--from"));
    }
}
#[test]
fn explicit_offset_and_default_count() {
    let r = dt(&["cron", "--utc", "--from", "2025-01-01T07:00:00+07:00", "0 * * * *"]);
    assert_eq!(
        times(&r.out),
        [
            "2025-01-01T01:00:00Z",
            "2025-01-01T02:00:00Z",
            "2025-01-01T03:00:00Z",
            "2025-01-01T04:00:00Z",
            "2025-01-01T05:00:00Z"
        ]
    );
}
#[test]
fn local_dst_gap_overlap_and_repeated_hour() {
    for (from, expected) in [
        (
            "2025-03-28T00:00",
            [
                "2025-03-28T02:30:00+01:00",
                "2025-03-29T02:30:00+01:00",
                "2025-03-31T02:30:00+02:00",
            ],
        ),
        (
            "2025-10-25T00:00",
            [
                "2025-10-25T02:30:00+02:00",
                "2025-10-26T02:30:00+02:00",
                "2025-10-27T02:30:00+01:00",
            ],
        ),
        (
            "2025-10-26T02:10:00+01:00",
            [
                "2025-10-27T02:30:00+01:00",
                "2025-10-28T02:30:00+01:00",
                "2025-10-29T02:30:00+01:00",
            ],
        ),
    ] {
        let r = dt_env(
            &["cron", "--from", from, "-n", "3", "30 2 * * *"],
            b"",
            &[("TZ", "Europe/Berlin")],
        );
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(times(&r.out), expected);
    }
}
#[test]
fn json_document_and_exhaustion() {
    let r = run(&["--json", "30 9 * * mon"], "2025-01-01T00:00:00Z", "2");
    assert_eq!(r.code, 0, "{}", r.err);
    let d: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(
        d,
        json!({"expression":"30 9 * * mon","fields":{"minute":[30],"hour":[9],"dayOfMonth":(1..=31).collect::<Vec<_>>(),"month":(1..=12).collect::<Vec<_>>(),"dayOfWeek":[1]},"from":"2025-01-01T00:00:00Z","timezone":"utc","next":["2025-01-06T09:30:00Z","2025-01-13T09:30:00Z"]})
    );
    let r = run(&["--json", "0 0 31 2 *"], "2025-01-01T00:00:00Z", "5");
    assert_eq!(r.code, 1);
    assert_eq!(serde_json::from_str::<Value>(&r.out).unwrap()["next"], json!([]));
}
#[test]
fn early_years_preserve_zero_to_ninety_nine() {
    for (from, expected) in [
        ("0000-02-28", ["0000-02-29T00:00:00Z", "0000-03-01T00:00:00Z"]),
        ("0050-01-01", ["0050-01-02T00:00:00Z", "0050-01-03T00:00:00Z"]),
        ("0050-01-01T00:00:00Z", ["0050-01-02T00:00:00Z", "0050-01-03T00:00:00Z"]),
        (
            "0050-01-01T07:00:00+07:00",
            ["0050-01-02T00:00:00Z", "0050-01-03T00:00:00Z"],
        ),
        ("0099-12-31", ["0100-01-01T00:00:00Z", "0100-01-02T00:00:00Z"]),
    ] {
        let r = run(&["@daily"], from, "2");
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(times(&r.out), expected);
    }
    assert_eq!(
        times(&run(&["@monthly"], "0050-01-01", "2").out),
        ["0050-02-01T00:00:00Z", "0050-03-01T00:00:00Z"]
    );
    assert_eq!(
        times(&run(&["0 0 1 3 *"], "0050-01-01", "1").out),
        ["0050-03-01T00:00:00Z"]
    );
    let r = dt_env(
        &["cron", "--from", "0050-01-01", "-n", "2", "@daily"],
        b"",
        &[("TZ", "UTC")],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(
        times(&r.out),
        ["0050-01-02T00:00:00+00:00", "0050-01-03T00:00:00+00:00"]
    );
    let r = run(&["--json", "@daily"], "0050-01-01", "1");
    let d: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(d["from"], "0050-01-01T00:00:00Z");
    assert_eq!(d["next"], json!(["0050-01-02T00:00:00Z"]));
    assert_eq!(run(&["@daily"], "0050-02-29", "1").code, 2);
}

#[test]
fn javascript_whitespace_field_separation() {
    assert_eq!(
        run(&["\u{feff}0\u{feff}0 * * *\u{feff}"], "2025-01-01T00:00:00Z", "1").out,
        run(&["@daily"], "2025-01-01T00:00:00Z", "1").out
    );
}

#[test]
fn malformed_and_empty_native_timezone_overrides() {
    for tz in ["Invalid/Zone", "", "UTC0"] {
        let r = dt_env(
            &["cron", "--from", "2025-01-01", "-n", "1", "@daily"],
            b"",
            &[("TZ", tz)],
        );
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(times(&r.out), ["2025-01-02T00:00:00+00:00"]);
    }
}

#[test]
fn native_timezone_env_is_case_sensitive_and_posix_signed() {
    for (tz, offset) in [
        ("asia/jakarta", "+00:00"),
        ("EUROPE/BERLIN", "+00:00"),
        ("Factory", "+00:00"),
        ("Etc/Unknown", "+00:00"),
        ("SystemV/PST8", "-08:00"),
        ("GMT+7", "-07:00"),
        ("GMT-3", "+03:00"),
        ("Europe/Berlin", "+01:00"),
    ] {
        let r = dt_env(
            &["cron", "--from", "2025-01-01", "-n", "1", "@daily"],
            b"",
            &[("TZ", tz)],
        );
        assert_eq!(r.code, 0, "{tz}: {}", r.err);
        assert_eq!(times(&r.out), [format!("2025-01-02T00:00:00{offset}")], "{tz}");
    }
}
