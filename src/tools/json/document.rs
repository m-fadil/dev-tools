use crate::errors::{CliError, Result, usage};
use serde::Deserialize;
use serde_json::Value;

pub const MAX_DEPTH: usize = 2000;
#[derive(Clone, Copy)]
enum State {
    Value,
    End,
    ArrayFirst,
    ArrayNext,
    ObjectFirst,
    ObjectKey,
    ObjectColon,
    ObjectNext,
}

fn syntax(text: &str, position: usize, message: String) -> CliError {
    let prefix = &text[..position.min(text.len())];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap_or("").encode_utf16().count() + 1;
    CliError::Failed(format!("{line}:{column} {message}"))
}
fn at(text: &str, i: usize, what: &str) -> CliError {
    let position = text[..i].encode_utf16().count();
    syntax(text, i, format!("{what} in JSON at position {position}"))
}
fn unexpected(text: &str, i: usize) -> CliError {
    if i == text.len() {
        return syntax(text, i, "Unexpected end of JSON input".into());
    }
    let chars: Vec<u16> = text.encode_utf16().collect();
    let p = text[..i].encode_utf16().count();
    let excerpt = if chars.len() <= 20 {
        format!("\"{text}\"")
    } else {
        let start = p.saturating_sub(10);
        let end = (p + 10).min(chars.len());
        format!(
            "{}\"{}\"{}",
            if start > 0 { "..." } else { "" },
            String::from_utf16_lossy(&chars[start..end]),
            if end < chars.len() { "..." } else { "" }
        )
    };
    let message = if matches!(text, "undefined" | "NaN" | "Infinity" | "[object Object]") {
        format!("{excerpt} is not valid JSON")
    } else {
        format!(
            "Unexpected token '{}', {excerpt} is not valid JSON",
            text[i..].chars().next().unwrap_or('\0')
        )
    };
    syntax(text, i, message)
}
fn string(text: &str, i: &mut usize) -> Result<()> {
    let b = text.as_bytes();
    *i += 1;
    while *i < b.len() {
        match b[*i] {
            b'"' => {
                *i += 1;
                return Ok(());
            }
            0..=31 => return Err(at(text, *i, "Bad control character in string literal")),
            b'\\' => {
                *i += 1;
                if *i == b.len() {
                    return Err(unexpected(text, *i));
                }
                if b[*i] == b'u' {
                    for _ in 0..4 {
                        *i += 1;
                        if *i == b.len() || !b[*i].is_ascii_hexdigit() {
                            return Err(at(text, *i, "Bad Unicode escape"));
                        }
                    }
                } else if !b"\"\\/bfnrt".contains(&b[*i]) {
                    return Err(at(text, *i, "Bad escaped character"));
                }
                *i += 1;
            }
            _ => {
                *i += text[*i..].chars().next().map_or(1, char::len_utf8);
            }
        }
    }
    Err(at(text, *i, "Unterminated string"))
}
/// The iterative legacy scanner is the sole syntax and nesting gate.
pub fn validate(text: &str) -> Result<()> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut depth = 0;
    let mut states = vec![State::End, State::Value];
    while let Some(state) = states.pop() {
        while i < b.len() && b" \t\r\n".contains(&b[i]) {
            i += 1;
        }
        let c = b.get(i).copied();
        match state {
            State::End => {
                if i < b.len() {
                    let p = text[..i].encode_utf16().count();
                    return Err(syntax(
                        text,
                        i,
                        format!("Unexpected non-whitespace character after JSON at position {p}"),
                    ));
                }
            }
            State::Value => match c {
                Some(b'{' | b'[') => {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err(CliError::Failed(format!("JSON terlalu dalam (maks {MAX_DEPTH} level)")));
                    }
                    i += 1;
                    states.push(if c == Some(b'{') {
                        State::ObjectFirst
                    } else {
                        State::ArrayFirst
                    });
                }
                Some(b'"') => string(text, &mut i)?,
                Some(b'-' | b'0'..=b'9') => {
                    if c == Some(b'-') {
                        i += 1;
                        if !b.get(i).is_some_and(u8::is_ascii_digit) {
                            return Err(at(text, i, "No number after minus sign"));
                        }
                    }
                    if b.get(i) == Some(&b'0') {
                        i += 1;
                        if b.get(i).is_some_and(u8::is_ascii_digit) {
                            return Err(at(text, i, "Unexpected number"));
                        }
                    } else {
                        while b.get(i).is_some_and(u8::is_ascii_digit) {
                            i += 1;
                        }
                    }
                    if b.get(i) == Some(&b'.') {
                        i += 1;
                        if !b.get(i).is_some_and(u8::is_ascii_digit) {
                            return Err(at(text, i, "Unterminated fractional number"));
                        }
                        while b.get(i).is_some_and(u8::is_ascii_digit) {
                            i += 1;
                        }
                    }
                    if matches!(b.get(i), Some(b'e' | b'E')) {
                        i += 1;
                        if matches!(b.get(i), Some(b'+' | b'-')) {
                            i += 1;
                        }
                        if !b.get(i).is_some_and(u8::is_ascii_digit) {
                            return Err(at(text, i, "Exponent part is missing a number"));
                        }
                        while b.get(i).is_some_and(u8::is_ascii_digit) {
                            i += 1;
                        }
                    }
                }
                _ => {
                    let mut matched = false;
                    for literal in ["true", "false", "null"] {
                        if text[i..].starts_with(literal) {
                            i += literal.len();
                            matched = true;
                            break;
                        }
                        let k = text[i..]
                            .bytes()
                            .zip(literal.bytes())
                            .take_while(|(a, b)| a == b)
                            .count();
                        if k > 0 {
                            return Err(unexpected(text, i + k));
                        }
                    }
                    if !matched {
                        return Err(unexpected(text, i));
                    }
                }
            },
            State::ArrayFirst => {
                if c == Some(b']') {
                    i += 1;
                    depth -= 1;
                } else {
                    states.extend([State::ArrayNext, State::Value]);
                }
            }
            State::ArrayNext => match c {
                Some(b']') => {
                    i += 1;
                    depth -= 1;
                }
                Some(b',') => {
                    i += 1;
                    states.extend([State::ArrayNext, State::Value]);
                }
                _ => return Err(at(text, i, "Expected ',' or ']' after array element")),
            },
            State::ObjectFirst | State::ObjectKey => {
                if matches!(state, State::ObjectFirst) && c == Some(b'}') {
                    i += 1;
                    depth -= 1;
                } else {
                    if c != Some(b'"') {
                        return Err(at(
                            text,
                            i,
                            if matches!(state, State::ObjectFirst) {
                                "Expected property name or '}'"
                            } else {
                                "Expected double-quoted property name"
                            },
                        ));
                    }
                    string(text, &mut i)?;
                    states.push(State::ObjectColon);
                }
            }
            State::ObjectColon => {
                if c != Some(b':') {
                    return Err(at(text, i, "Expected ':' after property name"));
                }
                i += 1;
                states.extend([State::ObjectNext, State::Value]);
            }
            State::ObjectNext => match c {
                Some(b'}') => {
                    i += 1;
                    depth -= 1;
                }
                Some(b',') => {
                    i += 1;
                    states.push(State::ObjectKey);
                }
                _ => return Err(at(text, i, "Expected ',' or '}' after property value")),
            },
        }
    }
    Ok(())
}

/// Reconstructs JSON.parse's diagnostic for JWT's payload usage errors.
pub fn legacy_error(text: &str) -> String {
    let Some(error) = validate(text).err() else {
        return "JSON tidak valid".into();
    };
    let error = error.to_string();
    let Some((location, message)) = error.split_once(' ') else {
        return error;
    };
    if message.contains(" at position ")
        && let Some((line, column)) = location.split_once(':')
    {
        return format!("{message} (line {line} column {column})");
    }
    message.to_string()
}

// serde_json strings are Unicode scalar values, whereas JSON.parse also accepts
// lone UTF-16 surrogates. This reversible string representation lets the legacy
// scanner remain the sole syntax gate.
const STRING_ESCAPE: char = '\u{ffff}';
pub fn prepare_strings(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains("\\u") && !text.contains(STRING_ESCAPE) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut inside = false;
    let unit = |at: usize| {
        text.get(at..at + 6)
            .and_then(|s| s.strip_prefix("\\u"))
            .and_then(|s| u16::from_str_radix(s, 16).ok())
    };
    while i < text.len() {
        let c = text[i..].chars().next().unwrap_or('\0');
        if c == '"' {
            inside = !inside;
            out.push(c);
            i += 1;
            continue;
        }
        if inside && c == '\\' {
            if let Some(u) = unit(i) {
                if (0xd800..=0xdbff).contains(&u) && unit(i + 6).is_some_and(|low| (0xdc00..=0xdfff).contains(&low)) {
                    out.push_str(&text[i..i + 12]);
                    i += 12;
                    continue;
                }
                if (0xd800..=0xdfff).contains(&u) {
                    out.push(STRING_ESCAPE);
                    out.push_str(&format!("{u:04x};"));
                    i += 6;
                    continue;
                }
                if u == 0xffff {
                    out.push(STRING_ESCAPE);
                    out.push(STRING_ESCAPE);
                    i += 6;
                    continue;
                }
            }
            out.push(c);
            i += 1;
            if let Some(next) = text[i..].chars().next() {
                out.push(next);
                i += next.len_utf8();
            }
            continue;
        }
        out.push(c);
        if inside && c == STRING_ESCAPE {
            out.push(c);
        }
        i += c.len_utf8();
    }
    std::borrow::Cow::Owned(out)
}
fn string_units(value: &str) -> Vec<u16> {
    let mut result = Vec::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == STRING_ESCAPE {
            if chars.clone().next() == Some(STRING_ESCAPE) {
                chars.next();
                result.push(0xffff);
            } else {
                let mut unit = 0u16;
                for c in chars.by_ref().take(4) {
                    unit = (unit << 4) | c.to_digit(16).unwrap_or(0) as u16;
                }
                chars.next();
                result.push(unit);
            }
        } else {
            let mut buf = [0; 2];
            result.extend_from_slice(c.encode_utf16(&mut buf));
        }
    }
    result
}
pub fn raw_string(value: &str) -> std::borrow::Cow<'_, str> {
    if value.contains(STRING_ESCAPE) {
        std::borrow::Cow::Owned(String::from_utf16_lossy(&string_units(value)))
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}
fn quote(value: &str) -> Result<String> {
    if !value.contains(STRING_ESCAPE) {
        return serde_json::to_string(value).map_err(|e| CliError::Failed(e.to_string()));
    }
    let units = string_units(value);
    let mut out = String::from("\"");
    for c in char::decode_utf16(units) {
        match c {
            Ok('"') => out.push_str("\\\""),
            Ok('\\') => out.push_str("\\\\"),
            Ok('\n') => out.push_str("\\n"),
            Ok('\r') => out.push_str("\\r"),
            Ok('\t') => out.push_str("\\t"),
            Ok('\u{8}') => out.push_str("\\b"),
            Ok('\u{c}') => out.push_str("\\f"),
            Ok(c) if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            Ok(c) => out.push(c),
            Err(e) => out.push_str(&format!("\\u{:04x}", e.unpaired_surrogate())),
        }
    }
    out.push('"');
    Ok(out)
}
pub fn parse(text: &str) -> Result<Value> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    validate(text)?;
    let prepared = prepare_strings(text);
    let mut deserializer = serde_json::Deserializer::from_str(&prepared);
    deserializer.disable_recursion_limit();
    Value::deserialize(&mut deserializer).map_err(|e| CliError::Failed(e.to_string()))
}

#[derive(Debug, PartialEq)]
pub enum Segment {
    Key(String),
    Index(u64),
}
pub fn query(path: &str) -> Result<Vec<Segment>> {
    let chars: Vec<char> = path.chars().collect();
    let mut i = usize::from(chars.first() == Some(&'$'));
    let mut first = true;
    let mut segments = vec![];
    let fail = |i: usize, why: &str| {
        usage(format!(
            "Path query tidak valid (posisi {}): {why}: {path}",
            chars[..i.min(chars.len())].iter().map(|c| c.len_utf16()).sum::<usize>() + 1
        ))
    };
    while i < chars.len() {
        if chars[i] == '[' {
            let start = i;
            if matches!(chars.get(i + 1), Some('\'' | '"')) {
                let quote = chars[i + 1];
                i += 2;
                let mut key = String::new();
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    key.push(chars[i]);
                    i += 1;
                }
                if chars.get(i) != Some(&quote) || chars.get(i + 1) != Some(&']') {
                    return Err(fail(start, "kunci berkutip tidak ditutup dengan ]"));
                }
                segments.push(Segment::Key(key));
                i += 2;
            } else {
                i += 1;
                let begin = i;
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                let s: String = chars[begin..i].iter().collect();
                let number = s.parse::<u64>().ok();
                if i == chars.len()
                    || s.is_empty()
                    || !s.bytes().all(|b| b.is_ascii_digit())
                    || (s.len() > 1 && s.starts_with('0'))
                    || number.is_none_or(|n| n > 9_007_199_254_740_991)
                {
                    return Err(fail(
                        start,
                        "indeks harus bilangan bulat tak negatif tanpa nol di depan (maks 2^53-1) di dalam []",
                    ));
                }
                if let Some(n) = number {
                    segments.push(Segment::Index(n));
                }
                i += 1;
            }
        } else {
            if chars[i] == '.' {
                i += 1;
            } else if !first {
                return Err(fail(i, "diharapkan . atau ["));
            }
            let start = i;
            while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                i += 1;
            }
            if start == i {
                if first && i == chars.len() {
                    return Ok(segments);
                }
                return Err(fail(i, "nama kunci kosong"));
            }
            segments.push(Segment::Key(chars[start..i].iter().collect()));
        }
        first = false;
    }
    Ok(segments)
}
pub fn get<'a>(root: &'a Value, segments: &[Segment]) -> Option<&'a Value> {
    let mut value = root;
    for segment in segments {
        value = match segment {
            Segment::Key(key) => {
                if key.contains(STRING_ESCAPE) {
                    value
                        .as_object()?
                        .get(&key.replace(STRING_ESCAPE, "\u{ffff}\u{ffff}"))?
                } else {
                    value.as_object()?.get(key)?
                }
            }
            Segment::Index(index) => value.as_array()?.get(usize::try_from(*index).ok()?)?,
        };
    }
    Some(value)
}
fn index_key(key: &str) -> Option<u32> {
    let n = key.parse::<u32>().ok()?;
    (n != u32::MAX && n.to_string() == key).then_some(n)
}
#[derive(Clone, Copy)]
enum DepthPolicy {
    Json,
    Unbounded,
}
/// Formatting runs on the tool's large stack, including recursive values' destruction.
pub fn stringify(value: &Value, indent: usize, sorted: bool, preserve_integers: bool) -> Result<String> {
    render(value, indent, sorted, preserve_integers, DepthPolicy::Json)
}
/// JWT has no explicit nesting cap; it must not inherit the JSON tool's policy.
pub fn stringify_jwt(value: &Value, indent: usize) -> Result<String> {
    render(value, indent, false, false, DepthPolicy::Unbounded)
}
fn render(value: &Value, indent: usize, sorted: bool, preserve_integers: bool, depth: DepthPolicy) -> Result<String> {
    fn walk(
        v: &Value,
        out: &mut String,
        indent: usize,
        level: usize,
        sorted: bool,
        preserve: bool,
        depth: DepthPolicy,
    ) -> Result<()> {
        if matches!(depth, DepthPolicy::Json) && level > MAX_DEPTH {
            return Err(CliError::Failed(format!("JSON terlalu dalam (maks {MAX_DEPTH} level)")));
        }
        let newline = |out: &mut String, n: usize| {
            if indent > 0 {
                out.push('\n');
                out.extend(std::iter::repeat_n(' ', n * indent));
            }
        };
        match v {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                let raw = n.as_str();
                if preserve && !raw.contains(['.', 'e', 'E']) && raw != "-0" {
                    out.push_str(raw);
                } else {
                    let n = raw.parse::<f64>().unwrap_or(f64::NAN);
                    if n.is_finite() {
                        out.push_str(&crate::numfmt::js_number(n));
                    } else {
                        out.push_str("null");
                    }
                }
            }
            Value::String(s) => out.push_str(&quote(s)?),
            Value::Array(values) => {
                out.push('[');
                for (i, child) in values.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, level + 1);
                    walk(child, out, indent, level + 1, sorted, preserve, depth)?;
                }
                if !values.is_empty() {
                    newline(out, level);
                }
                out.push(']');
            }
            Value::Object(values) => {
                let mut keys: Vec<_> = values
                    .keys()
                    .map(|key| (key, (sorted && key.contains(STRING_ESCAPE)).then(|| string_units(key))))
                    .collect();
                keys.sort_by(|(a, au), (b, bu)| match (index_key(a), index_key(b)) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    _ if sorted => match (au, bu) {
                        (Some(a), Some(b)) => a.cmp(b),
                        (Some(a), None) => a.iter().copied().cmp(b.encode_utf16()),
                        (None, Some(b)) => a.encode_utf16().cmp(b.iter().copied()),
                        (None, None) => a.encode_utf16().cmp(b.encode_utf16()),
                    },
                    _ => std::cmp::Ordering::Equal,
                });
                out.push('{');
                for (i, (key, _)) in keys.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    newline(out, level + 1);
                    out.push_str(&quote(key)?);
                    out.push(':');
                    if indent > 0 {
                        out.push(' ');
                    }
                    walk(&values[*key], out, indent, level + 1, sorted, preserve, depth)?;
                }
                if !values.is_empty() {
                    newline(out, level);
                }
                out.push('}');
            }
        }
        Ok(())
    }
    let mut out = String::new();
    walk(value, &mut out, indent, 0, sorted, preserve_integers, depth)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths() {
        assert_eq!(
            query("$.a[0]['b.c']").unwrap(),
            vec![Segment::Key("a".into()), Segment::Index(0), Segment::Key("b.c".into())]
        );
        for p in ["", "$", ".", "$."] {
            assert!(query(p).unwrap().is_empty());
        }
    }
    #[test]
    fn format_depth_guard() {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(|| {
                let mut v = Value::Null;
                for _ in 0..MAX_DEPTH + 2 {
                    v = Value::Array(vec![v]);
                }
                assert!(stringify(&v, 0, false, true).is_err());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
