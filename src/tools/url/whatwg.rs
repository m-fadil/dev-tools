//! WHATWG parsing with Node's ASCII-domain fast path and current path encode set.
//! Unicode hosts retain the URL crate's full UTS #46 normalization and validation.

pub struct Url {
    inner: ::url::Url,
    ascii_host: Option<String>,
}

pub fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "ftp" => Some(21),
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        _ => None,
    }
}

fn percent_decode_ascii(text: &str) -> Option<String> {
    let mut out = Vec::with_capacity(text.len());
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(a), Some(c)) = ((b[i + 1] as char).to_digit(16), (b[i + 2] as char).to_digit(16))
        {
            out.push((a * 16 + c) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    if !out.is_ascii() {
        return None;
    }
    String::from_utf8(out).ok().map(|s| s.to_ascii_lowercase())
}

pub fn parse(text: &str) -> Option<Url> {
    if let Ok(inner) = ::url::Url::parse(text) {
        return Some(Url {
            inner,
            ascii_host: None,
        });
    }
    // Node bypasses UTS #46 for ASCII domains, including ASCII ACE labels. The
    // crate deliberately validates those labels. Parse an otherwise identical
    // neutral domain, retaining the original domain for all observable fields.
    let clean: String = text
        .trim_matches(|c: char| c <= '\u{20}')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
        .collect();
    let colon = clean.find(':')?;
    let scheme = clean[..colon].to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https" | "ftp" | "ws" | "wss" | "file") {
        return None;
    }
    let rest = &clean[colon + 1..];
    let slashes = rest.bytes().take_while(|b| matches!(b, b'/' | b'\\')).count();
    if scheme == "file" && slashes != 2 {
        return None;
    }
    let authority_start = colon + 1 + slashes;
    let authority_end = clean[authority_start..]
        .find(['/', '\\', '?', '#'])
        .map(|n| authority_start + n)
        .unwrap_or(clean.len());
    let authority = &clean[authority_start..authority_end];
    let host_start = authority
        .rfind('@')
        .map(|n| authority_start + n + 1)
        .unwrap_or(authority_start);
    let host_end = clean[host_start..authority_end]
        .find(':')
        .map(|n| host_start + n)
        .unwrap_or(authority_end);
    let host = percent_decode_ascii(&clean[host_start..host_end])?;
    if !host.split('.').any(|label| label.starts_with("xn--")) {
        return None;
    }
    let neutral = host
        .split('.')
        .map(|label| {
            label
                .strip_prefix("xn--")
                .map(|v| format!("zz--{v}"))
                .unwrap_or_else(|| label.to_string())
        })
        .collect::<Vec<_>>()
        .join(".");
    let replacement = format!("{}{neutral}{}", &clean[..host_start], &clean[host_end..]);
    let inner = ::url::Url::parse(&replacement).ok()?;
    Some(Url {
        inner,
        ascii_host: Some(host),
    })
}

impl Url {
    pub fn protocol(&self) -> String {
        format!("{}:", self.inner.scheme())
    }
    pub fn username(&self) -> String {
        self.inner.username().replace('^', "%5E")
    }
    pub fn password(&self) -> String {
        self.inner.password().unwrap_or("").replace('^', "%5E")
    }
    pub fn hostname(&self) -> String {
        self.ascii_host
            .clone()
            .unwrap_or_else(|| self.inner.host_str().unwrap_or("").to_string())
    }
    pub fn host_text(&self) -> String {
        let hostname = self.hostname();
        match self.inner.port() {
            Some(port) => format!("{hostname}:{port}"),
            None => hostname,
        }
    }
    pub fn port_text(&self) -> String {
        self.inner.port().map(|p| p.to_string()).unwrap_or_default()
    }
    pub fn pathname(&self) -> String {
        let mut path = if self.inner.cannot_be_a_base() {
            self.inner.path().to_string()
        } else {
            self.inner.path().replace('^', "%5E")
        };
        let bytes = path.as_bytes();
        if self.inner.scheme() == "file"
            && bytes.first() == Some(&b'/')
            && bytes.get(1).is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(2) == Some(&b'|')
            && (bytes.len() == 3 || bytes.get(3) == Some(&b'/'))
        {
            path.replace_range(2..3, ":");
        }
        path
    }
    pub fn search(&self) -> String {
        self.inner
            .query()
            .filter(|q| !q.is_empty())
            .map(|q| format!("?{q}"))
            .unwrap_or_default()
    }
    pub fn hash(&self) -> String {
        self.inner
            .fragment()
            .filter(|f| !f.is_empty())
            .map(|f| format!("#{f}"))
            .unwrap_or_default()
    }
    pub fn origin(&self) -> String {
        if self.inner.scheme() == "blob" {
            return parse(self.inner.path())
                .filter(|u| matches!(u.inner.scheme(), "http" | "https"))
                .map(|u| u.origin())
                .unwrap_or_else(|| "null".into());
        }
        if self.ascii_host.is_some() && default_port(self.inner.scheme()).is_some() {
            format!("{}//{}", self.protocol(), self.host_text())
        } else {
            self.inner.origin().ascii_serialization()
        }
    }
    pub fn search_params(&self) -> Vec<(String, String)> {
        self.inner.query_pairs().into_owned().collect()
    }
}
