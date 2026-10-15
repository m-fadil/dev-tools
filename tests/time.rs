mod common;
use common::{dt, dt_env};
use serde_json::{Value, json};
const NOW: &str = "2026-01-02T03:04:05Z";
fn doc(args: &[&str]) -> Value {
    let r = dt(args);
    assert_eq!(r.code, 0, "{}", r.err);
    serde_json::from_str(&r.out).unwrap()
}
fn utc(value: &str) -> String {
    doc(&["time", "-z", "UTC", "--json", "--", value])["utc"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn epoch_seconds_and_jakarta_text() {
    let r = dt(&["time", "-z", "Asia/Jakarta", "--now", NOW, "1767323045"]);
    assert_eq!(r.code, 0, "{}", r.err);
    let rows: Vec<_> = r
        .out
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    assert_eq!(
        rows,
        [
            "UTC: 2026-01-02T03:04:05.000Z",
            "Asia/Jakarta: 2026-01-02T10:04:05.000+07:00",
            "Epoch (s): 1767323045",
            "Epoch (ms): 1767323045000",
            "Relatif: sekarang"
        ]
    );
}
#[test]
fn units_detection_negative_and_boundary() {
    for (v, want) in [
        ("1767323045123", "2026-01-02T03:04:05.123Z"),
        ("99999999999", "5138-11-16T09:46:39.000Z"),
        ("100000000000", "1973-03-03T09:46:40.000Z"),
        ("-86400", "1969-12-31T00:00:00.000Z"),
    ] {
        assert_eq!(utc(v), want);
    }
    assert_eq!(
        doc(&["time", "-z", "UTC", "-u", "ms", "1000", "--json"])["utc"],
        "1970-01-01T00:00:01.000Z"
    );
    assert_eq!(
        doc(&["time", "-z", "UTC", "-u", "s", "1767323045", "--json"])["utc"],
        "2026-01-02T03:04:05.000Z"
    );
}
#[test]
fn iso_zones_and_offsets() {
    for (zone, value, want, iso) in [
        (
            "Asia/Jakarta",
            "2026-01-02T10:04:05",
            "2026-01-02T03:04:05.000Z",
            "2026-01-02T10:04:05.000+07:00",
        ),
        (
            "America/New_York",
            "2026-07-01T12:00:00",
            "2026-07-01T16:00:00.000Z",
            "2026-07-01T12:00:00.000-04:00",
        ),
        (
            "Asia/Tokyo",
            "2026-01-02T10:04:05.5+07:00",
            "2026-01-02T03:04:05.500Z",
            "2026-01-02T12:04:05.500+09:00",
        ),
        (
            "Asia/Jakarta",
            "2026-01-02",
            "2026-01-01T17:00:00.000Z",
            "2026-01-02T00:00:00.000+07:00",
        ),
        (
            "Europe/Berlin",
            "2026-01-15T12:00:00",
            "2026-01-15T11:00:00.000Z",
            "2026-01-15T12:00:00.000+01:00",
        ),
        (
            "Europe/Berlin",
            "2026-07-15T12:00:00",
            "2026-07-15T10:00:00.000Z",
            "2026-07-15T12:00:00.000+02:00",
        ),
        ("Asia/Kolkata", NOW, NOW, "2026-01-02T08:34:05.000+05:30"),
    ] {
        let d = doc(&["time", "-z", zone, "--json", value]);
        assert_eq!(d["zone"]["iso"], iso);
        assert_eq!(d["utc"], if want == NOW { "2026-01-02T03:04:05.000Z" } else { want });
        if zone == "Asia/Jakarta" && value.contains("10:04:05") {
            assert_eq!(d["epochSeconds"], 1767323045);
        }
    }
}
#[test]
fn relative_units_and_now_epoch() {
    for (v, expected) in [
        ("2026-01-02T00:04:05Z", "3 jam lalu"),
        ("2026-01-04T03:04:05Z", "2 hari lagi"),
        ("2026-01-02T03:03:05Z", "1 menit lalu"),
        ("2026-01-02T03:04:35Z", "30 detik lagi"),
        ("2026-03-04T03:04:05Z", "2 bulan lagi"),
        ("2023-01-02T03:04:05Z", "3 tahun lalu"),
    ] {
        assert_eq!(
            doc(&["time", "-z", "UTC", "--now", NOW, "--json", v])["relative"],
            expected
        );
    }
    assert_eq!(
        doc(&["time", "-z", "UTC", "--now", "1767323045", "1767326645", "--json"])["relative"],
        "1 jam lagi"
    );
    assert_eq!(doc(&["time", "-z", "UTC", "--json"])["relative"], "sekarang");
}
#[test]
fn invalid_inputs() {
    for args in [
        vec!["kemarin"],
        vec!["2026-02-30"],
        vec!["2026-13-01"],
        vec!["2026-01-02T24:00:00"],
        vec!["2026-01-02T10:00:00+25:00"],
        vec!["1.5"],
        vec!["9999999999999999"],
        vec!["-u", "s", "1767323045000000000"],
        vec!["-u", "us", "1"],
        vec!["-z", "Mars/Olympus", "1"],
        vec!["--now", "bukan-waktu", "1"],
        vec!["1", "2"],
    ] {
        let mut a = vec!["time"];
        a.extend(args);
        let r = dt(&a);
        assert_eq!(r.code, 2, "{a:?}: {}", r.err);
        assert!(r.out.is_empty());
        assert!(!r.err.is_empty());
    }
}
#[test]
fn json_shape_alias_and_system_zone() {
    let d = doc(&["time", "--json", "-z", "Asia/Jakarta", "--now", NOW, "1767323045"]);
    assert_eq!(
        d,
        json!({"input":"1767323045","utc":"2026-01-02T03:04:05.000Z","zone":{"name":"Asia/Jakarta","iso":"2026-01-02T10:04:05.000+07:00"},"epochSeconds":1767323045_i64,"epochMillis":1767323045000_i64,"relative":"sekarang"})
    );
    assert!(doc(&["time", "--json", "-z", "UTC", "--now", NOW])["input"].is_null());
    assert_eq!(dt(&["time", "--json", "bukan-waktu"]).code, 2);
    assert_eq!(dt(&["ts", "0", "--json"]).code, 0);
    assert_eq!(doc(&["ts", "0", "-z", "UTC", "--json"])["epochSeconds"], 0);
    let r = dt_env(&["time", "--json", "0"], b"", &[("TZ", "Asia/Jakarta")]);
    assert_eq!(r.code, 0, "{}", r.err);
    let d: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(d["zone"]["name"], "Asia/Jakarta");
}
#[test]
fn full_date_range() {
    assert_eq!(utc("8640000000000000"), "+275760-09-13T00:00:00.000Z");
    for v in ["8640000000000001", "-8640000000000001"] {
        assert_eq!(dt(&["time", "-z", "UTC", "--", v]).code, 2);
    }
    let offset_suffix = regex::Regex::new(r"[+-]\d{2}:\d{2}(?::\d{2})?$").unwrap();
    for (zone, value, want) in [
        ("Asia/Jakarta", "+275760-09-13T00:00:00Z", "+275760-09-13T00:00:00.000Z"),
        ("America/New_York", "-8640000000000000", "-271821-04-20T00:00:00.000Z"),
    ] {
        let d = doc(&["time", "-z", zone, "--now", NOW, "--json", "--", value]);
        assert_eq!(d["utc"], want);
        let iso = d["zone"]["iso"].as_str().unwrap();
        assert!(!iso.contains("NaN"));
        assert!(offset_suffix.is_match(iso));
        if zone == "Asia/Jakarta" {
            assert_eq!(iso, "+275760-09-13T07:00:00.000+07:00");
        }
        let text = dt(&["time", "-z", zone, "--now", NOW, "--", value]);
        assert_eq!(text.code, 0, "{}", text.err);
        assert!(text.out.contains(iso) && text.out.contains(want));
    }
}
#[test]
fn dst_gap_and_overlap() {
    for (zone, value, utc, iso) in [
        (
            "America/New_York",
            "2026-03-08T02:30",
            "2026-03-08T07:30:00.000Z",
            "2026-03-08T03:30:00.000-04:00",
        ),
        (
            "Europe/Berlin",
            "2026-03-29T02:30",
            "2026-03-29T01:30:00.000Z",
            "2026-03-29T03:30:00.000+02:00",
        ),
        (
            "America/New_York",
            "2026-11-01T01:30",
            "2026-11-01T05:30:00.000Z",
            "2026-11-01T01:30:00.000-04:00",
        ),
        (
            "Europe/Berlin",
            "2026-10-25T02:30",
            "2026-10-25T00:30:00.000Z",
            "2026-10-25T02:30:00.000+02:00",
        ),
    ] {
        let d = doc(&["time", "-z", zone, "--json", value]);
        assert_eq!(d["utc"], utc);
        assert_eq!(d["zone"]["iso"], iso);
    }
}
#[test]
fn unit_applies_to_now_and_rejects_iso() {
    for args in [
        vec!["-u", "ms", "2026-01-02"],
        vec!["-u", "ms", "--now", NOW],
        vec!["-u", "ms", "--now", NOW, "1000"],
        vec!["-u", "ms", "--now", "5000", NOW],
        vec!["-u", "ms"],
    ] {
        let mut a = vec!["time"];
        a.extend(args);
        let r = dt(&a);
        assert_eq!(r.code, 2);
        assert!(r.out.is_empty());
        assert!(r.err.contains("--unit"));
    }
    for (unit, now, value) in [("ms", "5000", "1000"), ("s", "5", "1")] {
        let d = doc(&["time", "-z", "UTC", "-u", unit, "--now", now, value, "--json"]);
        assert_eq!(d["relative"], "4 detik lalu");
        assert_eq!(d["epochMillis"], 1000);
    }
    let d = doc(&["time", "-z", "UTC", "-u", "ms", "--now", "5000", "--json"]);
    assert_eq!(d["epochMillis"], 5000);
    assert_eq!(d["relative"], "sekarang");
}

#[test]
fn fixed_legacy_icu_timezone_database_vectors() {
    assert_eq!(jiff_tzdb::VERSION, Some("2026c"));
    for (zone, value, iso, millis) in [
        (
            "America/New_York",
            "2100-07-01T12:00:00",
            "2100-07-01T12:00:00.000-04:00",
            4_118_140_800_000_i64,
        ),
        (
            "Europe/Berlin",
            "2400-07-01T12:00:00",
            "2400-07-01T12:00:00.000+02:00",
            13_585_226_400_000,
        ),
        (
            "America/New_York",
            "+275760-07-01T12:00:00Z",
            "+275760-07-01T08:00:00.000-04:00",
            8_639_993_649_600_000,
        ),
        (
            "America/New_York",
            "-8640000000000000",
            "-271821-04-19T19:03:58.000-04:56:02",
            -8_640_000_000_000_000,
        ),
        (
            "Africa/Casablanca",
            "+275760-07-01T12:00:00Z",
            "+275760-07-01T12:00:00.000+00:00",
            8_639_993_649_600_000,
        ),
        (
            "Africa/Casablanca",
            "2026-02-15T01:59:59Z",
            "2026-02-15T02:59:59.000+01:00",
            1_771_120_799_000,
        ),
        (
            "Africa/Casablanca",
            "2026-02-15T02:00:00Z",
            "2026-02-15T02:00:00.000+00:00",
            1_771_120_800_000,
        ),
        (
            "Africa/Casablanca",
            "2026-03-22T01:59:59Z",
            "2026-03-22T01:59:59.000+00:00",
            1_774_144_799_000,
        ),
        (
            "Africa/Casablanca",
            "2026-03-22T02:00:00Z",
            "2026-03-22T03:00:00.000+01:00",
            1_774_144_800_000,
        ),
        (
            "Africa/Casablanca",
            "2037-01-01T00:00:00Z",
            "2037-01-01T00:00:00.000+00:00",
            2_114_380_800_000,
        ),
        (
            "Africa/Casablanca",
            "2037-07-01T12:00:00Z",
            "2037-07-01T12:00:00.000+00:00",
            2_130_062_400_000,
        ),
        (
            "Africa/Casablanca",
            "2099-01-01T00:00:00Z",
            "2099-01-01T00:00:00.000+00:00",
            4_070_908_800_000,
        ),
        (
            "Africa/Casablanca",
            "2099-07-01T12:00:00Z",
            "2099-07-01T12:00:00.000+00:00",
            4_086_590_400_000,
        ),
        (
            "SystemV/EST5EDT",
            "2026-04-26T06:59:59Z",
            "2026-04-26T01:59:59.000-05:00",
            1_777_186_799_000,
        ),
        (
            "SystemV/EST5EDT",
            "2026-04-26T07:00:00Z",
            "2026-04-26T03:00:00.000-04:00",
            1_777_186_800_000,
        ),
        (
            "SystemV/EST5EDT",
            "1901-07-01T12:00:00Z",
            "1901-07-01T07:00:00.000-05:00",
            -2_161_771_200_000,
        ),
    ] {
        let d = doc(&["time", "-z", zone, "--json", "--", value]);
        assert_eq!(d["zone"]["iso"], iso, "{zone}: {value}");
        assert_eq!(d["epochMillis"], millis, "{zone}: {value}");
    }
}

#[test]
fn intl_timezone_names_offsets_and_extended_iso_forms() {
    for zone in ["+07", "+0700", "+07:00", "asia/jakarta"] {
        let d = doc(&["time", "-z", zone, "--json", "2026-01-02T10:04:05"]);
        assert_eq!(d["epochSeconds"], 1767323045);
        assert_eq!(d["zone"]["name"], zone);
    }
    for zone in ["Factory", "Etc/Unknown", "UTC0", "+07::", "+25:00"] {
        assert_eq!(dt(&["time", "-z", zone, "0"]).code, 2, "{zone}");
    }
    for (value, want) in [
        ("0000-01-01T00:00:00Z", "0000-01-01T00:00:00.000Z"),
        ("0050-01-01T00:00:00Z", "0050-01-01T00:00:00.000Z"),
        ("2026-01-02T10:04:05.123456789+0700", "2026-01-02T03:04:05.123Z"),
    ] {
        assert_eq!(utc(value), want);
    }
    assert_eq!(dt(&["time", "-z", "UTC", "--", "-000000-01-01"]).code, 2);
}

#[test]
fn system_zone_aliases_and_javascript_whitespace() {
    for (tz, name) in [
        ("Etc/UTC", "UTC"),
        ("US/Eastern", "America/New_York"),
        ("CET", "Europe/Brussels"),
        ("EST", "America/Panama"),
        ("GMT", "+00:00"),
    ] {
        let r = dt_env(&["time", "--json", "0"], b"", &[("TZ", tz)]);
        assert_eq!(r.code, 0, "{}", r.err);
        let d: Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(d["zone"]["name"], name);
    }
    assert_eq!(utc("\u{feff}0\u{feff}"), "1970-01-01T00:00:00.000Z");
    assert_eq!(dt(&["time", "-z", "UTC", "\u{85}0"]).code, 2);
}

#[test]
fn malformed_and_empty_system_timezone_overrides() {
    for tz in ["Invalid/Zone", "UTC0"] {
        let r = dt_env(&["time", "--json", "--now", NOW, "0"], b"", &[("TZ", tz)]);
        assert_eq!(r.code, 0, "{}", r.err);
        let d: Value = serde_json::from_str(&r.out).unwrap();
        assert_eq!(d["zone"], json!({"iso":"1970-01-01T00:00:00.000+00:00"}));
        let r = dt_env(&["time", "0"], b"", &[("TZ", tz)]);
        assert_eq!(r.code, 1);
        assert_eq!(r.out, "");
        assert_eq!(r.err, "Cannot read properties of undefined (reading 'length')\n");
    }
    for args in [vec!["time", "--json", "0"], vec!["time", "0"]] {
        let r = dt_env(&args, b"", &[("TZ", "")]);
        assert_eq!(r.code, 1);
        assert_eq!(r.out, "");
        assert_eq!(r.err, "Invalid time zone specified: Etc/Unknown\n");
    }
    let explicit = dt_env(&["time", "-z", "UTC", "--json", "0"], b"", &[("TZ", "")]);
    assert_eq!(explicit.code, 0, "{}", explicit.err);
    let default = doc(&["time", "--json", "0"]);
    let r = dt_env(&["time", "--json", "0"], b"", &[("TZ", "SystemV/EST5EDT")]);
    assert_eq!(r.code, 0, "{}", r.err);
    let d: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(d["zone"], default["zone"]);
}
