use crate::errors::CliError;
fn escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' => "\\t".into(),
            '\n' => "\\n".into(),
            '\r' => "\\r".into(),
            c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' => format!("\\u{{{:x}}}", c as u32),
            c => c.to_string(),
        })
        .collect()
}
pub fn error(src: &str, col: usize, msg: impl AsRef<str>, domain: bool) -> CliError {
    let locate = if src.encode_utf16().count() > 200 {
        String::new()
    } else {
        let prefix: String = src
            .chars()
            .scan(0, |n, c| {
                *n += c.len_utf16();
                Some((*n, c))
            })
            .take_while(|(n, _)| *n < col)
            .map(|(_, c)| c)
            .collect();
        format!(
            "\n  {}\n  {}^",
            escape(src),
            " ".repeat(escape(&prefix).chars().count())
        )
    };
    let text = format!("{} (kolom {col}){locate}", msg.as_ref());
    if domain {
        CliError::Failed(text)
    } else {
        CliError::Usage(text)
    }
}
