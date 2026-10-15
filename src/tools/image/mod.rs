mod bytes;
mod exif;
mod gif;
mod inspect;
mod jpeg;
mod png;
mod webp;

use crate::errors::{CliError, Result, usage};
use crate::io;
use crate::tool::Tool;
use clap::Parser;
use inspect::inspect_file;
use serde_json::{Map, Value};
use std::ffi::OsString;
use std::path::PathBuf;

pub static TOOL: Tool = Tool {
    name: "image",
    aliases: &[],
    description: "Metadata gambar (PNG, JPEG, GIF, WebP): ukuran, rasio, EXIF, frame",
    category: "Media",
    run,
};

const HELP: &str = r#"dt image [options] <file...>

Tampilkan metadata gambar tanpa dependensi: format (dari magic bytes, bukan ekstensi),
ukuran, rasio, dan detail per format. Didukung: PNG, JPEG (+EXIF), GIF, WebP.
  --json        Keluaran JSON: { files: [{ path, ok, format, mime, size, width, height, ... | error }] }
  -h, --help    Tampilkan bantuan

Membaca maksimal 4 MiB pertama (PNG/JPEG/WebP) atau 64 MiB (GIF, untuk menghitung frame);
bila terpotong, peringatan dicetak dan jumlah frame berarti "minimal".
Beberapa file diproses semua; kegagalan dicetak ke stderr.
Exit: 0 sukses, 1 ada file gagal dibaca/rusak/tidak dikenal, 2 opsi tidak valid.
Contoh: dt image foto.jpg logo.png   |   dt image --json anim.gif
"#;

#[derive(Parser)]
#[command(name = "dt image", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(long)]
    json: bool,
    files: Vec<PathBuf>,
}

const NOT_PRINTED: [&str; 6] = ["path", "ok", "error", "warnings", "framesAtLeast", "truncated"];

fn format_value(key: &str, value: &Value, result: &Map<String, Value>) -> String {
    match value {
        Value::Null => "-".into(),
        Value::Bool(b) => (if *b { "ya" } else { "tidak" }).into(),
        _ if key == "size" => format!("{} B", display(value)),
        _ if key == "frames" && result.get("framesAtLeast") == Some(&Value::Bool(true)) => {
            format!("minimal {}", display(value))
        }
        Value::Array(items) if items.is_empty() => "-".into(),
        Value::Array(items) => items.iter().map(display).collect::<Vec<_>>().join(", "),
        _ => display(value),
    }
}

/// JS `String(value)` for scalars.
fn display(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

/// Flattens nested objects to `parent.child` rows.
fn rows(result: &Map<String, Value>) -> Vec<(String, String)> {
    fn add(out: &mut Vec<(String, String)>, key: String, value: &Value, result: &Map<String, Value>) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    add(out, format!("{key}.{k}"), v, result);
                }
            }
            _ => {
                let text = format_value(&key, value, result);
                out.push((key, text));
            }
        }
    }
    let mut out = vec![];
    for (key, value) in result {
        if !NOT_PRINTED.contains(&key.as_str()) {
            add(&mut out, key.clone(), value, result);
        }
    }
    out
}

fn print_section(result: &Map<String, Value>) {
    println!("{}:", display(result.get("path").unwrap_or(&Value::Null)));
    let table = rows(result);
    let width = table.iter().map(|(k, _)| k.encode_utf16().count()).max().unwrap_or(0);
    for (key, value) in &table {
        let pad = width - key.encode_utf16().count();
        println!("  {key}{}  {value}", " ".repeat(pad));
    }
    if let Some(Value::Array(warnings)) = result.get("warnings") {
        for w in warnings {
            println!("  peringatan: {}", display(w));
        }
    }
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = io::parse("image", args)?;
    if a.files.is_empty() {
        return Err(usage("Sebutkan minimal satu file gambar."));
    }
    let results: Vec<Map<String, Value>> = a.files.iter().map(|p| inspect_file(p)).collect();
    let is_ok = |r: &Map<String, Value>| r.get("ok") == Some(&Value::Bool(true));
    let failed: Vec<&Map<String, Value>> = results.iter().filter(|r| !is_ok(r)).collect();

    if a.json {
        let files: Vec<Value> = results.iter().cloned().map(Value::Object).collect();
        let mut top = Map::new();
        top.insert("files".into(), Value::Array(files));
        io::print_json(&Value::Object(top));
    } else {
        for (index, result) in results.iter().filter(|r| is_ok(r)).enumerate() {
            if index > 0 {
                println!();
            }
            print_section(result);
        }
    }
    for result in &failed {
        eprintln!("{}", display(result.get("error").unwrap_or(&Value::Null)));
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(CliError::Exit(1))
    }
}
