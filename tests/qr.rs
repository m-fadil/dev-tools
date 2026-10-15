mod common;
use common::{dt, dt_in};

fn json(args: &[&str], stdin: &[u8]) -> serde_json::Value {
    let mut full = vec!["qr", "--json"];
    full.extend_from_slice(args);
    let r = dt_in(&full, stdin);
    assert_eq!(r.code, 0, "{}", r.err);
    serde_json::from_str(&r.out).unwrap()
}

fn matrix(v: &serde_json::Value) -> Vec<Vec<bool>> {
    v["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap().chars().map(|c| c == '1').collect())
        .collect()
}

/// Dark-module rows from margin-0 unicode output; `inverted` = output was made with --invert.
fn from_unicode(out: &str, inverted: bool) -> Vec<Vec<bool>> {
    let mut rows = Vec::new();
    for line in out.lines() {
        let (mut top, mut bottom) = (Vec::new(), Vec::new());
        for c in line.chars() {
            let (t, b) = match c {
                '█' => (true, true),
                '▀' => (true, false),
                '▄' => (false, true),
                ' ' => (false, false),
                other => panic!("unexpected char {other:?}"),
            };
            // filled == dark when inverted, filled == light otherwise
            top.push(t == inverted);
            bottom.push(b == inverted);
        }
        rows.push(top);
        rows.push(bottom);
    }
    rows
}

fn finder(m: &[Vec<bool>], ox: usize, oy: usize) -> bool {
    (0..7).all(|y| {
        (0..7).all(|x| {
            let edge = x == 0 || y == 0 || x == 6 || y == 6;
            let core = (2..=4).contains(&x) && (2..=4).contains(&y);
            m[oy + y][ox + x] == (edge || core)
        })
    })
}

#[test]
fn hello_is_version_1_with_21_modules() {
    let v = json(&["hello"], b"");
    assert_eq!(v["version"], 1);
    assert_eq!(v["width"], 21);
    assert_eq!(v["level"], "M");
    assert_eq!(v["text_bytes"], 5);
    let m = matrix(&v);
    assert_eq!(m.len(), 21);
    assert!(m.iter().all(|r| r.len() == 21));
}

#[test]
fn json_has_exactly_the_documented_keys() {
    let v = json(&["hello"], b"");
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["level", "modules", "text_bytes", "version", "width"]);
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.as_str().unwrap().bytes().all(|b| b == b'0' || b == b'1'))
    );
}

#[test]
fn finder_patterns_in_three_corners_but_not_the_fourth() {
    let m = matrix(&json(&["hello"], b""));
    assert!(finder(&m, 0, 0));
    assert!(finder(&m, 14, 0));
    assert!(finder(&m, 0, 14));
    assert!(!finder(&m, 14, 14));
    // separators around the top-left finder are light
    assert!((0..8).all(|i| !m[7][i] && !m[i][7]));
}

#[test]
fn width_follows_version_and_levels_differ() {
    let text = "0123456789abcdefghijklmnopqrstuvwxyz";
    let l = json(&["-l", "L", text], b"");
    let h = json(&["--level", "H", text], b"");
    for v in [&l, &h] {
        let version = v["version"].as_u64().unwrap();
        assert_eq!(v["width"].as_u64().unwrap(), 17 + 4 * version);
    }
    assert!(l["version"].as_u64() < h["version"].as_u64());
    assert_eq!(l["level"], "L");
    assert_eq!(h["level"], "H");
}

#[test]
fn unicode_margin_defaults_to_two() {
    let r = dt(&["qr", "hello"]);
    assert_eq!(r.code, 0);
    assert_eq!(r.err, "");
    let lines: Vec<&str> = r.out.lines().collect();
    assert_eq!(lines.len(), 13); // ceil((21 + 4) / 2)
    assert!(lines.iter().all(|l| l.chars().count() == 25));
    assert!(r.out.ends_with('\n'));
}

#[test]
fn unicode_margin_sizes() {
    for (m, side) in [(0usize, 21usize), (1, 23), (8, 37)] {
        let r = dt(&["qr", "-m", &m.to_string(), "hello"]);
        assert_eq!(r.code, 0);
        let lines: Vec<&str> = r.out.lines().collect();
        assert_eq!(lines.len(), side.div_ceil(2), "margin {m}");
        assert!(lines.iter().all(|l| l.chars().count() == side), "margin {m}");
    }
}

#[test]
fn default_polarity_fills_light_modules_for_dark_terminals() {
    let r = dt(&["qr", "hello"]);
    let lines: Vec<&str> = r.out.lines().collect();
    // quiet zone is light, hence filled; the odd last row pairs a light row with padding
    assert!(lines[0].chars().all(|c| c == '█'));
    assert!(lines[12].chars().all(|c| c == '█' || c == '▀'));
    assert!(lines.iter().all(|l| l.starts_with("██") && l.ends_with("██")));
}

#[test]
fn invert_flips_every_cell() {
    let normal = dt(&["qr", "hello"]).out;
    let inverted = dt(&["qr", "--invert", "hello"]).out;
    assert_ne!(normal, inverted);
    assert_eq!(normal.lines().count(), inverted.lines().count());
    let flip = |c: char| match c {
        '█' => ' ',
        ' ' => '█',
        '▀' => '▄',
        '▄' => '▀',
        '\n' => '\n',
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(normal.chars().map(flip).collect::<String>(), inverted);
    assert!(inverted.lines().next().unwrap().chars().all(|c| c == ' '));
}

#[test]
fn unicode_round_trips_to_json_matrix() {
    let want = matrix(&json(&["-l", "Q", "round trip"], b""));
    let normal = dt(&["qr", "-m", "0", "-l", "Q", "round trip"]).out;
    let inverted = dt(&["qr", "-m", "0", "--invert", "-l", "Q", "round trip"]).out;
    let n = want.len();
    // odd widths: the final row pair has one padding row to drop
    assert_eq!(&from_unicode(&normal, false)[..n], &want[..]);
    assert_eq!(&from_unicode(&inverted, true)[..n], &want[..]);
}

#[test]
fn ascii_uses_two_chars_per_module_and_matches_matrix() {
    let want = matrix(&json(&["hello"], b""));
    let r = dt(&["qr", "--format", "ascii", "-m", "0", "hello"]);
    assert_eq!(r.code, 0);
    let lines: Vec<&str> = r.out.lines().collect();
    assert_eq!(lines.len(), 21);
    for (line, row) in lines.iter().zip(&want) {
        assert_eq!(line.len(), 42);
        // default polarity: light modules are '##'
        let got: Vec<bool> = line.as_bytes().chunks(2).map(|c| c == b"  ").collect();
        assert_eq!(&got, row);
    }
    let inv = dt(&["qr", "--format", "ascii", "-m", "0", "--invert", "hello"]);
    let flipped: String = r
        .out
        .chars()
        .map(|c| match c {
            '#' => ' ',
            ' ' => '#',
            o => o,
        })
        .collect();
    assert_eq!(inv.out, flipped);
}

#[test]
fn ascii_margin_adds_blank_rows_and_columns() {
    let r = dt(&["qr", "--format", "ascii", "hello"]);
    let lines: Vec<&str> = r.out.lines().collect();
    assert_eq!(lines.len(), 25);
    assert!(lines.iter().all(|l| l.len() == 50));
    assert!(lines[0].bytes().all(|b| b == b'#'));
}

#[test]
fn svg_is_self_contained_and_counts_match() {
    let v = json(&["hello"], b"");
    let dark = matrix(&v).iter().flatten().filter(|d| **d).count();
    let r = dt(&["qr", "--format", "svg", "hello"]);
    assert_eq!(r.code, 0);
    assert_eq!(r.err, "");
    let s = &r.out;
    assert!(s.starts_with("<?xml"));
    assert!(s.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(s.contains("viewBox=\"0 0 25 25\""));
    assert!(s.trim_end().ends_with("</svg>"));
    assert!(!s.contains("href") && !s.contains("<image") && !s.contains("<script"));
    assert_eq!(s.matches("<svg").count(), 1);
    assert_eq!(s.matches("<g ").count(), s.matches("</g>").count());
    // background rect plus one rect per dark module
    assert_eq!(s.matches("<rect").count(), dark + 1);
    assert!(s.contains("fill=\"#fff\"") && s.contains("fill=\"#000\""));
    // the top-left finder corner sits at the margin offset
    assert!(s.contains("<rect x=\"2\" y=\"2\" width=\"1\" height=\"1\"/>"));
    assert!(!s.contains("<rect x=\"1\" y=\"1\" width=\"1\""));
}

#[test]
fn svg_margin_and_invert() {
    let a = dt(&["qr", "--format", "svg", "-m", "0", "hello"]).out;
    assert!(a.contains("viewBox=\"0 0 21 21\""));
    assert!(a.contains("<rect x=\"0\" y=\"0\" width=\"1\" height=\"1\"/>"));
    // svg is always black on white: --invert changes nothing
    let b = dt(&["qr", "--format", "svg", "-m", "0", "--invert", "hello"]).out;
    assert_eq!(a, b);
}

#[test]
fn capacity_boundary_at_each_level() {
    for (level, max) in [("L", 2953usize), ("M", 2331), ("Q", 1663), ("H", 1273)] {
        let ok = dt_in(&["qr", "--json", "-l", level], "a".repeat(max).as_bytes());
        assert_eq!(ok.code, 0, "level {level}: {}", ok.err);
        let v: serde_json::Value = serde_json::from_str(&ok.out).unwrap();
        assert_eq!(v["version"], 40);
        assert_eq!(v["width"], 177);

        let over = dt_in(&["qr", "-l", level], "a".repeat(max + 1).as_bytes());
        assert_eq!(over.code, 1, "level {level}");
        assert_eq!(over.out, "");
        assert!(over.err.contains(&format!("{} byte", max + 1)), "{}", over.err);
        assert!(
            over.err.contains(&format!("level {level} adalah {max} byte")),
            "{}",
            over.err
        );
    }
}

#[test]
fn empty_input_is_usage_error() {
    let stdin = dt_in(&["qr"], b"");
    assert_eq!(stdin.code, 2);
    assert_eq!(stdin.out, "");
    assert!(stdin.err.contains("kosong"));
    let arg = dt(&["qr", ""]);
    assert_eq!(arg.code, 2);
}

#[test]
fn stdin_and_file_match_positional_text() {
    let want = dt(&["qr", "--json", "halo dunia"]).out;
    assert_eq!(dt_in(&["qr", "--json"], b"halo dunia").out, want);

    let dir = std::env::temp_dir().join(format!("dt-qr-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.txt");
    std::fs::write(&path, "halo dunia").unwrap();
    let p = path.to_str().unwrap();
    assert_eq!(dt(&["qr", "--json", "--file", p]).out, want);
    let both = dt(&["qr", "--file", p, "teks"]);
    assert_eq!(both.code, 2);
    let missing = dt(&["qr", "--file", dir.join("tidak-ada").to_str().unwrap()]);
    assert_eq!(missing.code, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multiple_words_are_joined_with_a_space() {
    let a = dt(&["qr", "--json", "a", "b"]).out;
    let b = dt(&["qr", "--json", "a b"]).out;
    assert_eq!(a, b);
}

#[test]
fn binary_input_is_encoded() {
    let r = dt_in(&["qr", "--json"], &[0, 1, 2, 255, 254]);
    assert_eq!(r.code, 0, "{}", r.err);
    let v: serde_json::Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(v["text_bytes"], 5);
}

#[test]
fn invalid_options_are_usage_errors() {
    for args in [
        &["qr", "-l", "X", "a"][..],
        &["qr", "-l", "m", "a"],
        &["qr", "--format", "png", "a"],
        &["qr", "-m", "9", "a"],
        &["qr", "-m", "x", "a"],
        &["qr", "--bogus", "a"],
    ] {
        let r = dt(args);
        assert_eq!(r.code, 2, "{args:?}");
        assert_eq!(r.out, "", "{args:?}");
        assert!(!r.err.is_empty(), "{args:?}");
    }
}

#[test]
fn help_lists_options_and_exit_codes() {
    for flag in ["-h", "--help"] {
        let r = dt(&["qr", flag]);
        assert_eq!(r.code, 0);
        assert!(r.out.starts_with("dt qr"));
        for needle in ["--level", "--margin", "--format", "--invert", "--json", "Exit:"] {
            assert!(r.out.contains(needle), "{needle}");
        }
    }
}
