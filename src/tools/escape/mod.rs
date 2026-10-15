mod convert;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;

use crate::errors::{CliError, Result, usage};
use crate::io::{parse, print_json, read_input, write_output};
use crate::tool::Tool;

pub static TOOL: Tool = Tool {
    name: "escape",
    aliases: &[],
    description: "Escape/unescape teks untuk JSON, HTML, URL, regex, dan shell",
    category: "Encoding",
    run,
};

const HELP: &str = r##"dt escape <mode> [options] [teks]

Escape teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
Mode:
  json    isi string JSON (RFC 8259)
  html    & < > " '  <->  &amp; &lt; &gt; &quot; &#39;
  url     percent-encoding komponen URL (RFC 3986)
  regex   escape untuk crate regex Rust (tanpa --decode)
  shell   kutip tunggal POSIX (tanpa --decode)

  -d, --decode        Balikkan (unescape)
  -f, --file <path>   Baca isi file
      --quotes        json: bungkus hasil dengan tanda kutip
      --ascii         json: escape karakter non-ASCII sebagai \uXXXX
      --plus          url --decode: ubah '+' menjadi spasi
      --json          Keluaran JSON: {mode, direction, output}
  -h, --help          Tampilkan bantuan

Saat --decode, satu akhir baris (\n atau \r\n) pada input dibuang.
Kode keluar: 0 berhasil, 1 gagal (input tidak valid), 2 salah pakai.
Contoh: dt escape json 'a "b"'   dt escape url -d 'a%20b'
"##;

#[derive(Parser)]
#[command(name = "dt escape", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'd', long, overrides_with = "decode")]
    decode: bool,
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    #[arg(long, overrides_with = "quotes")]
    quotes: bool,
    #[arg(long, overrides_with = "ascii")]
    ascii: bool,
    #[arg(long, overrides_with = "plus")]
    plus: bool,
    #[arg(long, overrides_with = "json")]
    json: bool,
    positionals: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Json,
    Html,
    Url,
    Regex,
    Shell,
}

const MODES: &[(&str, Mode)] = &[
    ("json", Mode::Json),
    ("html", Mode::Html),
    ("url", Mode::Url),
    ("regex", Mode::Regex),
    ("shell", Mode::Shell),
];

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("escape", args)?;
    let Some((name, text)) = a.positionals.split_first() else {
        return Err(usage("Mode wajib diisi: json, html, url, regex, atau shell."));
    };
    let Some(&(name, mode)) = MODES.iter().find(|(n, _)| n == name) else {
        return Err(usage(format!(
            "Mode tidak dikenal: {name}. Pilihan: json, html, url, regex, shell."
        )));
    };
    if a.decode && matches!(mode, Mode::Regex | Mode::Shell) {
        return Err(usage(format!("Mode {name} tidak mendukung --decode.")));
    }
    if (a.quotes || a.ascii) && (mode != Mode::Json || a.decode) {
        return Err(usage("--quotes dan --ascii hanya untuk 'json' tanpa --decode."));
    }
    if a.plus && (mode != Mode::Url || !a.decode) {
        return Err(usage("--plus hanya untuk 'url' dengan --decode."));
    }

    let data = read_input(text, a.file.as_deref(), "Teks yang akan diproses")?;
    let input = std::str::from_utf8(&data).map_err(|_| CliError::Failed("Input bukan UTF-8 yang valid.".into()))?;
    let output = if a.decode {
        let input = input
            .strip_suffix("\r\n")
            .or_else(|| input.strip_suffix('\n'))
            .unwrap_or(input);
        match mode {
            Mode::Json => convert::json_decode(input),
            Mode::Html => Ok(convert::html_decode(input)),
            Mode::Url => convert::url_decode(input, a.plus),
            Mode::Regex | Mode::Shell => unreachable!("rejected above"),
        }
        .map_err(CliError::Failed)?
    } else {
        match mode {
            Mode::Json => convert::json_encode(input, a.ascii, a.quotes),
            Mode::Html => convert::html_encode(input),
            Mode::Url => convert::url_encode(input),
            Mode::Regex => regex::escape(input),
            Mode::Shell => convert::shell_quote(input),
        }
    };
    if a.json {
        let direction = if a.decode { "decode" } else { "encode" };
        print_json(&serde_json::json!({ "mode": name, "direction": direction, "output": output }));
    } else {
        write_output(format!("{output}\n").as_bytes());
    }
    Ok(())
}
