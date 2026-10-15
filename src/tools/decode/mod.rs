use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;

use crate::codec::{CODECS, decode};
use crate::errors::{CliError, Result};
use crate::io::{choice, parse, read_input, write_output};
use crate::tool::Tool;

pub static TOOL: Tool = Tool {
    name: "decode",
    aliases: &[],
    description: "Decode base64, base64url, hex, atau URL ke teks/biner",
    category: "Encoding",
    run,
};

const HELP: &str = r##"dt decode [options] [teks]

Decode teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -t, --type <jenis>  base64, base64url, hex, url (default: base64)
  -f, --file <path>   Decode isi file
  -h, --help          Tampilkan bantuan

Hasil ditulis apa adanya (byte mentah, tanpa newline tambahan saat dipipe),
sehingga data biner aman diarahkan ke file. Input tidak valid: exit 1.
Contoh: dt decode -t hex 68616c6f
"##;

#[derive(Parser)]
#[command(name = "dt decode", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 't', long = "type", overrides_with = "kind")]
    kind: Option<String>,
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("decode", args)?;
    let codec = choice(a.kind.as_deref(), CODECS, CODECS[0].1, "type")?;
    let data = read_input(&a.positionals, a.file.as_deref(), "Teks yang akan di-decode")?;
    let text = String::from_utf8_lossy(&data);
    write_output(&decode(&text, codec).map_err(CliError::Failed)?);
    Ok(())
}
