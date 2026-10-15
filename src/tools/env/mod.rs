mod envfile;
use crate::errors::{CliError, Result, usage};
use crate::io::{parse, print_json};
use crate::tool::Tool;
use clap::Parser;
use serde_json::json;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "env",
    aliases: &[],
    description: "Bandingkan dua file .env (kunci dan nilai, nilai disamarkan)",
    category: "Data",
    run,
};
const HELP: &str = r#"dt env [options] <a> <b>

Bandingkan dua file .env: kunci yang hanya ada di satu sisi dan nilai yang berbeda.
  -s, --show-values  Tampilkan nilai (default: nilai disamarkan, hanya nama kunci)
  -k, --keys-only    Abaikan perbedaan nilai, bandingkan kunci saja
      --json         Keluaran satu dokumen JSON (onlyA, onlyB, different, same, invalidLines)
  -h, --help         Tampilkan bantuan

Format: KEY=VALUE, awalan export, komentar #, nilai '...' atau "..." (\n dan \" didukung).
Tanpa interpolasi variabel (${VAR} dibaca apa adanya). Kunci duplikat: yang terakhir dipakai (peringatan di stderr).
Baris tidak valid dilaporkan di stderr dengan nomor baris.
Exit: 0 tidak ada perbedaan, 1 ada perbedaan atau baris tidak valid, 2 penggunaan salah.
Contoh: dt env .env .env.example   |   dt env -k .env .env.production
"#;
#[derive(Parser)]
#[command(name = "dt env", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 's', long)]
    show_values: bool,
    #[arg(short = 'k', long)]
    keys_only: bool,
    #[arg(long)]
    json: bool,
    files: Vec<String>,
}
fn load(path: &str, label: &str) -> Result<envfile::Parsed> {
    let bytes = std::fs::read(path).map_err(|e| CliError::Failed(format!("Tidak bisa membaca {path}: {e}")))?;
    let result = envfile::parse(&String::from_utf8_lossy(&bytes));
    for (key, line, previous) in &result.duplicates {
        eprintln!("{label}:{line}: peringatan: {key} duplikat (baris {previous}), nilai terakhir dipakai");
    }
    for line in &result.invalid {
        eprintln!("{label}:{line}: baris tidak valid");
    }
    Ok(result)
}
fn escaped(value: &str) -> Result<String> {
    let s = serde_json::to_string(value).map_err(|e| CliError::Failed(e.to_string()))?;
    Ok(s[1..s.len() - 1].to_string())
}
fn run(args: Vec<OsString>) -> Result<()> {
    let args: Args = parse("env", args)?;
    if args.files.len() != 2 {
        return Err(usage("Berikan tepat dua file: dt env <a> <b>"));
    }
    let a = load(&args.files[0], "a")?;
    let b = load(&args.files[1], "b")?;
    let d = envfile::diff(&a, &b, args.keys_only);
    let invalid = a.invalid.len() + b.invalid.len();
    if args.json {
        let different: Vec<_> = d
            .changed
            .iter()
            .map(|key| {
                if args.show_values {
                    json!({"key":key,"a":a.values[key],"b":b.values[key]})
                } else {
                    json!({"key":key})
                }
            })
            .collect();
        let invalid_lines: Vec<_> = a
            .invalid
            .iter()
            .map(|line| json!({"file":args.files[0],"line":line}))
            .chain(b.invalid.iter().map(|line| json!({"file":args.files[1],"line":line})))
            .collect();
        print_json(
            &json!({"onlyA":d.only_a,"onlyB":d.only_b,"different":different,"same":d.same,"invalidLines":invalid_lines}),
        );
    } else {
        for (title, keys) in [("Hanya di a:", &d.only_a), ("Hanya di b:", &d.only_b)] {
            if !keys.is_empty() {
                println!("{title}");
                for key in keys {
                    println!("  {key}");
                }
            }
        }
        if !d.changed.is_empty() {
            println!("Beda nilai:");
            for key in &d.changed {
                if args.show_values {
                    println!("  {key}: {} -> {}", escaped(&a.values[key])?, escaped(&b.values[key])?);
                } else {
                    println!("  {key}");
                }
            }
        }
        println!(
            "{} sama, {} beda, {} hanya a, {} hanya b",
            d.same,
            d.changed.len(),
            d.only_a.len(),
            d.only_b.len()
        );
        if invalid > 0 {
            eprintln!("{invalid} baris tidak valid");
        }
    }
    if invalid > 0 || !d.changed.is_empty() || !d.only_a.is_empty() || !d.only_b.is_empty() {
        Err(CliError::Exit(1))
    } else {
        Ok(())
    }
}
