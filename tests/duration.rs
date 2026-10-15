mod common;
use common::dt;
use serde_json::Value;

/// Runs `dt duration <args>` successfully and returns the four text rows' values.
fn rows(args: &[&str]) -> [String; 4] {
    let mut full = vec!["duration"];
    full.extend_from_slice(args);
    let r = dt(&full);
    assert_eq!(r.code, 0, "{args:?}: {}", r.err);
    let values: Vec<String> = r
        .out
        .lines()
        .map(|l| l.split_once(':').unwrap().1.trim().to_string())
        .collect();
    values
        .try_into()
        .unwrap_or_else(|v: Vec<String>| panic!("{args:?}: {v:?}"))
}

fn single(args: &[&str]) -> String {
    let mut full = vec!["duration"];
    full.extend_from_slice(args);
    let r = dt(&full);
    assert_eq!(r.code, 0, "{args:?}: {}", r.err);
    r.out.trim_end().to_string()
}

fn fails(args: &[&str], code: i32, needle: &str) {
    let mut full = vec!["duration"];
    full.extend_from_slice(args);
    let r = dt(&full);
    assert_eq!(r.code, code, "{args:?}: {}", r.err);
    assert_eq!(r.out, "", "{args:?}");
    assert!(r.err.contains(needle), "{args:?}: {}", r.err);
}

fn json(args: &[&str]) -> Value {
    let mut full = vec!["duration"];
    full.extend_from_slice(args);
    let r = dt(&full);
    assert_eq!(r.code, 0, "{args:?}: {}", r.err);
    serde_json::from_str(&r.out).unwrap()
}

#[test]
fn text_output_has_four_labelled_rows() {
    let r = dt(&["duration", "1h30m"]);
    assert_eq!(r.code, 0);
    assert_eq!(
        r.out,
        "Ringkas:   1h 30m\nISO-8601:  PT1H30M\nDetik:     5400\nMilidetik: 5400000\n"
    );
    assert_eq!(r.err, "");
}

#[test]
fn every_compact_syntax_normalizes() {
    let cases: &[(&[&str], &str, &str, &str)] = &[
        (&["1h30m"], "1h 30m", "PT1H30M", "5400000"),
        (&["1d", "2h"], "1d 2h", "P1DT2H", "93600000"),
        (&["1d 2h"], "1d 2h", "P1DT2H", "93600000"),
        (&["90m"], "1h 30m", "PT1H30M", "5400000"),
        (&["1w2d"], "1w 2d", "P9D", "777600000"),
        (&["1.5h"], "1h 30m", "PT1H30M", "5400000"),
        (&["250ms"], "250ms", "PT0.25S", "250"),
        (&["90"], "1m 30s", "PT1M30S", "90000"),
        (&["1.5"], "1s 500ms", "PT1.5S", "1500"),
        (&["30m1h"], "1h 30m", "PT1H30M", "5400000"),
        (&["1m", "1s", "1ms"], "1m 1s 1ms", "PT1M1.001S", "61001"),
        (&["24h"], "1d", "P1D", "86400000"),
        (&["7d"], "1w", "P1W", "604800000"),
        (&["0"], "0s", "PT0S", "0"),
        (&["0h"], "0s", "PT0S", "0"),
        (&["+5s"], "5s", "PT5S", "5000"),
    ];
    for (args, compact, iso, ms) in cases {
        let r = rows(args);
        assert_eq!((&r[0][..], &r[1][..], &r[3][..]), (*compact, *iso, *ms), "{args:?}");
    }
}

#[test]
fn iso_syntax_and_round_trip() {
    let cases = [
        ("PT1H30M", "1h 30m", "PT1H30M"),
        ("P1DT2H", "1d 2h", "P1DT2H"),
        ("P2W", "2w", "P2W"),
        ("P14D", "2w", "P2W"),
        ("PT90M", "1h 30m", "PT1H30M"),
        ("PT0.5S", "500ms", "PT0.5S"),
        ("PT0,5S", "500ms", "PT0.5S"),
        ("P0.5D", "12h", "PT12H"),
        ("pt1h30m", "1h 30m", "PT1H30M"),
        ("P1W2DT3H4M5.678S", "1w 2d 3h 4m 5s 678ms", "P9DT3H4M5.678S"),
        ("PT0S", "0s", "PT0S"),
        ("-P1D", "-1d", "-P1D"),
    ];
    for (input, compact, iso) in cases {
        let r = rows(&[input]);
        assert_eq!((&r[0][..], &r[1][..]), (compact, iso), "{input}");
        // The emitted ISO string and compact string parse back to the same value.
        assert_eq!(rows(&[iso]), r, "iso round trip {input}");
        assert_eq!(rows(&[compact]), r, "compact round trip {input}");
    }
}

#[test]
fn millisecond_precision() {
    let r = rows(&["1m1s1ms"]);
    assert_eq!(r[2], "61.001");
    assert_eq!(r[3], "61001");
    assert_eq!(rows(&["0.001s"])[3], "1");
    assert_eq!(rows(&["1.001"])[3], "1001");
    assert_eq!(single(&["1ms", "--unit", "s"]), "0.001");
}

#[test]
fn negative_values() {
    assert_eq!(
        dt(&["duration", "-1h30m"]).out,
        "Ringkas:   -1h 30m\nISO-8601:  -PT1H30M\nDetik:     -5400\nMilidetik: -5400000\n"
    );
    assert_eq!(rows(&["--", "-90m"]), rows(&["-90m"]));
    assert_eq!(rows(&["-PT5M"])[3], "-300000");
    assert_eq!(rows(&["-45"])[3], "-45000");
    assert_eq!(single(&["-1.5h", "--unit", "m"]), "-90");
    assert_eq!(single(&["-90m", "-u", "h"]), "-1.5");
    // Negative zero collapses to a plain zero.
    assert_eq!(rows(&["-0s"]), ["0s", "PT0S", "0", "0"].map(String::from));
}

#[test]
fn unit_prints_exact_totals() {
    assert_eq!(single(&["1h30m", "--unit", "h"]), "1.5");
    assert_eq!(single(&["1h30m", "-u", "m"]), "90");
    assert_eq!(single(&["1h30m", "--unit", "s"]), "5400");
    assert_eq!(single(&["1h30m", "--unit", "ms"]), "5400000");
    assert_eq!(single(&["1h30m", "--unit", "d"]), "0.0625");
    assert_eq!(single(&["1h30m", "--unit", "w"]), "0.008928571");
    assert_eq!(single(&["2w", "--unit", "d"]), "14");
    assert_eq!(single(&["0.1s", "--unit", "ms"]), "100");
    assert_eq!(single(&["1d", "2h", "--unit", "h"]), "26");
    // A float would print 0.30000000000000004 or similar; integers give the exact value.
    assert_eq!(single(&["300ms", "--unit", "s"]), "0.3");
    assert_eq!(single(&["1h", "--unit", "ms"]), "3600000");
    // Non-terminating results round to 9 digits, half away from zero.
    assert_eq!(single(&["1ms", "--unit", "m"]), "0.000016667");
    assert_eq!(single(&["-1ms", "--unit", "m"]), "-0.000016667");
    assert_eq!(single(&["1ms", "--unit", "h"]), "0.000000278");
    // Large values keep every digit.
    assert_eq!(
        single(&["9223372036854775807ms", "--unit", "ms"]),
        "9223372036854775807"
    );
}

#[test]
fn accepted_limits() {
    let r = rows(&["9223372036854775807ms"]);
    assert_eq!(r[0], "15250284452w 3d 7h 12m 55s 807ms");
    assert_eq!(r[1], "P106751991167DT7H12M55.807S");
    assert_eq!(rows(&["-9223372036854775807ms"])[3], "-9223372036854775807");
}

#[test]
fn json_shape() {
    let v = json(&["1h30m", "--json"]);
    let object = v.as_object().unwrap();
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(keys, ["input", "sign", "compact", "iso", "seconds", "milliseconds"]);
    assert_eq!(v["input"], "1h30m");
    assert_eq!(v["sign"], 1);
    assert_eq!(v["compact"], "1h 30m");
    assert_eq!(v["iso"], "PT1H30M");
    assert_eq!(v["seconds"], 5400);
    assert_eq!(v["milliseconds"], 5_400_000);

    let v = json(&["-250ms", "--json"]);
    assert_eq!(v["sign"], -1);
    assert_eq!(v["seconds"].to_string(), "-0.25");
    assert_eq!(v["milliseconds"], -250);
    assert_eq!(v["iso"], "-PT0.25S");

    assert_eq!(json(&["0", "--json"])["sign"], 0);
    // Arguments are joined before parsing, and the joined text is what `input` echoes.
    assert_eq!(json(&["1d", "2h", "--json"])["input"], "1d 2h");
}

#[test]
fn json_with_unit_adds_unit_and_value() {
    let v = json(&["1h30m", "--json", "--unit", "h"]);
    assert_eq!(v["unit"], "h");
    assert_eq!(v["value"].to_string(), "1.5");
    assert_eq!(v["milliseconds"], 5_400_000);
    assert!(json(&["1h", "--json"]).get("unit").is_none());
}

#[test]
fn options_may_follow_or_precede_the_value() {
    assert_eq!(single(&["--unit", "m", "1h"]), "60");
    assert_eq!(single(&["1h", "--unit=m"]), "60");
    assert_eq!(json(&["--json", "-90m"])["milliseconds"], -5_400_000);
    assert_eq!(single(&["-90m", "-u", "m"]), "-90");
}

#[test]
fn years_and_months_are_rejected() {
    for input in [
        "1y",
        "2mo",
        "1M",
        "3months",
        "1year",
        "P1Y",
        "P1M",
        "P1Y2M3DT4H",
        "1y2d",
    ] {
        fails(&[input], 2, "tahun dan bulan");
    }
    // `M` after `T` is minutes, and lowercase `m` is minutes.
    assert_eq!(rows(&["PT1M"])[0], "1m");
    assert_eq!(rows(&["1m"])[0], "1m");
}

#[test]
fn overflow_is_rejected() {
    fails(&["9223372036854775808ms"], 2, "terlalu besar");
    fails(&["999999999999999999999999999999999999999999d"], 2, "terlalu besar");
    fails(&["9223372036854775807ms", "1s"], 2, "terlalu besar");
    fails(&["9223372036854775807ms1s"], 2, "terlalu besar");
    fails(&["15250284453w"], 2, "terlalu besar");
    fails(&["P106751991168D"], 2, "terlalu besar");
    fails(&["-9223372036854775808ms"], 2, "terlalu besar");
}

#[test]
fn sub_millisecond_precision_is_rejected() {
    fails(&["0.0005s"], 2, "Presisi");
    fails(&["1.5ms"], 2, "Presisi");
    fails(&["0.3333333h"], 2, "Presisi");
    fails(&["PT0.0001S"], 2, "Presisi");
    fails(&["1.1234567890123456789012345678901s"], 2, "Presisi");
    // Fractions that land exactly on a millisecond are fine.
    assert_eq!(rows(&["0.0036h"])[3], "12960");
}

#[test]
fn garbage_and_malformed_input_is_rejected() {
    for input in [
        "foo", "h", "1h30", "1h 30", "1 h", "1hm", "1H", "1,5h", "1h-30m", "1.h", ".5h", "1__0s", "1e3s", "0x10",
        "--1h", "1h;", "1d1d", "1h30m1h", "5ms5ms",
    ] {
        fails(&[input], 2, "");
    }
    fails(&["1h1h"], 2, "lebih dari sekali");
    fails(&["1h30"], 2, "tanpa satuan");
    fails(&["1x"], 2, "Satuan tidak dikenal: x");
    fails(&["foo"], 2, "Durasi tidak valid");
}

#[test]
fn iso_malformed_input_is_rejected() {
    for input in [
        "P", "PT", "P1DT", "PT1", "P1", "PT1M1H", "P1DW", "PT1HT2M", "P-1D", "PT-1H", "P1H", "PT1D", "P1.D", "PT1S2",
    ] {
        fails(&[input], 2, "ISO-8601");
    }
    fails(&["PT1H1H"], 2, "lebih dari sekali");
    fails(&["PT1M1H"], 2, "Urutan");
    fails(&["P1D2W"], 2, "Urutan");
}

#[test]
fn empty_and_sign_only_input() {
    fails(&[], 2, "wajib");
    fails(&[""], 2, "kosong");
    fails(&["  "], 2, "kosong");
    fails(&["-"], 2, "kosong");
    fails(&["+"], 2, "kosong");
}

#[test]
fn option_errors_exit_2() {
    fails(&["1h", "--unit", "x"], 2, "unit");
    fails(&["1h", "--unit", "y"], 2, "unit");
    fails(&["1h", "--unit"], 2, "");
    fails(&["1h", "--bogus"], 2, "");
}

#[test]
fn help_exits_zero() {
    for flag in ["-h", "--help"] {
        let r = dt(&["duration", flag]);
        assert_eq!(r.code, 0);
        assert!(r.out.starts_with("dt duration [options] <durasi...>"));
        assert!(r.out.contains("dt duration diff"));
        assert_eq!(r.err, "");
    }
}

#[test]
fn diff_utc_instants() {
    let r = dt(&["duration", "diff", "2026-01-02T00:00:00Z", "2026-01-03T06:30:00Z"]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(
        r.out,
        "Tanda:     +\nRingkas:   1d 6h 30m\nISO-8601:  P1DT6H30M\nDetik:     109800\nMilidetik: 109800000\n"
    );
}

#[test]
fn diff_is_b_minus_a_and_signed() {
    let r = dt(&["duration", "diff", "2026-01-03T06:30:00Z", "2026-01-02T00:00:00Z"]);
    assert_eq!(r.code, 0);
    assert_eq!(
        r.out,
        "Tanda:     -\nRingkas:   -1d 6h 30m\nISO-8601:  -P1DT6H30M\nDetik:     -109800\nMilidetik: -109800000\n"
    );
    let same = dt(&["duration", "diff", "2026-01-02T00:00:00Z", "2026-01-02T00:00:00Z"]);
    assert_eq!(
        same.out,
        "Tanda:     0\nRingkas:   0s\nISO-8601:  PT0S\nDetik:     0\nMilidetik: 0\n"
    );
}

#[test]
fn diff_honours_offsets() {
    // Same instant written in two offsets.
    let r = dt(&["duration", "diff", "2026-01-02T10:00:00+07:00", "2026-01-02T03:00:00Z"]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("Tanda:     0\n"), "{}", r.out);
    // 10:00+07:00 is 03:00Z; 10:00-05:00 is 15:00Z.
    assert_eq!(
        single(&[
            "diff",
            "2026-01-02T10:00:00+07:00",
            "2026-01-02T10:00:00-05:00",
            "--unit",
            "h"
        ]),
        "12"
    );
    assert_eq!(
        single(&["diff", "2026-01-02T10:00:00+05:30", "2026-01-02T10:00:00Z", "-u", "m"]),
        "330"
    );
}

#[test]
fn diff_epoch_seconds_and_milliseconds() {
    assert_eq!(single(&["diff", "1767323045", "1767323105", "-u", "s"]), "60");
    assert_eq!(single(&["diff", "1767323045", "1767323045123", "-u", "ms"]), "123");
    assert_eq!(single(&["diff", "1767323045000", "1767323105000", "-u", "s"]), "60");
    // ISO and epoch mix: 2026-01-02T03:04:05Z is 1767323045.
    assert_eq!(single(&["diff", "2026-01-02T03:04:05Z", "1767323105", "-u", "s"]), "60");
    // Negative epochs are values, not flags.
    assert_eq!(single(&["diff", "-86400", "0", "-u", "d"]), "1");
    assert_eq!(single(&["diff", "0", "-86400", "-u", "d"]), "-1");
}

#[test]
fn diff_milliseconds_and_dates() {
    let r = dt(&["duration", "diff", "2026-01-02T00:00:00Z", "2026-01-02T00:00:00.5Z"]);
    assert_eq!(r.code, 0);
    assert!(r.out.ends_with("Detik:     0.5\nMilidetik: 500\n"), "{}", r.out);
    // Missing offset reads as UTC; a bare date is midnight UTC.
    assert_eq!(single(&["diff", "2026-01-02", "2026-01-02T12:00:00", "-u", "h"]), "12");
    // No DST: a day is always 24h in UTC arithmetic, including across 2026-03-29 in Europe.
    assert_eq!(
        single(&["diff", "2026-03-28T12:00:00Z", "2026-03-29T12:00:00Z", "-u", "h"]),
        "24"
    );
    // Leap day.
    assert_eq!(single(&["diff", "2028-02-28", "2028-03-01", "-u", "d"]), "2");
    assert_eq!(single(&["diff", "2027-02-28", "2027-03-01", "-u", "d"]), "1");
}

#[test]
fn diff_json_shape() {
    let v = json(&[
        "diff",
        "2026-01-02T10:00:00+07:00",
        "2026-01-02T03:00:00.250Z",
        "--json",
    ]);
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["a", "b", "sign", "compact", "iso", "seconds", "milliseconds"]);
    assert_eq!(v["a"], "2026-01-02T03:00:00.000Z");
    assert_eq!(v["b"], "2026-01-02T03:00:00.250Z");
    assert_eq!(v["sign"], 1);
    assert_eq!(v["compact"], "250ms");
    assert_eq!(v["iso"], "PT0.25S");
    assert_eq!(v["milliseconds"], 250);

    let v = json(&["diff", "1767323105", "1767323045", "--json", "-u", "m"]);
    assert_eq!(v["sign"], -1);
    assert_eq!(v["compact"], "-1m");
    assert_eq!(v["unit"], "m");
    assert_eq!(v["value"], -1);
}

#[test]
fn diff_extremes_do_not_overflow() {
    // Both ends at the +-8.64e15 ms limit of the time tool.
    assert_eq!(
        single(&["diff", "-8640000000000000", "8640000000000000", "-u", "ms"]),
        "17280000000000000"
    );
}

#[test]
fn diff_errors_exit_2() {
    fails(&["diff"], 2, "tepat dua");
    fails(&["diff", "2026-01-02"], 2, "tepat dua");
    fails(&["diff", "2026-01-02", "2026-01-03", "2026-01-04"], 2, "tepat dua");
    fails(&["diff", "abc", "2026-01-02"], 2, "Format waktu");
    fails(&["diff", "2026-01-02", "xyz"], 2, "Format waktu");
    fails(&["diff", "2026-13-02", "2026-01-02"], 2, "tidak valid");
    fails(&["diff", "2026-02-30T00:00:00Z", "2026-01-02"], 2, "tidak valid");
    fails(&["diff", "99999999999999999", "0"], 2, "rentang");
    fails(&["diff", "1h", "2h"], 2, "Format waktu");
}

#[test]
fn diff_help_and_flag_position() {
    let r = dt(&["duration", "diff", "-h"]);
    assert_eq!(r.code, 0);
    assert!(r.out.contains("dt duration diff"));
    assert_eq!(single(&["--unit", "h", "diff", "2026-01-02", "2026-01-03"]), "24");
}
