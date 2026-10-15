use std::collections::HashMap;

use serde::Serialize;

/// Exact bytes (including any CR) without LF; unterminated last lines compare differently.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Line<'a> {
    raw: &'a [u8],
    eol: bool,
}

pub struct Op<'a> {
    kind: char,
    line: Line<'a>,
}

pub fn split_lines(data: &[u8]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    while start < data.len() {
        let nl = data[start..].iter().position(|&b| b == b'\n').map(|i| start + i);
        let end = nl.unwrap_or(data.len());
        lines.push(Line {
            raw: &data[start..end],
            eol: nl.is_some(),
        });
        start = end + 1;
    }
    lines
}

/// LCS edit script, trimming common ends and preferring deletion on equal LCS lengths.
pub fn diff_lines<'a>(a: &[Line<'a>], b: &[Line<'a>]) -> Vec<Op<'a>> {
    let mut head = 0;
    while head < a.len() && head < b.len() && a[head] == b[head] {
        head += 1;
    }
    let mut tail = 0;
    while tail < a.len() - head && tail < b.len() - head && a[a.len() - 1 - tail] == b[b.len() - 1 - tail] {
        tail += 1;
    }
    let n = a.len() - head - tail;
    let m = b.len() - head - tail;
    let mut ops = Vec::with_capacity(a.len() + b.len() - head - tail);
    ops.extend(a[..head].iter().map(|&line| Op { kind: ' ', line }));
    if n == 0 || m == 0 {
        ops.extend(a[head..head + n].iter().map(|&line| Op { kind: '-', line }));
        ops.extend(b[head..head + m].iter().map(|&line| Op { kind: '+', line }));
    } else {
        let mut ids = HashMap::new();
        let mut id = |line| {
            let next = ids.len();
            *ids.entry(line).or_insert(next)
        };
        let ai: Vec<usize> = a[head..head + n].iter().copied().map(&mut id).collect();
        let bi: Vec<usize> = b[head..head + m].iter().copied().map(id).collect();
        let w = m + 1;
        // The caller's product guard bounds the LCS by 5000, fitting u16.
        let mut lcs = vec![0u16; (n + 1) * w];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i * w + j] = if ai[i] == bi[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if ai[i] == bi[j] {
                ops.push(Op {
                    kind: ' ',
                    line: a[head + i],
                });
                i += 1;
                j += 1;
            } else if lcs[(i + 1) * w + j] >= lcs[i * w + j + 1] {
                ops.push(Op {
                    kind: '-',
                    line: a[head + i],
                });
                i += 1;
            } else {
                ops.push(Op {
                    kind: '+',
                    line: b[head + j],
                });
                j += 1;
            }
        }
        ops.extend(a[head + i..head + n].iter().map(|&line| Op { kind: '-', line }));
        ops.extend(b[head + j..head + m].iter().map(|&line| Op { kind: '+', line }));
    }
    ops.extend(a[a.len() - tail..].iter().map(|&line| Op { kind: ' ', line }));
    ops
}

#[derive(Serialize)]
pub struct HunkLine {
    op: char,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    a_start: usize,
    a_lines: usize,
    b_start: usize,
    b_lines: usize,
    lines: Vec<HunkLine>,
    #[serde(skip)]
    no_eol: Vec<usize>,
}

pub fn build_hunks(ops: &[Op<'_>], context: usize) -> Vec<Hunk> {
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter_map(|(i, op)| (op.kind != ' ').then_some(i))
        .collect();
    let mut a_before = Vec::with_capacity(ops.len() + 1);
    let mut b_before = Vec::with_capacity(ops.len() + 1);
    a_before.push(0);
    b_before.push(0);
    for (i, op) in ops.iter().enumerate() {
        a_before.push(a_before[i] + usize::from(op.kind != '+'));
        b_before.push(b_before[i] + usize::from(op.kind != '-'));
    }
    let mut hunks = Vec::new();
    let mut c = 0;
    while c < changed.len() {
        let mut last = c;
        while last + 1 < changed.len() && changed[last + 1] - changed[last] - 1 <= 2 * context {
            last += 1;
        }
        let from = changed[c].saturating_sub(context);
        let to = ops.len().min(changed[last] + 1 + context);
        let a_count = a_before[to] - a_before[from];
        let b_count = b_before[to] - b_before[from];
        let mut hunk = Hunk {
            a_start: a_before[from] + usize::from(a_count != 0),
            a_lines: a_count,
            b_start: b_before[from] + usize::from(b_count != 0),
            b_lines: b_count,
            lines: Vec::with_capacity(to - from),
            no_eol: Vec::new(),
        };
        for op in &ops[from..to] {
            if !op.line.eol {
                hunk.no_eol.push(hunk.lines.len());
            }
            hunk.lines.push(HunkLine {
                op: op.kind,
                text: String::from_utf8_lossy(op.line.raw).into_owned(),
            });
        }
        hunks.push(hunk);
        c = last + 1;
    }
    hunks
}

pub fn format_unified(hunks: &[Hunk]) -> String {
    let mut out = Vec::new();
    for h in hunks {
        out.push(format!(
            "@@ -{},{} +{},{} @@",
            h.a_start, h.a_lines, h.b_start, h.b_lines
        ));
        for (i, line) in h.lines.iter().enumerate() {
            out.push(format!("{}{}", line.op, line.text));
            if h.no_eol.contains(&i) {
                out.push("\\ Tidak ada newline di akhir file".to_string());
            }
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_keys_and_lf_only_splitting() {
        assert!(split_lines(b"").is_empty());
        assert_eq!(split_lines(b"x\r\n")[0].raw, b"x\r");
        assert_eq!(split_lines(b"\n\n").len(), 2);
        let a = split_lines(b"\xff\n");
        let b = split_lines(b"\xfe\n");
        let out = format_unified(&build_hunks(&diff_lines(&a, &b), 0));
        assert_eq!(out, "@@ -1,1 +1,1 @@\n-�\n+�");
    }

    #[test]
    fn deletion_wins_lcs_ties_and_common_suffix_is_preserved() {
        let a = split_lines(b"a\nb\nc\n");
        let b = split_lines(b"b\na\nc\n");
        assert_eq!(
            format_unified(&build_hunks(&diff_lines(&a, &b), 0)),
            "@@ -1,1 +0,0 @@\n-a\n@@ -2,0 +2,1 @@\n+a"
        );
    }
}
