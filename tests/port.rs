mod common;

use common::dt;
use serde_json::{Value, json};
use std::net::TcpListener;
use std::time::{Duration, Instant};

#[test]
fn check_open_closed_and_json() {
    let server = TcpListener::bind("127.0.0.1:0").expect("local listener");
    let port = server.local_addr().expect("listener address").port();
    let number = port.to_string();
    let open = dt(&["port", "check", &number]);
    assert_eq!(open.code, 0, "{}", open.err);
    assert_eq!(open.out, format!("127.0.0.1:{port} terbuka\n"));
    let open_json = dt(&["port", "check", "--json", &number]);
    assert_eq!(open_json.code, 0);
    assert_eq!(
        serde_json::from_str::<Value>(&open_json.out).expect("JSON"),
        json!({"host":"127.0.0.1","port":port,"status":"open"})
    );
    drop(server);
    let closed = dt(&["port", "check", "--timeout", "10000", &number]);
    assert_eq!(closed.code, 1, "{}", closed.err);
    assert_eq!(closed.out, format!("127.0.0.1:{port} tertutup\n"));
    let closed_json = dt(&["port", "check", "--json", &number]);
    assert_eq!(closed_json.code, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&closed_json.out).expect("JSON"),
        json!({"host":"127.0.0.1","port":port,"status":"closed"})
    );
}

#[test]
fn strict_port_range_and_usage() {
    for bad in ["0", "65536", "80x", "-1", "1.5", ""] {
        assert_eq!(dt(&["port", "check", bad]).code, 2, "{bad}");
    }
    for args in [
        vec!["port"],
        vec!["port", "check"],
        vec!["port", "check", "80", "--timeout", "0"],
        vec!["port", "check", "80", "--timeout", "60001"],
        vec!["port", "free", "--from", "10", "--to", "9"],
        vec!["port", "free", "--from", "0"],
        vec!["port", "free", "--to", "65536"],
        vec!["port", "free", "extra"],
    ] {
        assert_eq!(dt(&args).code, 2, "{args:?}");
    }
    let unknown = dt(&["port", "scan"]);
    assert_eq!(unknown.code, 2);
    assert!(unknown.err.contains("Subcommand tidak dikenal: scan"));
    let bad = dt(&["port", "check", "abc"]);
    assert_eq!(bad.code, 2);
    assert!(bad.err.contains("Port harus bilangan bulat 1-65535: abc"));
    assert!(!bad.err.contains("--port"));
}

#[test]
fn malformed_hostname_is_not_closed() {
    let result = dt(&["port", "check", "80", "--host", "[invalid]"]);
    assert_eq!(result.code, 1);
    assert!(result.out.is_empty());
    assert!(!result.err.is_empty());
}

#[test]
fn free_skips_busy_ports_and_reports_exhaustion() {
    let server = TcpListener::bind("127.0.0.1:0").expect("local listener");
    let port = server.local_addr().expect("listener address").port();
    let number = port.to_string();
    if port < 65535 {
        let next = (port + 1).to_string();
        let result = dt(&["port", "free", "--from", &number, "--to", &next]);
        if result.code == 0 {
            assert_eq!(result.out.trim(), next);
        } else {
            assert_eq!(result.code, 1);
            assert!(result.err.contains("Tidak ada port kosong"));
        }
    }
    for extra in [vec![], vec!["--json"]] {
        let mut args = vec!["port", "free", "--from", &number, "--to", &number];
        args.extend(extra);
        let busy = dt(&args);
        assert_eq!(busy.code, 1);
        assert!(busy.out.is_empty());
        assert!(busy.err.contains("Tidak ada port kosong"));
    }
    drop(server);
    let free = dt(&["port", "free", "--from", &number, "--to", &number]);
    assert_eq!(free.code, 0, "{}", free.err);
    assert_eq!(free.out.trim(), number);
    let free_json = dt(&["port", "free", "--json", "--from", "20000", "--to", "65535"]);
    assert_eq!(free_json.code, 0, "{}", free_json.err);
    let doc: Value = serde_json::from_str(&free_json.out).expect("JSON");
    assert_eq!(doc["host"], "127.0.0.1");
    assert!(doc["port"].as_u64().expect("port number") >= 20000);
}

#[test]
fn check_network_timeout_is_bounded() {
    let start = Instant::now();
    let result = dt(&[
        "port",
        "check",
        "--json",
        "--host",
        "10.255.255.1",
        "--timeout",
        "200",
        "80",
    ]);
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(result.code, 1, "{}", result.err);
    if result.out.is_empty() {
        // Platform socket failures (including unavailable sandbox networking) are errors.
        assert!(!result.err.is_empty());
    } else {
        assert_eq!(
            serde_json::from_str::<Value>(&result.out).expect("JSON"),
            json!({"host":"10.255.255.1","port":80,"status":"timeout"})
        );
    }
}
