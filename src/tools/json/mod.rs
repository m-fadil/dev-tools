pub(crate) mod document;
use crate::errors::{CliError, Result, usage};
use crate::io::{integer, parse, read_input};
use crate::tool::Tool;
use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;
pub static TOOL: Tool = Tool {
    name: "json",
    aliases: &[],
    description: "Format, minify, urutkan key, validasi, atau query JSON",
    category: "Data",
    run,
};
const HELP: &str = r#"dt json [options] [teks]

Format, validasi, atau query JSON dari teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -m, --minify         Satu baris tanpa spasi (tidak bisa dengan --indent)
      --indent <n>     Lebar indentasi 0-8 (default: 2)
  -s, --sort-keys      Urutkan key objek secara rekursif
      --check          Validasi saja: cetak "valid" (exit 0) atau error baris:kolom (exit 1)
  -q, --query <path>   Ambil nilai, mis. a.b[0].c, $.a, ["key.dengan.titik"]
  -r, --raw            Dengan --query: cetak string tanpa tanda kutip
  -f, --file <path>    Baca JSON dari file
  -h, --help           Tampilkan bantuan

Exit: 0 sukses, 1 JSON tidak valid/terlalu dalam atau path tidak ditemukan, 2 penggunaan salah.
Angka bulat di atas 2^53 dipertahankan persis (tanpa pembulatan); key bilangan bulat selalu diurutkan lebih dulu oleh JavaScript.
Nesting di atas 2000 level ditolak (exit 1). Indeks query tanpa nol di depan dan maks 2^53-1.
Contoh: dt json -s -f data.json   |   curl -s api | dt json -q 'items[0].id' -r
"#;
#[derive(Parser)]
#[command(name = "dt json", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'm', long)]
    minify: bool,
    #[arg(long)]
    indent: Option<String>,
    #[arg(short = 's', long)]
    sort_keys: bool,
    #[arg(long)]
    check: bool,
    #[arg(short = 'q', long)]
    query: Option<String>,
    #[arg(short = 'r', long)]
    raw: bool,
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,
    text: Vec<String>,
}
fn run(args: Vec<OsString>) -> Result<()> {
    std::thread::Builder::new()
        .name("dt-json".into())
        .stack_size(256 * 1024 * 1024)
        .spawn(move || body(args))?
        .join()
        .map_err(|_| CliError::Failed("Pemrosesan JSON gagal.".into()))?
}
fn body(args: Vec<OsString>) -> Result<()> {
    let args: Args = parse("json", args)?;
    let indent = integer(args.indent.as_deref(), 2, 0, 8, "indent")? as usize;
    if args.minify && args.indent.is_some() {
        return Err(usage("--minify tidak bisa dengan --indent."));
    }
    if args.raw && args.query.is_none() {
        return Err(usage("--raw hanya berlaku dengan --query."));
    }
    if args.check {
        for (name, active) in [
            ("minify", args.minify),
            ("query", args.query.is_some()),
            ("raw", args.raw),
            ("sort-keys", args.sort_keys),
            ("indent", args.indent.is_some()),
        ] {
            if active {
                return Err(usage(format!("--check tidak bisa dengan --{name}.")));
            }
        }
    }
    let segments = args.query.as_deref().map(document::query).transpose()?;
    let bytes = read_input(&args.text, args.file.as_deref(), "JSON")?;
    let text = String::from_utf8_lossy(&bytes);
    let data = document::parse(&text).map_err(|e| match e {
        CliError::Failed(message) if !message.starts_with("JSON terlalu dalam") => {
            CliError::Failed(format!("invalid: {message}"))
        }
        other => other,
    })?;
    if args.check {
        println!("valid");
        return Ok(());
    }
    let selected = if let Some(segments) = segments {
        document::get(&data, &segments)
            .ok_or_else(|| CliError::Failed(format!("path tidak ditemukan: {}", args.query.as_deref().unwrap_or(""))))?
    } else {
        &data
    };
    if args.raw
        && let Some(value) = selected.as_str()
    {
        println!("{}", document::raw_string(value));
        return Ok(());
    }
    println!(
        "{}",
        document::stringify(selected, if args.minify { 0 } else { indent }, args.sort_keys, true)?
    );
    Ok(())
}
