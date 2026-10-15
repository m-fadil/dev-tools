//! Pipeline shared by `dt yaml` and `dt toml`: input, option checks, query, key sorting and output.
use crate::errors::{CliError, Result, usage};
use crate::io::{choice, read_input, write_output};
use crate::tools::json::document::{self, Segment};
use serde::Deserialize;
use serde_json::Value as Json;
use std::path::PathBuf;

/// Options common to both tools (flattened into each tool's own `Args`).
#[derive(clap::Args)]
pub struct Options {
    #[arg(long)]
    pub validate: bool,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(short = 'm', long)]
    pub minify: bool,
    #[arg(short = 's', long)]
    pub sort_keys: bool,
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    #[arg(short = 'r', long)]
    pub raw: bool,
    #[arg(short = 'f', long)]
    pub file: Option<PathBuf>,
    pub text: Vec<String>,
}

/// One concrete format (YAML or TOML) plugged into [`execute`].
pub trait Format {
    type Value;
    /// Name shown in prompts and messages.
    const NAME: &'static str;
    /// Parses the first document, or every document when `all` is set. Errors are already `invalid: ...`.
    fn parse(text: &str, all: bool) -> Result<Vec<Self::Value>>;
    fn to_json(value: &Self::Value, path: &mut String) -> Result<Json>;
    fn from_json(value: Json, path: &mut String) -> Result<Self::Value>;
    /// Rejects JSON that cannot be the root of this format.
    fn check_root(_value: &Json) -> Result<()> {
        Ok(())
    }
    fn get<'a>(value: &'a Self::Value, segments: &[Segment]) -> Option<&'a Self::Value>;
    fn sort_keys(value: &mut Self::Value);
    fn as_str(value: &Self::Value) -> Option<&str>;
    /// Native-format text ending in a newline (empty for an empty document).
    fn render(value: &Self::Value) -> Result<String>;
}

/// Runs `body` on a thread with a large stack: nested values are converted and dropped recursively.
pub fn on_big_stack(name: &str, body: impl FnOnce() -> Result<()> + Send + 'static) -> Result<()> {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(256 * 1024 * 1024)
        .spawn(body)?
        .join()
        .map_err(|_| CliError::Failed("Pemrosesan gagal.".into()))?
}

/// `invalid: line:column message`, the shape `dt json --check` uses.
pub fn invalid(position: Option<(usize, usize)>, message: &str) -> CliError {
    match position {
        Some((line, column)) => CliError::Failed(format!("invalid: {line}:{column} {message}")),
        None => CliError::Failed(format!("invalid: {message}")),
    }
}

/// Line and column (1-based, counted in characters) of a byte offset.
pub fn locate(text: &str, offset: usize) -> (usize, usize) {
    let mut end = offset.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[start..].chars().count() + 1)
}

/// Path helpers: `a.b[0]`, with `["odd.key"]` for keys the query syntax cannot spell plainly.
pub fn push_key(path: &mut String, key: &str) -> usize {
    let len = path.len();
    if !key.is_empty() && !key.contains(['.', '[', ']', '"', '\'', '\\']) {
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(key);
    } else {
        path.push_str("[\"");
        for c in key.chars() {
            if matches!(c, '"' | '\\') {
                path.push('\\');
            }
            path.push(c);
        }
        path.push_str("\"]");
    }
    len
}

pub fn push_index(path: &mut String, index: usize) -> usize {
    use std::fmt::Write;
    let len = path.len();
    let _ = write!(path, "[{index}]");
    len
}

pub fn show(path: &str) -> &str {
    if path.is_empty() { "$" } else { path }
}

fn base_path(segments: &[Segment]) -> String {
    let mut path = String::new();
    for segment in segments {
        match segment {
            Segment::Key(key) => push_key(&mut path, key),
            Segment::Index(index) => push_index(&mut path, *index as usize),
        };
    }
    path
}

fn sort_json(value: &mut Json) {
    match value {
        Json::Array(values) => values.iter_mut().for_each(sort_json),
        Json::Object(map) => {
            map.sort_keys();
            map.values_mut().for_each(sort_json);
        }
        _ => {}
    }
}

fn json_text(value: &Json, minify: bool) -> Result<String> {
    let mut text = if minify {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .map_err(|e| CliError::Failed(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

fn missing(query: &str, document: Option<usize>) -> CliError {
    match document {
        Some(n) => CliError::Failed(format!("path tidak ditemukan: {query} (dokumen {n})")),
        None => CliError::Failed(format!("path tidak ditemukan: {query}")),
    }
}

pub fn execute<F: Format>(o: Options, all: bool) -> Result<()> {
    let to_json = choice(o.to.as_deref(), &[("json", true)], false, "to")?;
    let from_json = choice(o.from.as_deref(), &[("json", true)], false, "from")?;
    if to_json && from_json {
        return Err(usage("--to dan --from tidak bisa bersamaan."));
    }
    if o.minify && !to_json {
        return Err(usage("--minify hanya berlaku dengan --to json."));
    }
    if o.raw && o.query.is_none() {
        return Err(usage("--raw hanya berlaku dengan --query."));
    }
    if o.raw && all {
        return Err(usage("--raw tidak bisa dengan --all."));
    }
    if all && from_json {
        return Err(usage("--all tidak bisa dengan --from."));
    }
    if o.validate {
        for (name, active) in [
            ("to", to_json),
            ("from", from_json),
            ("minify", o.minify),
            ("query", o.query.is_some()),
            ("raw", o.raw),
            ("sort-keys", o.sort_keys),
            ("all", all),
        ] {
            if active {
                return Err(usage(format!("--validate tidak bisa dengan --{name}.")));
            }
        }
    }
    let query = o.query.as_deref().unwrap_or("");
    let segments = o.query.as_deref().map(document::query).transpose()?.unwrap_or_default();
    let bytes = read_input(&o.text, o.file.as_deref(), F::NAME)?;
    let text = std::str::from_utf8(&bytes).map_err(|e| {
        invalid(
            None,
            &format!("input bukan UTF-8 yang valid (byte ke-{})", e.valid_up_to() + 1),
        )
    })?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if o.validate {
        F::parse(text, true)?;
        write_output(b"valid\n");
        return Ok(());
    }

    let mut path = base_path(&segments);
    let out = if from_json {
        let mut json = parse_json(text)?;
        if o.sort_keys {
            sort_json(&mut json);
        }
        let selected = if segments.is_empty() {
            F::check_root(&json)?;
            json
        } else {
            document::get(&json, &segments)
                .cloned()
                .ok_or_else(|| missing(query, None))?
        };
        match &selected {
            Json::String(s) if o.raw => format!("{s}\n"),
            _ => F::render(&F::from_json(selected, &mut path)?)?,
        }
    } else {
        let mut docs = F::parse(text, all)?;
        let count = docs.len();
        if o.sort_keys {
            docs.iter_mut().for_each(F::sort_keys);
        }
        let mut selected = Vec::with_capacity(count);
        for (i, doc) in docs.iter().enumerate() {
            let value = if segments.is_empty() {
                doc
            } else {
                F::get(doc, &segments).ok_or_else(|| missing(query, all.then_some(i + 1)))?
            };
            selected.push(value);
        }
        if to_json {
            let mut converted = Vec::with_capacity(count);
            for value in &selected {
                converted.push(F::to_json(value, &mut path)?);
            }
            let json = if all {
                Json::Array(converted)
            } else {
                converted.pop().unwrap_or(Json::Null)
            };
            match json {
                Json::String(s) if o.raw => format!("{s}\n"),
                other => json_text(&other, o.minify)?,
            }
        } else {
            let mut out = String::new();
            for (i, value) in selected.iter().enumerate() {
                if i > 0 {
                    out.push_str("---\n");
                }
                match F::as_str(value) {
                    Some(s) if o.raw => {
                        out.push_str(s);
                        out.push('\n');
                    }
                    _ => out.push_str(&F::render(value)?),
                }
            }
            out
        }
    };
    write_output(out.as_bytes());
    Ok(())
}

/// Strict JSON through the same scanner as `dt json`, then built with serde_json (exact big integers).
fn parse_json(text: &str) -> Result<Json> {
    let wrap = |e: CliError| match e {
        CliError::Failed(message) if !message.starts_with("JSON terlalu dalam") => {
            CliError::Failed(format!("invalid: {message}"))
        }
        other => other,
    };
    document::validate(text).map_err(wrap)?;
    // validate() already capped the depth; serde_json's own 128-level limit would reject valid input.
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    Json::deserialize(&mut de)
        .and_then(|value| de.end().map(|()| value))
        .map_err(|e| invalid(Some((e.line(), e.column())), &e.to_string()))
}
