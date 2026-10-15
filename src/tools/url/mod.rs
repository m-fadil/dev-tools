mod parse;
mod whatwg;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;

use crate::errors::{CliError, Result, usage};
use crate::io::{parse, print_json, read_input};
use crate::tool::Tool;

use parse::{format_url, parse_url};

pub static TOOL: Tool = Tool {
    name: "url",
    aliases: &[],
    description: "Uraikan URL menjadi komponen dan parameter query",
    category: "Encoding",
    run,
};

const HELP: &str = "dt url [options] [url]

Uraikan URL absolut (WHATWG URL) menjadi komponen dan parameter query.
Input dari argumen, --file, atau stdin (spasi di tepi dibuang).
  -q, --query <key>       Cetak hanya nilai terdekode dari key itu, satu per baris
      --json              Cetak seluruh struktur sebagai JSON (query: array [key, value])
      --show-password     Tampilkan password (default: disamarkan ***)
      --file <path>       Baca URL dari berkas
  -h, --help              Tampilkan bantuan

Query ditampilkan terdekode, urutan asli; key berulang menjadi baris terpisah.
Port tanpa nilai eksplisit ditampilkan \"(default n)\" untuk http, https, ftp, ws, wss.
Skema wajib ada; URL relatif tidak diselesaikan. --json tidak bisa dengan --query.
Exit: 0 sukses, 1 URL tidak valid atau key tidak ada, 2 penggunaan salah.
Contoh: dt url \"https://a.com/p?x=1&x=2#h\"   |   dt url -q x \"https://a.com/?x=1&x=2\"
";

#[derive(Parser)]
#[command(name = "dt url", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'q', long = "query")]
    query: Option<String>,
    #[arg(long = "json")]
    json: bool,
    #[arg(long = "show-password")]
    show_password: bool,
    #[arg(long = "file")]
    file: Option<PathBuf>,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("url", args)?;
    if a.json && a.query.is_some() {
        return Err(usage("--json tidak bisa digabung dengan --query."));
    }
    let bytes = read_input(&a.positionals, a.file.as_deref(), "URL:")?;
    let text = String::from_utf8_lossy(&bytes);
    let parsed = parse_url(text.trim_matches(parse::whitespace), a.show_password).map_err(CliError::Failed)?;
    if let Some(key) = &a.query {
        let found: Vec<&str> = parsed
            .query
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect();
        if found.is_empty() {
            return Err(CliError::Failed(format!("Key query tidak ditemukan: {key}")));
        }
        println!("{}", found.join("\n"));
        return Ok(());
    }
    if a.json {
        print_json(&parsed.to_json());
    } else {
        println!("{}", format_url(&parsed));
    }
    Ok(())
}
