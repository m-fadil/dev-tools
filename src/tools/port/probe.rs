use crate::errors::{CliError, Result};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv6Addr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    Terbuka,
    Tertutup,
    Timeout,
}

impl CheckResult {
    pub fn text(self) -> &'static str {
        match self {
            CheckResult::Terbuka => "terbuka",
            CheckResult::Tertutup => "tertutup",
            CheckResult::Timeout => "timeout",
        }
    }

    pub fn status(self) -> &'static str {
        match self {
            CheckResult::Terbuka => "open",
            CheckResult::Tertutup => "closed",
            CheckResult::Timeout => "timeout",
        }
    }
}

/// Reject malformed DNS names before entering the platform resolver. Some
/// resolvers retry these names long enough to hide ENOTFOUND behind our timer.
/// Literal addresses (including scoped IPv6) are not DNS names.
fn validate_host(host: &str) -> Result<()> {
    if host.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    if let Some((address, scope)) = host.split_once('%')
        && address.parse::<Ipv6Addr>().is_ok()
        && !scope.is_empty()
        && scope
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Ok(());
    }
    let name = host.strip_suffix('.').unwrap_or(host);
    let valid = !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|b| !b.is_ascii() || b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        });
    if valid {
        Ok(())
    } else {
        Err(CliError::Failed(format!("getaddrinfo ENOTFOUND {host}")))
    }
}

fn connect_all(host: &str, port: u16, deadline: Instant) -> std::result::Result<CheckResult, String> {
    let addrs = (host, port).to_socket_addrs().map_err(|e| e.to_string())?;
    let mut timed_out = false;
    let mut error: Option<String> = None;
    for addr in addrs {
        let now = Instant::now();
        if now >= deadline {
            timed_out = true;
            break;
        }
        match TcpStream::connect_timeout(&addr, deadline - now) {
            Ok(_) => return Ok(CheckResult::Terbuka),
            Err(e) if e.kind() == ErrorKind::ConnectionRefused => {}
            Err(e) if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock => timed_out = true,
            Err(e) => {
                error.get_or_insert_with(|| format!("connect {addr}: {e}"));
            }
        }
    }
    if timed_out {
        Ok(CheckResult::Timeout)
    } else if let Some(e) = error {
        Err(e)
    } else {
        Ok(CheckResult::Tertutup)
    }
}

/// TCP connect attempt bounded by `timeout_ms` overall (name resolution included).
/// Non-refusal socket errors fail.
pub fn check_port(host: &str, port: u16, timeout_ms: u64) -> Result<CheckResult> {
    validate_host(host)?;
    let timeout = Duration::from_millis(timeout_ms);
    let deadline = Instant::now() + timeout;
    let (tx, rx) = mpsc::channel();
    let host = host.to_string();
    std::thread::spawn(move || {
        let _ = tx.send(connect_all(&host, port, deadline));
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(message)) => Err(CliError::Failed(message)),
        Err(_) => Ok(CheckResult::Timeout),
    }
}

/// True if the port can be bound (listen, then close); address-in-use and permission denial mean false.
pub fn can_bind(host: &str, port: u16) -> Result<bool> {
    validate_host(host)?;
    match TcpListener::bind((host, port)) {
        Ok(_) => Ok(true),
        Err(e) if matches!(e.kind(), ErrorKind::AddrInUse | ErrorKind::PermissionDenied) => Ok(false),
        Err(e) => Err(CliError::Failed(e.to_string())),
    }
}

/// First bindable port in [from, to], or None.
pub fn find_free_port(host: &str, from: u16, to: u16) -> Result<Option<u16>> {
    for port in from..=to {
        if can_bind(host, port)? {
            return Ok(Some(port));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::validate_host;

    #[test]
    fn distinguishes_hostnames_from_malformed_resolver_input() {
        for host in [
            "localhost",
            "example.com.",
            "a_b.example",
            "127.0.0.1",
            "::1",
            "fe80::1%eth0",
        ] {
            assert!(validate_host(host).is_ok(), "{host}");
        }
        for host in ["[invalid]", "[::1]", "a b", "bad/host", "::bad::", "a..b", "a!.example"] {
            assert!(validate_host(host).is_err(), "{host}");
        }
    }
}
