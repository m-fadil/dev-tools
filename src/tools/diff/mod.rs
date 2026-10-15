mod lines;

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};

use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::errors::{CliError, Result, usage};
use crate::io::{integer, parse, print_json};
use crate::tool::Tool;

use lines::{build_hunks, diff_lines, format_unified, split_lines};

pub static TOOL: Tool = Tool {
    name: "diff",
    aliases: &[],
    description: "Bandingkan dua file (sha256, diff baris, atau offset byte beda)",
    category: "Data",
    run,
};

const SNIFF_BYTES: usize = 8000;
const MAX_TEXT_DIFF_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LINE_PRODUCT: usize = 25_000_000;
const NOTE: &str = "Gunakan `git diff --no-index <a> <b>` untuk diff yang lebih besar.";
const HELP: &str = r#"dt diff [options] <a> <b>

Bandingkan dua file. Identik: cetak sha256 dan ukuran. Berbeda: cetak ukuran dan
sha256 keduanya, lalu diff baris (teks) atau offset byte pertama yang berbeda (biner).
  -U, --context <n>  Baris konteks di sekitar perubahan, 0-20 (default: 3)
      --json         Keluaran satu dokumen JSON (identical, a, b, kind, firstDifferingByte, hunks)
  -h, --help         Tampilkan bantuan

File dianggap biner jika ada byte NUL di 8000 byte pertama. Diff baris dilewati
(hanya ringkasan) jika file > 5 MiB atau jumlah baris a x b > 25.000.000.
Exit: 0 identik, 1 berbeda ATAU error (error ditulis ke stderr, hasil beda ke stdout), 2 penggunaan salah.
Contoh: dt diff -U 1 lama.txt baru.txt"#;

#[derive(Parser)]
#[command(name = "dt diff", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(short = 'U', long)]
    context: Option<String>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

struct Summary {
    path: String,
    size: u64,
    sha256: String,
    head: Vec<u8>,
}

fn read_error(path: &str, error: io::Error) -> CliError {
    let message = if error.kind() == io::ErrorKind::NotFound {
        format!("File tidak ditemukan: {path}")
    } else if error.kind() == io::ErrorKind::IsADirectory {
        format!("Bukan file (direktori): {path}")
    } else {
        format!("Gagal membaca {path}: {error}")
    };
    CliError::Failed(message)
}

fn open_file(path: &str) -> Result<File> {
    let file = File::open(path).map_err(|e| read_error(path, e))?;
    if file.metadata().map_err(|e| read_error(path, e))?.is_dir() {
        return Err(CliError::Failed(format!("Bukan file (direktori): {path}")));
    }
    Ok(file)
}

fn summarize(path: String) -> Result<Summary> {
    let mut file = open_file(&path)?;
    let mut hash = Sha256::new();
    let mut head = Vec::with_capacity(SNIFF_BYTES);
    let mut size = 0;
    let mut buf = [0u8; 65536];
    loop {
        let n = match file.read(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(read_error(&path, e)),
        };
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        size += n as u64;
        let capture = n.min(SNIFF_BYTES - head.len());
        head.extend_from_slice(&buf[..capture]);
    }
    Ok(Summary {
        path,
        size,
        sha256: hex::encode(hash.finalize()),
        head,
    })
}

fn read_block(file: &mut File, buf: &mut [u8], path: &str) -> Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(read_error(path, e)),
        }
    }
    Ok(filled)
}

fn first_difference(a: &str, b: &str) -> Result<u64> {
    let mut fa = open_file(a)?;
    let mut fb = open_file(b)?;
    let mut ba = [0u8; 65536];
    let mut bb = [0u8; 65536];
    let mut offset = 0;
    loop {
        let na = read_block(&mut fa, &mut ba, a)?;
        let nb = read_block(&mut fb, &mut bb, b)?;
        let n = na.min(nb);
        if let Some(i) = ba[..n].iter().zip(&bb[..n]).position(|(a, b)| a != b) {
            return Ok(offset + i as u64);
        }
        if na != nb || n == 0 {
            return Ok(offset + n as u64);
        }
        offset += n as u64;
    }
}

fn emit(mut doc: Value, a: &Summary, b: &Summary) {
    for (label, s) in [("a", a), ("b", b)] {
        doc[label] = json!({ "path": s.path, "size": s.size, "sha256": s.sha256 });
    }
    print_json(&doc);
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("diff", args)?;
    if a.positionals.len() != 2 {
        return Err(usage("Butuh tepat dua file: dt diff <a> <b>"));
    }
    let context = integer(a.context.as_deref(), 3, 0, 20, "context")? as usize;
    let json_output = a.json;
    let mut paths = a.positionals.into_iter();
    // Length was checked above; consuming the strings avoids copying paths.
    let (Some(path_a), Some(path_b)) = (paths.next(), paths.next()) else {
        return Err(usage("Butuh tepat dua file: dt diff <a> <b>"));
    };
    let a = summarize(path_a)?;
    let b = summarize(path_b)?;
    if a.size == b.size && a.sha256 == b.sha256 {
        if json_output {
            emit(json!({ "identical": true, "kind": "identical" }), &a, &b);
        } else {
            println!("Identik (sha256 {}, {} byte)", a.sha256, a.size);
        }
        return Ok(());
    }
    if !json_output {
        println!("Berbeda");
        for (label, s) in [("a", &a), ("b", &b)] {
            println!("  {label}: {}  {} byte  sha256 {}", s.path, s.size, s.sha256);
        }
    }
    if a.head.contains(&0) || b.head.contains(&0) {
        let offset = first_difference(&a.path, &b.path)?;
        if json_output {
            emit(
                json!({ "identical": false, "kind": "binary", "firstDifferingByte": offset }),
                &a,
                &b,
            );
        } else {
            println!("Biner, berbeda di byte offset {offset}");
        }
    } else if a.size > MAX_TEXT_DIFF_BYTES || b.size > MAX_TEXT_DIFF_BYTES {
        if json_output {
            emit(json!({ "identical": false, "kind": "summary-only" }), &a, &b);
        } else {
            println!("File lebih dari 5 MiB, diff baris dilewati. {NOTE}");
        }
    } else {
        let data_a = std::fs::read(&a.path).map_err(|e| read_error(&a.path, e))?;
        let data_b = std::fs::read(&b.path).map_err(|e| read_error(&b.path, e))?;
        let lines_a = split_lines(&data_a);
        let lines_b = split_lines(&data_b);
        if lines_a.len().saturating_mul(lines_b.len()) > MAX_LINE_PRODUCT {
            if json_output {
                emit(json!({ "identical": false, "kind": "summary-only" }), &a, &b);
            } else {
                println!(
                    "Terlalu banyak baris ({} x {}), diff baris dilewati. {NOTE}",
                    lines_a.len(),
                    lines_b.len()
                );
            }
        } else {
            let hunks = build_hunks(&diff_lines(&lines_a, &lines_b), context);
            if json_output {
                emit(json!({ "identical": false, "kind": "text", "hunks": hunks }), &a, &b);
            } else {
                println!("--- {}\n+++ {}", a.path, b.path);
                let body = format_unified(&hunks);
                if !body.is_empty() {
                    println!("{body}");
                }
            }
        }
    }
    Err(CliError::Exit(1))
}
