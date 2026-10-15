mod common;
use common::{dt, dt_in};
use serde_json::{Value, json};

fn to(style: &str, text: &str) -> String {
    dt(&["case", "-t", style, text]).out.trim_end().to_string()
}

#[test]
fn word_splitting() {
    assert_eq!(to("snake", "HTTPServerError"), "http_server_error");
    assert_eq!(to("lower", "parseURL2Json"), "parse url 2 json");
    assert_eq!(to("kebab", "foo_bar-baz.qux/quux  corge"), "foo-bar-baz-qux-quux-corge");
    assert_eq!(to("snake", "getHTTPResponse"), "get_http_response");
    assert_eq!(to("snake", "don't stop"), "dont_stop");
}

#[test]
fn all_styles() {
    let t = "hello WORLD fooBar";
    for (s, e) in [
        ("camel", "helloWorldFooBar"),
        ("pascal", "HelloWorldFooBar"),
        ("snake", "hello_world_foo_bar"),
        ("screaming", "HELLO_WORLD_FOO_BAR"),
        ("kebab", "hello-world-foo-bar"),
        ("train", "Hello-World-Foo-Bar"),
        ("dot", "hello.world.foo.bar"),
        ("path", "hello/world/foo/bar"),
        ("title", "Hello World Foo Bar"),
        ("sentence", "Hello world foo bar"),
        ("lower", "hello world foo bar"),
        ("upper", "HELLO WORLD FOO BAR"),
    ] {
        assert_eq!(to(s, t), e, "{s}");
    }
}

#[test]
fn unicode_letters() {
    assert_eq!(to("snake", "ÉcoleNormale"), "école_normale");
    assert_eq!(to("pascal", "straße größe"), "StraßeGröße");
    assert_eq!(to("camel", "привет мир"), "приветМир");
}

#[test]
fn default_is_all_labelled_multiline() {
    let r = dt_in(&["case"], b"fooBar\nbaz_qux\n");
    assert_eq!(r.code, 0);
    let lines: Vec<&str> = r.out.split('\n').collect();
    assert!(lines.contains(&"camel      fooBar"));
    assert!(lines.contains(&"screaming  BAZ_QUX"));
    assert_eq!(lines.iter().filter(|l| l.starts_with("camel")).count(), 2);
    assert_eq!(
        dt_in(&["case", "-t", "snake"], b"fooBar\r\n\r\nbazQux\n").out,
        "foo_bar\n\nbaz_qux\n"
    );
}

#[test]
fn json_shape() {
    let r = dt_in(&["case", "-t", "kebab", "--json"], b"fooBar\nx Y");
    let v: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(
        v,
        json!({"style": "kebab", "lines": [{"input": "fooBar", "output": "foo-bar"}, {"input": "x Y", "output": "x-y"}]})
    );
    let all: Value = serde_json::from_str(&dt(&["case", "--json", "a b"]).out).unwrap();
    assert_eq!(all["style"], "all");
    assert_eq!(all["lines"][0]["output"]["pascal"], "AB");
    assert_eq!(all["lines"][0]["output"].as_object().unwrap().len(), 12);
}

#[test]
fn invalid_style_is_usage_error() {
    let r = dt(&["case", "-t", "weird", "x"]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("--to tidak valid: weird"));
}
