use crate::errors::{CliError, Result, usage};

use super::parse::CsvRecord;

pub struct Ragged {
    pub row: usize,
    pub cols: usize,
}

pub struct CsvTable {
    /// Column names: the header row, or c1..cn without one.
    pub names: Vec<String>,
    /// Data rows padded to `names.len()`.
    pub rows: Vec<Vec<String>>,
    pub expected: usize,
    pub ragged: Vec<Ragged>,
}

/// JavaScript `String.prototype.trim` whitespace set.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    })
}

/// Builds the table; rows whose width differs from the header (or the first row) are reported as ragged and padded.
pub fn build_table(records: Vec<CsvRecord>, has_header: bool) -> CsvTable {
    let mut records = records.into_iter();
    let head = if has_header { records.next() } else { None };
    let data: Vec<CsvRecord> = records.collect();
    let expected = match &head {
        Some(h) => h.fields.len(),
        None => data.first().map_or(0, |r| r.fields.len()),
    };
    let ragged: Vec<Ragged> = data
        .iter()
        .filter(|r| r.fields.len() != expected)
        .map(|r| Ragged {
            row: r.row,
            cols: r.fields.len(),
        })
        .collect();
    let count = data.iter().map(|r| r.fields.len()).fold(expected, usize::max);
    let names = column_names(head.as_ref().map(|h| h.fields.as_slice()), count);
    let rows = data
        .into_iter()
        .map(|mut r| {
            r.fields.resize(count, String::new());
            r.fields
        })
        .collect();
    CsvTable {
        names,
        rows,
        expected,
        ragged,
    }
}

/// Header names, padded to `count` with c<n>; a generated name never collides with an existing one.
fn column_names(head: Option<&[String]>, count: usize) -> Vec<String> {
    let head = head.unwrap_or(&[]);
    let mut used: std::collections::HashSet<String> = head.iter().cloned().collect();
    (0..count)
        .map(|i| match head.get(i) {
            Some(name) => name.clone(),
            None => {
                let mut n = i + 1;
                while used.contains(&format!("c{n}")) {
                    n += 1;
                }
                let name = format!("c{n}");
                used.insert(name.clone());
                name
            }
        })
        .collect()
}

/// First duplicated column name, with its 1-based positions.
pub fn find_duplicate(names: &[String]) -> Option<String> {
    let mut first_seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut best: Option<(usize, usize)> = None;
    for (j, name) in names.iter().enumerate() {
        match first_seen.get(name.as_str()) {
            Some(&i) => {
                if best.is_none_or(|(bi, _)| i < bi) {
                    best = Some((i, j));
                }
            }
            None => {
                first_seen.insert(name, j);
            }
        }
    }
    let (i, j) = best?;
    Some(format!(
        "{} (kolom {} dan {})",
        serde_json::to_string(&names[i]).unwrap_or_default(),
        i + 1,
        j + 1
    ))
}

/// Resolves --select tokens to 0-based column indexes: exact header name first, then 1-based index.
pub fn resolve_select(spec: &str, names: &[String]) -> Result<Vec<usize>> {
    let mut picked: Vec<usize> = Vec::new();
    for raw in spec.split(',') {
        let token = js_trim(raw);
        if token.is_empty() {
            return Err(usage(format!("Nilai --select berisi kolom kosong: {spec}")));
        }
        let mut index = names.iter().position(|n| n == raw);
        if index.is_none() {
            index = names.iter().position(|n| n == token);
        }
        if index.is_none()
            && token.bytes().all(|b| b.is_ascii_digit())
            && let Ok(n) = token.parse::<usize>()
            && n >= 1
            && n <= names.len()
        {
            index = Some(n - 1);
        }
        let Some(index) = index else {
            return Err(CliError::Failed(format!(
                "Kolom tidak ditemukan: {token} (ada {} kolom).",
                names.len()
            )));
        };
        if picked.contains(&index) {
            return Err(usage(format!("Kolom dipilih dua kali pada --select: {token}")));
        }
        picked.push(index);
    }
    Ok(picked)
}
