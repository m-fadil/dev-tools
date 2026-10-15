use std::ffi::OsString;
use std::io::{IsTerminal, Read, Write};
use std::path::Path;

use crate::errors::{CliError, Result, usage};

/// Parses tool arguments. `--help` prints the (verbatim legacy) help and exits 0.
pub fn parse<A: clap::Parser>(tool: &str, args: Vec<OsString>) -> Result<A> {
    let argv = std::iter::once(OsString::from(format!("dt {tool}"))).chain(args);
    match A::try_parse_from(argv) {
        Ok(a) => Ok(a),
        Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
            let _ = write!(std::io::stdout(), "{e}");
            Err(CliError::Exit(0))
        }
        Err(e) => Err(usage(e.to_string().trim_end())),
    }
}

/// Validates an option value against an allowed list; `None` yields the fallback.
pub fn choice<T: Copy>(value: Option<&str>, allowed: &[(&str, T)], fallback: T, option: &str) -> Result<T> {
    let Some(v) = value else { return Ok(fallback) };
    if let Some((_, t)) = allowed.iter().find(|(name, _)| *name == v) {
        return Ok(*t);
    }
    let names: Vec<&str> = allowed.iter().map(|(n, _)| *n).collect();
    Err(usage(format!(
        "Nilai --{option} tidak valid: {v}. Pilihan: {}",
        names.join(", ")
    )))
}

/// Strict non-negative integer within [min, max] (digits only).
pub fn integer(value: Option<&str>, fallback: i64, min: i64, max: i64, option: &str) -> Result<i64> {
    let Some(v) = value else { return Ok(fallback) };
    let n = if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) {
        v.parse::<i64>().ok()
    } else {
        None
    };
    match n {
        // JS safe integer bound
        Some(n) if n <= 9_007_199_254_740_991 && n >= min && n <= max => Ok(n),
        _ => Err(usage(format!("Nilai --{option} harus bilangan bulat {min}-{max}: {v}"))),
    }
}

/// Input bytes from positional text, --file, piped stdin, or (terminal only) a prompt.
pub fn read_input(positionals: &[String], file: Option<&Path>, prompt: &str) -> Result<Vec<u8>> {
    if file.is_some() && !positionals.is_empty() {
        return Err(usage("Gunakan teks atau --file, bukan keduanya."));
    }
    if let Some(f) = file {
        return Ok(std::fs::read(f)?);
    }
    if !positionals.is_empty() {
        return Ok(positionals.join(" ").into_bytes());
    }
    if !std::io::stdin().is_terminal() {
        let mut buf = Vec::new();
        std::io::stdin().lock().read_to_end(&mut buf)?;
        return Ok(buf);
    }
    Ok(inquire::Text::new(prompt).prompt()?.into_bytes())
}

/// Writes result bytes verbatim; on a terminal ensures the prompt starts on a new line.
pub fn write_output(data: &[u8]) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(data);
    if std::io::stdout().is_terminal() && data.last() != Some(&b'\n') {
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}

/// Machine-readable result for `--json`.
pub fn print_json(value: &serde_json::Value) {
    let s = serde_json::to_string_pretty(value).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{s}");
}
