use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;

use crate::codec::{CODECS, encode};
use crate::errors::Result;
use crate::io::{choice, parse, print_json, read_input, write_output};
use crate::tool::Tool;

pub static TOOL: Tool = Tool {
    name: "encode",
    aliases: &[],
    description: "Encode teks/file ke base64, base64url, hex, atau URL",
    category: "Encoding",
    run,
};

const HELP: &str = r##"dt encode [options] [teks]

Encode teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -t, --type <jenis>  base64, base64url, hex, url (default: base64)
  -f, --file <path>   Encode isi file
      --json          Keluaran JSON: {type, output}
  -h, --help          Tampilkan bantuan

Kebalikannya: dt decode. Contoh: dt encode -t url "a b&c"
"##;

#[derive(Parser)]
#[command(name = "dt encode", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 't', long = "type", overrides_with = "kind")]
    kind: Option<String>,
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("encode", args)?;
    let codec = choice(a.kind.as_deref(), CODECS, CODECS[0].1, "type")?;
    let data = read_input(&a.positionals, a.file.as_deref(), "Teks yang akan di-encode")?;
    let output = encode(&data, codec);
    if a.json {
        print_json(&serde_json::json!({ "type": codec.name(), "output": output }));
    } else {
        write_output(format!("{output}\n").as_bytes());
    }
    Ok(())
}
