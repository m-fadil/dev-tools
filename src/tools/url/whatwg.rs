//! WHATWG parsing with Node's ASCII-domain fast path and current path encode set.
//! Unicode hosts retain the URL crate's full UTS #46 normalization and validation.

pub struct Url {
    inner: ::url::Url,
    ascii_host: Option<String>,
    /// Node keeps a non-empty file host and leading empty path segments that the crate drops.
    file_host: Option<String>,
    file_path: Option<String>,
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
        let (file_host, file_path) = file_fixups(text, &inner);
        return Some(Url {
            inner,
            ascii_host: None,
            file_host,
            file_path,
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
    let file_path = file_fixups(&replacement, &inner).1;
    Some(Url {
        inner,
        ascii_host: Some(host),
        file_host: None,
        file_path,
    })
}

fn is_slash(b: u8) -> bool {
    matches!(b, b'/' | b'\\')
}

/// Corrects `file:` quirks of the crate: a host followed by a drive-letter path is dropped,
/// and leading empty path segments are collapsed. Returns (host, path) overrides.
fn file_fixups(text: &str, inner: &::url::Url) -> (Option<String>, Option<String>) {
    if inner.scheme() != "file" {
        return (None, None);
    }
    let clean: String = text
        .trim_matches(|c: char| c <= '\u{20}')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
        .collect();
    let Some(rest) = clean.get(5..).filter(|_| clean[..5].eq_ignore_ascii_case("file:")) else {
        return (None, None);
    };
    let b = rest.as_bytes();
    let (hostpart, path_text) = if b.len() >= 2 && is_slash(b[0]) && is_slash(b[1]) {
        let end = rest[2..].find(['/', '\\', '?', '#']).map_or(rest.len(), |n| n + 2);
        (&rest[2..end], rest[end..].to_string())
    } else if b.first().is_some_and(|&c| is_slash(c)) {
        ("", rest.to_string())
    } else {
        ("", format!("/{rest}"))
    };
    let hb = hostpart.as_bytes();
    let drive_host = hb.len() == 2 && hb[0].is_ascii_alphabetic() && matches!(hb[1], b':' | b'|');
    let file_host = if hostpart.is_empty() || drive_host || inner.host_str().is_some() {
        None
    } else {
        ::url::Url::parse(&format!("file://{hostpart}/"))
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .filter(|h| !h.is_empty())
    };
    let path_text = if drive_host {
        format!("/{hostpart}{path_text}")
    } else {
        path_text
    };
    (file_host, Some(file_path(&path_text)))
}

fn is_drive(seg: &str) -> bool {
    let b = seg.as_bytes();
    b.len() == 2 && b[0].is_ascii_alphabetic() && matches!(b[1], b':' | b'|')
}

/// WHATWG file path state (Node/ada semantics: no host clearing, no leading-empty collapse).
fn file_path(text: &str) -> String {
    let end = text.find(['?', '#']).unwrap_or(text.len());
    let text = &text[..end];
    let text = text.strip_prefix(['/', '\\']).unwrap_or(text);
    let mut path: Vec<String> = Vec::new();
    let segs: Vec<&str> = text.split(['/', '\\']).collect();
    for (i, seg) in segs.iter().enumerate() {
        let last = i + 1 == segs.len();
        let lower = seg.to_ascii_lowercase();
        if matches!(lower.as_str(), ".." | ".%2e" | "%2e." | "%2e%2e") {
            let protected = path.len() == 1 && is_drive(&path[0]) && path[0].ends_with(':');
            if !protected {
                path.pop();
            }
            if last {
                path.push(String::new());
            }
        } else if matches!(lower.as_str(), "." | "%2e") {
            if last {
                path.push(String::new());
            }
        } else {
            let seg = if path.is_empty() && is_drive(seg) {
                format!("{}:", &seg[..1])
            } else {
                (*seg).to_string()
            };
            let encoded = ::url::Url::parse(&format!("http://x/{seg}/"))
                .map(|u| u.path()[1..u.path().len() - 1].to_string())
                .unwrap_or(seg);
            path.push(encoded);
        }
    }
    format!("/{}", path.join("/"))
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
            .or_else(|| self.file_host.clone())
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
            let mut p = self.inner.path().to_string();
            // Node percent-encodes one trailing space of an opaque path before `?`/`#`.
            if p.ends_with(' ') && (self.inner.query().is_some() || self.inner.fragment().is_some()) {
                p.pop();
                p.push_str("%20");
            }
            p
        } else {
            self.file_path
                .as_deref()
                .unwrap_or(self.inner.path())
                .replace('^', "%5E")
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
