mod probe;

use crate::errors::{CliError, Result, usage};
use crate::io::{integer, parse, print_json};
use crate::tool::Tool;
use clap::Parser;
use serde_json::json;
use std::ffi::OsString;

use probe::{CheckResult, check_port, find_free_port};

pub static TOOL: Tool = Tool {
    name: "port",
    aliases: &[],
    description: "Periksa port TCP terbuka atau cari port kosong",
    category: "Jaringan",
    run,
};

const HELP: &str = r##"dt port <check|free> [options]

Periksa port TCP atau cari port yang masih kosong.
  check <port>            Coba koneksi TCP; cetak "<host>:<port> terbuka|tertutup|timeout"
  free                    Cetak port pertama yang bisa di-bind dalam rentang --from..--to
      --host <host>       Host (default: 127.0.0.1)
      --timeout <ms>      Batas waktu koneksi untuk check, 1-60000 (default: 1000)
      --from <n>          Awal rentang free, 1-65535 (default: 3000)
      --to <n>            Akhir rentang free, 1-65535 (default: 65535)
      --json              Keluaran satu dokumen JSON (check: host, port, status open|closed|timeout; free: host, port)
  -h, --help              Tampilkan bantuan

Exit: check 0 jika terbuka, 1 jika tertutup/timeout/galat soket; free 1 jika
tidak ada port kosong; 2 penggunaan salah.
free benar-benar listen lalu menutup port, jadi port bisa terpakai lagi sebelum Anda memakainya.
Contoh: dt port check 5432   |   dt port free --from 8000 --to 8100"##;

#[derive(Parser)]
#[command(name = "dt port", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    timeout: Option<String>,
    #[arg(long)]
    from: Option<String>,
    #[arg(long)]
    to: Option<String>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn parse_port(value: &str) -> Result<u16> {
    let ok = !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit());
    match value.parse::<u64>() {
        Ok(n) if ok && (1..=65535).contains(&n) => Ok(n as u16),
        _ => Err(usage(format!("Port harus bilangan bulat 1-65535: {value}"))),
    }
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("port", args)?;
    let command = a.positionals.first().map(String::as_str);
    let rest = a.positionals.get(1..).unwrap_or(&[]);
    let command = match command {
        Some(c @ ("check" | "free")) => c,
        None => return Err(usage("Subcommand diperlukan: check atau free.")),
        Some(other) => return Err(usage(format!("Subcommand tidak dikenal: {other}"))),
    };
    let host = a.host.as_deref().unwrap_or("127.0.0.1");
    if host.is_empty() {
        return Err(usage("Nilai --host tidak boleh kosong."));
    }

    if command == "check" {
        if rest.len() != 1 {
            return Err(usage("Gunakan: dt port check <port>"));
        }
        if a.from.is_some() || a.to.is_some() {
            return Err(usage("--from dan --to hanya untuk free."));
        }
        let port = parse_port(&rest[0])?;
        let timeout = integer(a.timeout.as_deref(), 1000, 1, 60000, "timeout")?;
        let result = check_port(host, port, timeout as u64)?;
        if a.json {
            print_json(&json!({ "host": host, "port": port, "status": result.status() }));
        } else {
            println!("{host}:{port} {}", result.text());
        }
        return if result == CheckResult::Terbuka {
            Ok(())
        } else {
            Err(CliError::Exit(1))
        };
    }

    if let Some(first) = rest.first() {
        return Err(usage(format!("Argumen tidak dikenal: {first}")));
    }
    if a.timeout.is_some() {
        return Err(usage("--timeout hanya untuk check."));
    }
    let from = integer(a.from.as_deref(), 3000, 1, 65535, "from")?;
    let to = integer(a.to.as_deref(), 65535, 1, 65535, "to")?;
    if from > to {
        return Err(usage(format!(
            "--from ({from}) tidak boleh lebih besar dari --to ({to})."
        )));
    }
    let Some(port) = find_free_port(host, from as u16, to as u16)? else {
        return Err(CliError::Failed(format!(
            "Tidak ada port kosong di {host} pada rentang {from}-{to}."
        )));
    };
    if a.json {
        print_json(&json!({ "host": host, "port": port }));
    } else {
        println!("{port}");
    }
    Ok(())
}
