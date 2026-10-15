pub struct CsvRecord {
    /// 1-based ordinal of the record in the input; blank lines count, embedded newlines do not.
    pub row: usize,
    pub fields: Vec<String>,
}

const DETECT_ORDER: [char; 4] = [',', ';', '\t', '|'];

/// Accepts a name (comma, semicolon, tab, pipe) or a single character.
pub fn resolve_delimiter(value: &str) -> Option<char> {
    match value {
        "comma" => return Some(','),
        "semicolon" => return Some(';'),
        "tab" => return Some('\t'),
        "pipe" => return Some('|'),
        _ => {}
    }
    let mut it = value.chars();
    let (Some(c), None) = (it.next(), it.next()) else {
        return None;
    };
    // JS `value.length !== 1` counts UTF-16 units: astral characters are rejected.
    if c.len_utf16() != 1 || c == '"' || c == '\n' || c == '\r' {
        None
    } else {
        Some(c)
    }
}

pub fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Counts `delimiter` in the first record, quoting like the parser: a quote only opens at the start of a field.
fn count_in_first_record(text: &str, delimiter: char) -> usize {
    let mut count = 0;
    let mut quoted = false;
    let mut field_start = true;
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if quoted {
            if c != '"' {
                continue;
            }
            if it.peek() == Some(&'"') {
                it.next();
            } else {
                quoted = false;
            }
            continue;
        }
        if c == '\n' || c == '\r' {
            break;
        }
        if c == '"' && field_start {
            quoted = true;
            field_start = false;
        } else if c == delimiter {
            count += 1;
            field_start = true;
        } else {
            field_start = false;
        }
    }
    count
}

/// Guesses the delimiter by counting candidates outside quotes on the first record; comma on a tie or none.
pub fn detect_delimiter(text: &str) -> char {
    let body = strip_bom(text);
    let mut best = ',';
    let mut best_count = 0;
    let mut tied = false;
    for d in DETECT_ORDER {
        let count = count_in_first_record(body, d);
        if count > best_count {
            best = d;
            best_count = count;
            tied = false;
        } else if count > 0 && count == best_count {
            tied = true;
        }
    }
    if tied { ',' } else { best }
}

/// `JSON.stringify` of the UTF-16 code unit JS sees at this position (a lone high surrogate for astral characters).
fn json_unit(c: char) -> String {
    if c.len_utf16() == 2 {
        let mut buf = [0u16; 2];
        c.encode_utf16(&mut buf);
        format!("\"\\u{:04x}\"", buf[0])
    } else {
        serde_json::to_string(&c.to_string()).unwrap_or_default()
    }
}

/// RFC 4180 parser. LF, CRLF and lone CR end a record; blank lines are skipped. A quote inside an
/// unquoted field is literal; anything between a closing quote and the next delimiter is an error.
pub fn parse_csv(input: &str, delimiter: char) -> Result<Vec<CsvRecord>, String> {
    let text = strip_bom(input);
    let bytes = text.as_bytes();
    let n = text.len();
    let mut records = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    let mut row = 1;
    let mut i = 0;
    let at = |i: usize| text[i..].chars().next();
    while i < n {
        let first = bytes[i];
        if fields.is_empty() && (first == b'\n' || first == b'\r') {
            i += if first == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            row += 1;
            continue;
        }
        let mut field = String::new();
        if first == b'"' {
            i += 1;
            loop {
                let Some(close) = text[i..].find('"').map(|p| p + i) else {
                    return Err(format!(
                        "CSV tidak valid: tanda kutip tidak ditutup (mulai di baris {row})."
                    ));
                };
                field.push_str(&text[i..close]);
                i = close + 1;
                if bytes.get(i) != Some(&b'"') {
                    break;
                }
                field.push('"');
                i += 1;
            }
            if i < n
                && let Some(after) = at(i)
                && after != delimiter
                && after != '\n'
                && after != '\r'
            {
                return Err(format!(
                    "CSV tidak valid pada baris {row}: karakter {} setelah tanda kutip penutup.",
                    json_unit(after)
                ));
            }
        } else {
            let start = i;
            while let Some(c) = at(i) {
                if c == delimiter || c == '\n' || c == '\r' {
                    break;
                }
                i += c.len_utf8();
            }
            field.push_str(&text[start..i]);
        }
        fields.push(field);
        if i >= n {
            break;
        }
        if at(i) == Some(delimiter) {
            i += delimiter.len_utf8();
            if i >= n {
                fields.push(String::new()); // trailing delimiter at end of input
            } else {
                continue;
            }
        } else {
            i += if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
        }
        records.push(CsvRecord {
            row,
            fields: std::mem::take(&mut fields),
        });
        row += 1;
    }
    if !fields.is_empty() {
        records.push(CsvRecord { row, fields });
    }
    Ok(records)
}

/// Serializes one record as RFC 4180; a lone empty field is quoted so the line is not mistaken for blank.
pub fn to_csv_line(fields: &[String], delimiter: char) -> String {
    if fields.len() == 1 && fields[0].is_empty() {
        return "\"\"".to_string();
    }
    fields
        .iter()
        .map(|f| {
            if f.contains(delimiter) || f.contains(['"', '\r', '\n']) {
                format!("\"{}\"", f.replace('"', "\"\""))
            } else {
                f.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(&delimiter.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(input: &str, d: char) -> Vec<(usize, Vec<String>)> {
        parse_csv(input, d)
            .unwrap()
            .into_iter()
            .map(|r| (r.row, r.fields))
            .collect()
    }

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn strips_exactly_one_bom() {
        assert_eq!(
            rows("\u{feff}\u{feff}a,b\n1,2", ','),
            vec![(1, v(&["\u{feff}a", "b"])), (2, v(&["1", "2"]))]
        );
    }

    #[test]
    fn detection_defaults_and_quotes() {
        for input in ["a;b|c", "a;b|c;d|e", "plain", ""] {
            assert_eq!(detect_delimiter(input), ',', "{input}");
        }
        assert_eq!(detect_delimiter("a;b|c;d"), ';');
        assert_eq!(detect_delimiter("5\" pipe,x"), ',');
        assert_eq!(detect_delimiter("5\" literal;x;y"), ';');
        assert_eq!(detect_delimiter("\"a;\"\"b;c\";d"), ';');
        assert_eq!(detect_delimiter("\"a\n,b\";c\nx,y,z"), ';');
    }

    #[test]
    fn blank_lines_and_trailing_delimiter() {
        assert_eq!(
            rows("a,\n\n,\nb,c", ','),
            vec![(1, v(&["a", ""])), (3, v(&["", ""])), (4, v(&["b", "c"]))]
        );
    }

    #[test]
    fn error_messages() {
        assert_eq!(
            parse_csv("a\n\"x\"y", ',').err().unwrap(),
            "CSV tidak valid pada baris 2: karakter \"y\" setelah tanda kutip penutup."
        );
        assert!(parse_csv("a,\"oops", ',').err().unwrap().contains("mulai di baris 1"));
        assert!(parse_csv("\"x\"😀", ',').err().unwrap().contains("\"\\ud83d\""));
    }

    #[test]
    fn serializes_lines() {
        assert_eq!(to_csv_line(&v(&[""]), ','), "\"\"");
        assert_eq!(to_csv_line(&v(&["a,b", "q\"r", "x"]), ','), "\"a,b\",\"q\"\"r\",x");
    }
}
