use std::collections::BTreeMap;

fn space(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

#[derive(Default)]
pub struct Parsed {
    pub values: BTreeMap<String, String>,
    pub invalid: Vec<usize>,
    pub duplicates: Vec<(String, usize, usize)>,
}

fn value(rest: &str) -> Option<String> {
    let text = rest.trim_start_matches(space);
    let quote = text.chars().next();
    if matches!(quote, Some('\'' | '"')) {
        let mut chars = text.char_indices().skip(1);
        let mut result = String::new();
        while let Some((i, c)) = chars.next() {
            if Some(c) == quote {
                let tail = &text[i + c.len_utf8()..];
                if tail.trim_matches(space).is_empty()
                    || (tail.trim_matches(space).starts_with('#') && tail.starts_with(space))
                {
                    return Some(result);
                }
                return None;
            }
            if quote == Some('"')
                && c == '\\'
                && let Some((_, next)) = chars.next()
            {
                match next {
                    'n' => result.push('\n'),
                    '"' | '\\' => result.push(next),
                    _ => {
                        result.push(c);
                        result.push(next);
                    }
                }
                continue;
            }
            result.push(c);
        }
        None
    } else {
        let end = rest
            .char_indices()
            .find_map(|(i, c)| (c == '#' && rest[..i].chars().next_back().is_some_and(space)).then_some(i))
            .unwrap_or(rest.len());
        Some(rest[..end].trim_matches(space).to_string())
    }
}

pub fn parse(content: &str) -> Parsed {
    let mut result = Parsed::default();
    let mut previous = BTreeMap::new();
    for (index, raw) in content.trim_start_matches('\u{feff}').split('\n').enumerate() {
        let mut line = raw.trim_matches(space);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export")
            && rest.starts_with(space)
        {
            line = rest.trim_start_matches(space);
        }
        let parsed = line.split_once('=').and_then(|(key, rest)| {
            let key = key.trim_end_matches(space);
            let mut cs = key.chars();
            if !cs.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                || !cs.all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
            {
                return None;
            }
            value(rest).map(|v| (key.to_string(), v))
        });
        if let Some((key, value)) = parsed {
            if let Some(old) = previous.insert(key.clone(), index + 1) {
                result.duplicates.push((key.clone(), index + 1, old));
            }
            result.values.insert(key, value);
        } else {
            result.invalid.push(index + 1);
        }
    }
    result
}

pub struct Diff {
    pub only_a: Vec<String>,
    pub only_b: Vec<String>,
    pub changed: Vec<String>,
    pub same: usize,
}
pub fn diff(a: &Parsed, b: &Parsed, keys_only: bool) -> Diff {
    let mut d = Diff {
        only_a: vec![],
        only_b: vec![],
        changed: vec![],
        same: 0,
    };
    for (k, v) in &a.values {
        match b.values.get(k) {
            None => d.only_a.push(k.clone()),
            Some(w) if !keys_only && w != v => d.changed.push(k.clone()),
            _ => d.same += 1,
        }
    }
    d.only_b = b
        .values
        .keys()
        .filter(|k| !a.values.contains_key(*k))
        .cloned()
        .collect();
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting_and_duplicates() {
        let p = parse("\u{feff}export A=1\r\nA=2\nK= #c\nHEX=#fff\nQ=\"x\\n\\q\" # c\nBAD=\"x\"#bad\n");
        assert_eq!(p.values["A"], "2");
        assert_eq!(p.values["K"], "");
        assert_eq!(p.values["HEX"], "#fff");
        assert_eq!(p.values["Q"], "x\n\\q");
        assert_eq!(p.invalid, [6]);
        assert_eq!(p.duplicates, [("A".into(), 2, 1)]);
    }
}
