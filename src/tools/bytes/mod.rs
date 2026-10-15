mod units;
use crate::{
    errors::{Result, usage},
    io::{integer, parse, print_json},
    tool::Tool,
};
use clap::Parser;
use std::ffi::OsString;
use units::*;
pub static TOOL: Tool = Tool {
    name: "bytes",
    aliases: &[],
    description: "Konversi ukuran data (KB/MB/GB dan KiB/MiB/GiB) secara eksak",
    category: "Utilitas",
    run,
};
const HELP: &str = r#"dt bytes <ukuran> [options]

Konversi ukuran data antar satuan dengan aritmetika eksak (tanpa galat float).
Ukuran: angka + satuan opsional, mis. 1536, 1.5GB, "2 GiB", 10kb. Angka tanpa
satuan = byte; desimal boleh; negatif ditolak.
Satuan: B; desimal (basis 1000) KB/kB, MB, GB, TB, PB; biner (basis 1024)
KiB, MiB, GiB, TiB, PiB. Huruf besar/kecil tidak dibedakan: kb=1000, kib=1024.
  -t, --to <satuan>       Cetak hanya nilai konversi ke satuan itu (angka saja)
  -p, --precision <n>     Angka desimal, 0-10 (default: 2); nol di ujung dibuang
      --json              Keluaran JSON
  -h, --help              Tampilkan bantuan

Hasil byte yang tidak bulat dibulatkan (setengah ke atas) dan ditandai.
Contoh: dt bytes 1.5GB   |   dt bytes "2 GiB" --to MB"#;
#[derive(Parser)]
#[command(name="dt bytes",override_help=HELP,disable_version_flag=true,args_override_self=true)]
struct Args {
    #[arg(short = 't', long)]
    to: Option<String>,
    #[arg(short = 'p', long)]
    precision: Option<String>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}
fn negative(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or("");
    let s = s.strip_prefix('.').unwrap_or(s);
    s.as_bytes().first().is_some_and(u8::is_ascii_digit)
}
fn run(args: Vec<OsString>) -> Result<()> {
    let mut rest = Vec::new();
    let mut iter = args.into_iter();
    while let Some(a) = iter.next() {
        let s = a.to_string_lossy();
        if s == "--" {
            rest.push(a);
            rest.extend(iter);
            break;
        }
        if matches!(s.as_ref(), "-p" | "--precision" | "-t" | "--to") {
            if let Some(v) = iter.next() {
                rest.push(OsString::from(format!(
                    "{}={}",
                    if s == "-p" {
                        "--precision"
                    } else if s == "-t" {
                        "--to"
                    } else {
                        &s
                    },
                    v.to_string_lossy()
                )));
            } else {
                rest.push(a);
            }
        } else if negative(&s) {
            return Err(usage(format!("Ukuran tidak boleh negatif: {s}")));
        } else {
            rest.push(a);
        }
    }
    let a: Args = parse("bytes", rest)?;
    if a.positionals.is_empty() {
        return Err(usage("Ukuran wajib diisi. Contoh: dt bytes 1.5GB"));
    }
    let p = integer(a.precision.as_deref(), 2, 0, 10, "precision")? as usize;
    let target = a
        .to
        .as_deref()
        .map(|s| find(s).ok_or_else(|| usage(format!("Nilai --to tidak valid: {s}. Pilihan: {}", NAMES.join(", ")))))
        .transpose()?;
    let input = a.positionals.join(" ");
    let (n, rounded) = parse_size(&input)?;
    let bytes = n.to_radix_string(10);
    if rounded {
        eprintln!("Peringatan: ukuran bukan byte bulat, dibulatkan menjadi {bytes} B.");
    }
    if let Some(i) = target {
        let value = format_value(&n, i, p);
        if a.json {
            print_json(
                &serde_json::json!({"input":input,"bytes":bytes,"rounded":rounded,"unit":NAMES[i],"value":value}),
            );
        } else {
            println!("{value}");
        }
        return Ok(());
    }
    let decimal = readable(&n, 1, p);
    let binary = readable(&n, 6, p);
    if a.json {
        let table = |start: usize| {
            let mut m = serde_json::Map::new();
            for (i, name) in NAMES.iter().enumerate().skip(start).take(5) {
                m.insert((*name).into(), serde_json::json!(format_value(&n, i, p)));
            }
            m
        };
        print_json(
            &serde_json::json!({"input":input,"bytes":bytes,"rounded":rounded,"precision":p,"decimal":table(1),"binary":table(6),"readable":{"decimal":decimal,"binary":binary}}),
        );
    } else {
        println!("Byte: {bytes}{}", if rounded { " (dibulatkan)" } else { "" });
        for start in [1, 6] {
            let rows: Vec<_> = (start..start + 5).map(|i| (NAMES[i], format_value(&n, i, p))).collect();
            let width = rows.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
            println!();
            for (name, value) in rows {
                println!("{name:<4} {value:>width$}");
            }
        }
        println!("\nTerbaca: {decimal} | {binary}");
    }
    Ok(())
}
