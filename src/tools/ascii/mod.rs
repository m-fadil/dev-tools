mod chars;

use crate::errors::{CliError, Result, usage};
use crate::io;
use crate::tool::Tool;
use chars::{describe, label, parse_value, table};
use clap::Parser;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "ascii",
    aliases: &[],
    description: "Tabel ASCII dan info karakter (desimal, hex, oktal, biner, Unicode)",
    category: "Utilitas",
    run,
};

const HELP: &str = r#"dt ascii [nilai]

Tabel ASCII atau info satu karakter.
nilai: satu karakter (A), kode desimal (65), atau hex (0x41). Satu karakter
diutamakan: "5" adalah karakter 5; kode 0-9 desimal tulis sebagai 0x05.
Karakter non-ASCII menampilkan code point Unicode, byte UTF-8, dan nilai desimal.
  -t, --table     Tabel 0-127 dalam 4 kolom (default tanpa nilai)
      --json      Keluaran JSON
  -h, --help      Tampilkan bantuan

Contoh: dt ascii A   |   dt ascii 0x0a   |   dt ascii é
"#;

#[derive(Parser)]
#[command(name = "dt ascii", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 't', long)]
    table: bool,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = io::parse("ascii", args)?;
    if a.positionals.len() > 1 {
        return Err(usage(format!(
            "Hanya satu nilai yang diizinkan: {}",
            a.positionals.join(" ")
        )));
    }
    if a.table && !a.positionals.is_empty() {
        return Err(usage("--table tidak dapat digabung dengan nilai."));
    }

    let Some(value) = a.positionals.first() else {
        let rows = table();
        if a.json {
            io::print_json(&serde_json::json!({ "table": rows }));
            return Ok(());
        }
        let cell = |i: usize| {
            let c = &rows[i];
            format!("{:>3} {} {} {:<3}", c.decimal, &c.hex[2..], c.octal, label(c))
        };
        let header = ["dec hx oct chr"; 4].join(" | ");
        let rule: String = header.chars().map(|c| if c == '|' { '+' } else { '-' }).collect();
        println!("{header}");
        println!("{rule}");
        for r in 0..32 {
            let line = [0, 32, 64, 96].map(|c| cell(c + r)).join(" | ");
            println!("{}", line.trim_end());
        }
        return Ok(());
    };

    let info = describe(parse_value(value).map_err(CliError::Failed)?);
    if a.json {
        io::print_json(&serde_json::to_value(&info).map_err(|e| CliError::Failed(e.to_string()))?);
        return Ok(());
    }
    println!("desimal   {}", info.decimal);
    println!("hex       {}", info.hex);
    println!("oktal     {}", info.octal);
    println!("biner     {}", info.binary);
    if !info.ascii {
        println!("unicode   {}", info.unicode);
        println!("utf-8     {}", info.utf8.join(" "));
    }
    if let Some(n) = &info.name {
        println!("nama      {n}");
    }
    match &info.character {
        Some(c) => println!("karakter  {c}"),
        None if info.name.is_none() => println!("karakter  (tidak tercetak)"),
        None => {}
    }
    if let Some(d) = &info.description {
        println!("deskripsi {d}");
    }
    Ok(())
}
