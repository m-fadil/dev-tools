use regex::Regex;
use serde_json::{Value, json};

use super::whatwg;

pub fn whitespace(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | '\u{20}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub struct ParsedUrl {
    pub protocol: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
    pub origin: String,
    pub query: Vec<(String, String)>,
}

impl ParsedUrl {
    pub fn to_json(&self) -> Value {
        json!({
            "protocol": self.protocol,
            "username": self.username,
            "password": self.password,
            "host": self.host,
            "hostname": self.hostname,
            "port": self.port,
            "pathname": self.pathname,
            "search": self.search,
            "hash": self.hash,
            "origin": self.origin,
            "query": self.query.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        })
    }
}

fn starts_with_scheme(text: &str) -> bool {
    let mut chars = text.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-')) {
            return false;
        }
    }
    false
}

/// Masks everything through the last "@" in authority-style or scheme-less input.
fn mask_credentials(text: &str) -> String {
    let prefix = Regex::new(r"(?i-u)^(?:[a-z][a-z0-9+.-]*:)?//")
        .ok()
        .and_then(|re| re.find(text).map(|m| m.end()));
    if let Some(end) = prefix
        && let Some(at) = text[end..].rfind('@')
    {
        return format!("{}***@{}", &text[..end], &text[end + at + 1..]);
    }
    if !starts_with_scheme(text)
        && let Some(at) = text.rfind('@')
    {
        return format!("***@{}", &text[at + 1..]);
    }
    text.to_string()
}

/// Parses an absolute URL (no base); the error is the user-facing message.
pub fn parse_url(text: &str, show_password: bool) -> Result<ParsedUrl, String> {
    let Some(url) = whatwg::parse(text) else {
        let hint = if starts_with_scheme(text) {
            ""
        } else {
            " (tambahkan skema, mis. https://)"
        };
        return Err(format!(
            "URL tidak valid: {}{hint}",
            mask_credentials(text.trim_matches(whitespace))
        ));
    };
    let password = url.password();
    Ok(ParsedUrl {
        protocol: url.protocol(),
        username: url.username(),
        password: if !password.is_empty() && !show_password {
            "***".to_string()
        } else {
            password
        },
        host: url.host_text(),
        hostname: url.hostname(),
        port: url.port_text(),
        pathname: url.pathname(),
        search: url.search(),
        hash: url.hash(),
        origin: url.origin(),
        query: url.search_params(),
    })
}

/// Explicit port, or "(default n)" for known schemes; None when neither applies.
pub fn port_label(parsed: &ParsedUrl) -> Option<String> {
    if !parsed.port.is_empty() {
        return Some(parsed.port.clone());
    }
    let scheme = parsed.protocol.trim_end_matches(':');
    whatwg::default_port(scheme).map(|p| format!("(default {p})"))
}

fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn pad_end(s: &str, width: usize) -> String {
    let len = js_len(s);
    format!("{s}{}", " ".repeat(width.saturating_sub(len)))
}

pub fn format_url(parsed: &ParsedUrl) -> String {
    let fields: [(&str, Option<String>); 10] = [
        ("protocol", Some(parsed.protocol.clone())),
        ("username", Some(parsed.username.clone())),
        ("password", Some(parsed.password.clone())),
        ("host", Some(parsed.host.clone())),
        ("hostname", Some(parsed.hostname.clone())),
        ("port", port_label(parsed)),
        ("pathname", Some(parsed.pathname.clone())),
        ("search", Some(parsed.search.clone())),
        ("hash", Some(parsed.hash.clone())),
        // Opaque origins serialize as "null"; that is noise, not information.
        ("origin", (parsed.origin != "null").then(|| parsed.origin.clone())),
    ];
    let shown: Vec<(&str, String)> = fields
        .into_iter()
        .filter_map(|(l, v)| v.filter(|v| !v.is_empty()).map(|v| (l, v)))
        .collect();
    let width = shown.iter().map(|(l, _)| l.len()).max().unwrap_or(0) + 1;
    let mut lines: Vec<String> = shown
        .iter()
        .map(|(label, value)| format!("{}{value}", pad_end(&format!("{label}:"), width + 1)))
        .collect();
    if !parsed.query.is_empty() {
        let key_width = parsed.query.iter().map(|(k, _)| js_len(k)).max().unwrap_or(0);
        lines.push(String::new());
        lines.push("Query:".to_string());
        lines.extend(
            parsed
                .query
                .iter()
                .map(|(k, v)| format!("  {} = {v}", pad_end(k, key_width))),
        );
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_url_errors_never_retain_raw_credentials() {
        let err = match parse_url("https://user:SECRETPW@exa mple.com/", true) {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("https://***@exa mple.com/"), "{err}");
        assert!(!err.contains("SECRETPW") && !err.contains("user:"));
    }

    #[test]
    fn whatwg_basics() {
        let p = |s: &str| parse_url(s, true).ok();
        let u = p("http://0x7f.1/").map(|u| u.hostname);
        assert_eq!(u.as_deref(), Some("127.0.0.1"));
        let u = p("http://[2001:db8:0:0:1:0:0:1]/").map(|u| u.hostname);
        assert_eq!(u.as_deref(), Some("[2001:db8::1:0:0:1]"));
        let u = p("file:///C|/a/../../b").map(|u| u.pathname);
        assert_eq!(u.as_deref(), Some("/C:/b"));
        let u = p("http://bücher.de/ü").map(|u| (u.hostname, u.pathname));
        assert_eq!(u, Some(("xn--bcher-kva.de".into(), "/%C3%BC".into())));
        assert!(p("http://a b/").is_none());
        assert!(p("http://1.2.3.4.5/").is_none());
    }
}
