mod convert;

use crate::errors::Result;
use crate::io;
use crate::tool::Tool;
use clap::Parser;
use convert::{STYLES, Style, convert, split_lines};
use serde_json::{Map, Value, json};
use std::ffi::OsString;
use std::path::PathBuf;

pub static TOOL: Tool = Tool {
    name: "case",
    aliases: &[],
    description: "Ubah gaya penulisan teks (camel, snake, kebab, title, dst.)",
    category: "Data",
    run,
};

const HELP: &str = "dt case [options] [teks]

Ubah gaya penulisan teks/identifier. Kata dikenali dari spasi, _ - . /, batas
camelCase, akronim (HTTPServerError -> http server error), angka, dan huruf Unicode.
Teks multibaris: tiap baris dikonversi sendiri. Tanpa teks: baca stdin.
  -t, --to <gaya>     camel, pascal, snake, screaming, kebab, train, dot, path, title, sentence, lower, upper, all
                      (default: all, tampilkan semua gaya berlabel)
  -f, --file <path>   Baca teks dari file
      --json          Keluaran JSON
  -h, --help          Tampilkan bantuan

Contoh: dt case -t snake parseURL2Json   |   echo \"hello world\" | dt case
";

#[derive(Parser)]
#[command(name = "dt case", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 't', long, overrides_with = "to")]
    to: Option<String>,
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = io::parse("case", args)?;
    let mut allowed: Vec<(&str, Option<Style>)> = STYLES.iter().map(|&(n, s)| (n, Some(s))).collect();
    allowed.push(("all", None));
    let to = io::choice(a.to.as_deref(), &allowed, None, "to")?;
    let data = io::read_input(&a.positionals, a.file.as_deref(), "Teks yang akan dikonversi")?;
    let lines = split_lines(&String::from_utf8_lossy(&data));

    if a.json {
        let items: Vec<Value> = lines
            .iter()
            .map(|input| {
                let output = match to {
                    None => {
                        let mut m = Map::new();
                        for (name, s) in STYLES {
                            m.insert(name.to_string(), Value::String(convert(input, s)));
                        }
                        Value::Object(m)
                    }
                    Some(s) => Value::String(convert(input, s)),
                };
                json!({ "input": input, "output": output })
            })
            .collect();
        let style = allowed.iter().find(|(_, s)| *s == to).map_or("all", |(n, _)| n);
        io::print_json(&json!({ "style": style, "lines": items }));
        return Ok(());
    }
    if let Some(s) = to {
        for line in &lines {
            println!("{}", convert(line, s));
        }
        return Ok(());
    }
    let width = STYLES.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            println!();
        }
        for (name, s) in STYLES {
            println!("{name:<width$}  {}", convert(line, s));
        }
    }
    Ok(())
}
