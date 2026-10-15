mod manifest;

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use serde_json::{Value, json};

use crate::errors::{CliError, Result, usage};
use crate::io::{choice, parse, print_json};
use crate::tool::Tool;
use crate::tools::hash::digest::{HASH_ALGORITHMS, digest_length};
use manifest::{HashFileError, digests_equal, hash_file, parse_sums};

pub static TOOL: Tool = Tool {
    name: "checksum",
    aliases: &["sum"],
    description: "Hitung dan verifikasi checksum file (format sha256sum)",
    category: "Keamanan & Kripto",
    run,
};

const HELP: &str = r##"dt checksum [options] <file...>

Cetak checksum tiap file dengan format sha256sum: "<hex>  <path>".
  -a, --algorithm <nama>  md5, sha1, sha224, sha256, sha384, sha512, sha3-256, sha3-512, blake2b512 (default: sha256)
  -c, --check <sums>      Verifikasi file di daftar checksum (tanpa argumen file)
      --json              Keluaran JSON tunggal ke stdout (ringkasan tetap ke stderr)
  -h, --help              Tampilkan bantuan

File dibaca streaming, aman untuk file besar. Direktori dan file non-reguler (FIFO, device) ditolak.
Mode --check: baris "<hex>  <path>" atau "<hex> *<path>"; baris kosong dan
"# komentar" diabaikan. Path relatif terhadap lokasi file daftar. Panjang digest
harus cocok dengan --algorithm, kalau tidak baris dianggap tidak valid.
Hasil per file: OK, GAGAL, TIDAK ADA, atau ERROR (alasan); ringkasan ke stderr.
Exit: 0 semua cocok, 1 ada yang gagal/tidak ada/tidak valid atau error, 2 penggunaan salah.
Contoh: dt checksum a.bin b.bin > SUMS   |   dt checksum -c SUMS
"##;

#[derive(Parser)]
#[command(name = "dt checksum", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'a', long, overrides_with = "algorithm")]
    algorithm: Option<String>,
    #[arg(short = 'c', long, overrides_with = "check")]
    check: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    positionals: Vec<String>,
}

macro_rules! out {
    ($($t:tt)*) => {{ let _ = writeln!(std::io::stdout(), $($t)*); }};
}
macro_rules! err {
    ($($t:tt)*) => {{ let _ = writeln!(std::io::stderr(), $($t)*); }};
}

#[derive(Clone, Copy)]
enum Status {
    Ok,
    Failed,
    Missing,
    Error,
}

fn check(sums_file: &str, algorithm: &str, json: bool) -> Result<()> {
    let raw = std::fs::read(sums_file).map_err(|e| {
        let reason = manifest::io_error(&e).reason;
        CliError::Failed(format!("{sums_file}: tidak dapat dibaca ({reason})"))
    })?;
    let raw = String::from_utf8_lossy(&raw);
    let text = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
    let parsed = parse_sums(text, digest_length(algorithm));
    for line in &parsed.invalid {
        err!("{sums_file}: baris {line} tidak valid");
    }
    if parsed.entries.is_empty() && parsed.invalid.is_empty() {
        return Err(CliError::Failed(format!("Tidak ada baris checksum di {sums_file}")));
    }

    let abs = std::path::absolute(sums_file).unwrap_or_else(|_| PathBuf::from(sums_file));
    let base = abs.parent().map(Path::to_path_buf).unwrap_or_default();
    let (mut failed, mut missing, mut errors) = (0usize, 0usize, 0usize);
    let mut results: Vec<Value> = Vec::new();
    let mut say = |path: &str, status: Status, label: &str, reason: Option<&str>| {
        let name = match status {
            Status::Ok => "ok",
            Status::Failed => "failed",
            Status::Missing => "missing",
            Status::Error => "error",
        };
        results.push(match reason {
            Some(r) => json!({ "path": path, "status": name, "error": r }),
            None => json!({ "path": path, "status": name }),
        });
        if !json {
            out!("{path}: {label}");
        }
    };
    for entry in &parsed.entries {
        match hash_file(&base.join(&entry.path), algorithm) {
            Err(HashFileError {
                code: Some("ENOENT"), ..
            }) => {
                say(&entry.path, Status::Missing, "TIDAK ADA", None);
                missing += 1;
            }
            Err(e) => {
                say(
                    &entry.path,
                    Status::Error,
                    &format!("ERROR ({})", e.reason),
                    Some(&e.reason),
                );
                errors += 1;
            }
            Ok(actual) if digests_equal(&actual, &entry.digest) => say(&entry.path, Status::Ok, "OK", None),
            Ok(_) => {
                say(&entry.path, Status::Failed, "GAGAL", None);
                failed += 1;
            }
        }
    }
    let ok = parsed.entries.len() - failed - missing - errors;
    let invalid = parsed.invalid.len();
    err!("checksum: {ok} OK, {failed} GAGAL, {missing} TIDAK ADA, {errors} ERROR, {invalid} baris tidak valid");
    let passed = failed == 0 && missing == 0 && errors == 0 && invalid == 0;
    if json {
        print_json(&json!({
            "algorithm": algorithm,
            "results": results,
            "malformed": parsed.invalid,
            "ok": ok,
            "failed": failed,
            "missing": missing,
            "errors": errors,
            "passed": passed,
        }));
    }
    if passed { Ok(()) } else { Err(CliError::Exit(1)) }
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("checksum", args)?;
    let algorithms: Vec<(&str, &str)> = HASH_ALGORITHMS.iter().map(|n| (*n, *n)).collect();
    let algorithm = choice(a.algorithm.as_deref(), &algorithms, "sha256", "algorithm")?;
    if let Some(sums) = &a.check {
        if !a.positionals.is_empty() {
            return Err(usage("--check tidak bisa dipakai bersama argumen file."));
        }
        return check(sums, algorithm, a.json);
    }
    if a.positionals.is_empty() {
        return Err(usage("Butuh minimal satu file. Lihat: dt checksum --help"));
    }
    let mut files: Vec<Value> = Vec::new();
    let mut errors: Vec<Value> = Vec::new();
    for path in &a.positionals {
        match hash_file(Path::new(path), algorithm) {
            Ok(digest) => {
                if a.json {
                    files.push(json!({ "path": path, "digest": digest }));
                } else {
                    out!("{digest}  {path}");
                }
            }
            Err(e) => {
                err!("{path}: {}", e.reason);
                errors.push(json!({ "path": path, "message": e.reason }));
            }
        }
    }
    let failed = !errors.is_empty();
    if a.json {
        print_json(&json!({ "algorithm": algorithm, "files": files, "errors": errors }));
    }
    if failed { Err(CliError::Exit(1)) } else { Ok(()) }
}
