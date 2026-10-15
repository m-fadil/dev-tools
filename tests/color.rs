mod common;
use common::{dt, dt_in};
use serde_json::Value;

fn fmt(value: &str, format: &str) -> String {
    dt(&["color", "-f", format, value]).out.trim().to_string()
}

fn json(args: &[&str]) -> Value {
    let r = dt(args);
    assert_eq!(r.code, 0, "{}", r.err);
    serde_json::from_str(&r.out).unwrap()
}

#[test]
fn coral_round_trips() {
    for input in [
        "coral",
        "CORAL",
        "#ff7f50",
        "ff7f50",
        "rgb(255, 127, 80)",
        "rgb(255 127 80)",
    ] {
        assert_eq!(fmt(input, "hex"), "#ff7f50", "{input}");
        assert_eq!(fmt(input, "rgb"), "rgb(255, 127, 80)", "{input}");
        assert_eq!(fmt(input, "hsl"), "hsl(16, 100%, 66%)", "{input}");
        assert_eq!(fmt(input, "name"), "coral", "{input}");
    }
    assert_eq!(fmt("coral", "hwb"), "hwb(16 31% 0%)");
}

#[test]
fn hsl_input_prints_back() {
    assert_eq!(fmt("hsl(16, 100%, 66%)", "hsl"), "hsl(16, 100%, 66%)");
}

#[test]
fn hex_forms_and_joining() {
    assert_eq!(fmt("#f80", "hex"), "#ff8800");
    assert_eq!(fmt("#f808", "hex"), "#ff880088");
    assert_eq!(fmt("#ff880080", "rgb"), "rgba(255, 136, 0, 0.502)");
    assert_eq!(fmt("ff880080", "hex"), "#ff880080");
    assert_eq!(fmt("#ff8800ff", "hex"), "#ff8800");
    assert_eq!(dt(&["color", "-f", "hex", "rgb(255", "0", "0)"]).out.trim(), "#ff0000");
    assert_eq!(dt_in(&["color", "-f", "hex"], b"navy\n").out.trim(), "#000080");
}

#[test]
fn syntax_variants() {
    let pi = format!("hsl({}rad 100% 50%)", std::f64::consts::PI);
    for (input, expected) in [
        ("rgb(100% 0% 0% / 50%)", "#ff000080"),
        ("rgb(300 -5 127.5)", "#ff0080"),
        ("hsl(120deg 100% 25%)", "#008000"),
        ("hsl(0.5turn 100% 50%)", "#00ffff"),
        ("hsl(200grad 100% 50%)", "#00ffff"),
        (pi.as_str(), "#00ffff"),
        ("hsl(-120 100% 50%)", "#0000ff"),
        ("hsla(120, 100%, 50%, 0.25)", "#00ff0040"),
        ("hwb(0 0% 0%)", "#ff0000"),
        ("hwb(120 50% 50%)", "#808080"),
        ("hwb(0 25% 25% / 0.5)", "#bf404080"),
    ] {
        assert_eq!(fmt(input, "hex"), expected, "{input}");
    }
    assert_eq!(fmt("rgba(0,0,255,.5)", "hsl"), "hsla(240, 100%, 50%, 0.5)");
    assert_eq!(fmt("#bf4040", "hwb"), "hwb(0 25% 25%)");
}

#[test]
fn names() {
    assert_eq!(fmt("#00ffff", "name"), "aqua");
    assert_eq!(fmt("#808080", "name"), "gray");
    assert_eq!(fmt("transparent", "rgb"), "rgba(0, 0, 0, 0)");
    assert_eq!(fmt("rgba(0,0,0,0)", "name"), "transparent");
    assert_eq!(fmt("#ff7f51", "name"), "~coral");
    assert_eq!(fmt("rebeccapurple", "hex"), "#663399");
}

#[test]
fn wcag() {
    let j = json(&["color", "--json", "white"]);
    assert_eq!(j["luminance"], 1);
    assert_eq!(
        j["contrast"]["black"],
        serde_json::json!({"ratio": 21, "aa": true, "aaa": true, "aaLarge": true, "aaaLarge": true})
    );
    assert_eq!(j["contrast"]["white"]["ratio"], 1);
    assert_eq!(json(&["color", "--json", "--contrast", "black", "white"])["ratio"], 21);
    let g = json(&["color", "--json", "--contrast", "#fff", "#767676"]);
    assert_eq!(g["ratio"], 4.54);
    assert_eq!(g["aa"], true);
    assert_eq!(g["aaa"], false);
    let h = json(&["color", "--json", "--contrast", "#fff", "#777"]);
    assert_eq!(h["ratio"], 4.47);
    assert_eq!(h["aa"], false);
    assert_eq!(h["aaLarge"], true);
    assert_eq!(h["aaaLarge"], false);
    assert!(dt(&["color", "--contrast", "white", "black"]).out.contains("21:1"));
    assert_eq!(json(&["color", "--json", "#ff0000"])["luminance"], 0.2126);
}

fn has_line(out: &str, label: &str, rest: &str) -> bool {
    out.lines()
        .any(|l| l.strip_prefix(label).is_some_and(|r| r.trim_start().starts_with(rest)))
}

#[test]
fn default_output() {
    let out = dt(&["color", "coral"]).out;
    assert!(has_line(&out, "hex", "#ff7f50"));
    assert!(has_line(&out, "hsl", "hsl(16, 100%, 66%)"));
    assert!(has_line(&out, "nama", "coral"));
    assert!(has_line(&out, "vs putih", "2.49:1  AA gagal"));
    assert!(has_line(&out, "vs hitam", "8.4:1  AA lulus, AAA lulus"));
}

#[test]
fn json_shape() {
    let r = dt(&["color", "--json", "rgba(255,127,80,0.5)"]);
    assert_eq!(r.code, 0);
    let j: Value = serde_json::from_str(&r.out).unwrap();
    let keys: Vec<&String> = j.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        [
            "input",
            "hex",
            "rgb",
            "hsl",
            "hwb",
            "css",
            "alpha",
            "name",
            "nearestName",
            "luminance",
            "contrast"
        ]
    );
    assert_eq!(j["hex"], "#ff7f5080");
    assert_eq!(j["rgb"], serde_json::json!({"r": 255, "g": 127, "b": 80}));
    assert_eq!(j["hsl"], serde_json::json!({"h": 16, "s": 100, "l": 66}));
    assert_eq!(j["hwb"], serde_json::json!({"h": 16, "w": 31, "b": 0}));
    assert_eq!(j["alpha"], 0.5);
    assert_eq!(j["name"], "coral");
    assert_eq!(j["css"]["hsl"], "hsla(16, 100%, 66%, 0.5)");
    assert_eq!(json(&["color", "--json", "#ff7f51"])["name"], Value::Null);
}

#[test]
fn invalid_input_is_usage_error() {
    for bad in [
        "",
        "notacolor",
        "#12",
        "#12345",
        "abc",
        "ggg000",
        "rgb(1,2)",
        "rgb(1 2 3 4)",
        "rgb(1, 2 3)",
        "rgb(1%, 2, 3)",
        "hsl(10, 50, 50)",
        "hsl(10xyz 50% 50%)",
        "hwb(1, 2%, 3%)",
        "rgb(1 2 3 / )",
        "rgb(1, 2, 3 / 0.5)",
    ] {
        let r = dt(&["color", bad]);
        assert_eq!(r.code, 2, "{bad}");
        assert!(!r.err.is_empty());
        assert_eq!(r.out, "");
    }
}

#[test]
fn option_conflicts() {
    assert_eq!(dt(&["color", "-f", "bogus", "red"]).code, 2);
    assert_eq!(dt(&["color", "-f", "hex", "--json", "red"]).code, 2);
    assert_eq!(dt(&["color", "-f", "hex", "--contrast", "blue", "red"]).code, 2);
    assert_eq!(dt(&["color", "--contrast", "nope", "red"]).code, 2);
}

#[test]
fn non_finite_components_rejected_without_output() {
    let huge = "9".repeat(400);
    let finite = "9".repeat(308);
    for v in [
        format!("hsl({huge} 50% 50%)"),
        format!("hsl(-{huge}deg 50% 50%)"),
        format!("hsl({huge}grad 50% 50%)"),
        format!("hsl({huge}rad 50% 50%)"),
        format!("hsl({huge}turn 50% 50%)"),
        format!("hsl({finite}turn 50% 50%)"),
        format!("hsl({finite}rad 50% 50%)"),
        format!("hsl(0 {huge}% 50%)"),
        format!("hsl(0 50% {huge}%)"),
        format!("hwb({huge} 0% 0%)"),
        format!("hwb(0 {huge}% 0%)"),
        format!("hwb(0 0% {huge}%)"),
        format!("rgb({huge} 0 0)"),
        format!("rgb(0 {huge} 0)"),
        format!("rgb(0 0 -{huge})"),
        format!("rgb({huge}% 0% 0%)"),
        format!("rgb({finite}% 0% 0%)"),
        format!("rgb(0 0 0 / {huge})"),
        format!("rgb(0 0 0 / {huge}%)"),
        format!("rgba(0,0,0,-{huge})"),
        "rgb(NaN 0 0)".to_string(),
        "rgb(0 0 0 / Infinity)".to_string(),
    ] {
        let r = dt(&["color", "-f", "rgb", &v]);
        assert_eq!(r.code, 2, "{}", &v[..v.len().min(40)]);
        assert_eq!(r.out, "");
        assert!(!r.err.is_empty());
    }
}

#[test]
fn alpha_rounded_once() {
    type Case<'a> = (&'a str, &'a str, f64, Option<&'a str>, &'a str, [&'a str; 3]);
    let cases: [Case; 2] = [
        (
            "rgb(1 2 3 / 0.9999)",
            "#010203",
            1.0,
            None,
            "black",
            ["rgb(1, 2, 3)", "hsl(210, 50%, 1%)", "hwb(210 0% 99%)"],
        ),
        (
            "rgba(0,0,0,0.0001)",
            "#00000000",
            0.0,
            Some("transparent"),
            "transparent",
            ["rgba(0, 0, 0, 0)", "hsla(0, 0%, 0%, 0)", "hwb(0 0% 100% / 0)"],
        ),
    ];
    for (input, hex, alpha, name, nearest, css) in cases {
        assert_eq!(fmt(input, "hex"), hex);
        for (f, c) in ["rgb", "hsl", "hwb"].iter().zip(css) {
            assert_eq!(fmt(input, f), c);
        }
        assert_eq!(fmt(input, "name"), name.map_or(format!("~{nearest}"), String::from));
        let j = json(&["color", "--json", input]);
        assert_eq!(j["hex"], hex);
        assert_eq!(j["alpha"], alpha);
        assert_eq!(j["name"], name.map_or(Value::Null, |n| Value::String(n.into())));
        assert_eq!(j["nearestName"], nearest);
        assert_eq!(j["css"]["rgb"], css[0]);
        assert_eq!(j["css"]["hsl"], css[1]);
        assert_eq!(j["css"]["hwb"], css[2]);
        let plain = dt(&["color", input]).out;
        assert!(has_line(&plain, "alpha", &format!("{}", alpha)));
        assert!(plain.contains(hex) && plain.contains(css[0]));
    }
}
