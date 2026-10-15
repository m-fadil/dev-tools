mod text;

use crate::errors::{Result, usage};
use crate::io;
use crate::tool::Tool;
use crate::tools::fake::rng::from_seed;
use clap::Parser;
use serde_json::json;
use std::ffi::OsString;
use text::{Unit, generate};

pub static TOOL: Tool = Tool {
    name: "lorem",
    aliases: &[],
    description: "Buat teks lorem ipsum (kata, kalimat, atau paragraf)",
    category: "Utilitas",
    run,
};

const HELP: &str = "dt lorem [options]

Buat teks lorem ipsum.
  -w, --words <n>        Jumlah kata, 1-5000
  -s, --sentences <n>    Jumlah kalimat, 1-500
  -p, --paragraphs <n>   Jumlah paragraf, 1-100 (default: 3)
      --seed <n>         Seed bilangan bulat 0-4294967295 agar hasil dapat diulang
      --classic          Mulai dengan \"Lorem ipsum dolor sit amet, consectetur adipiscing elit\"
      --json             Keluaran JSON: {unit, count, text}
  -h, --help             Tampilkan bantuan

Pilih tepat satu satuan. Kalimat 5-15 kata, paragraf 3-7 kalimat dipisah baris kosong.
Contoh: dt lorem -w 20 --seed 1   |   dt lorem -p 2 --classic
";

#[derive(Parser)]
#[command(name = "dt lorem", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'w', long, overrides_with = "words")]
    words: Option<String>,
    #[arg(short = 's', long, overrides_with = "sentences")]
    sentences: Option<String>,
    #[arg(short = 'p', long, overrides_with = "paragraphs")]
    paragraphs: Option<String>,
    #[arg(long, overrides_with = "seed")]
    seed: Option<String>,
    #[arg(long)]
    classic: bool,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = io::parse("lorem", args)?;
    if let Some(first) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {first}")));
    }
    let given: Vec<(Unit, &str)> = [
        (Unit::Words, a.words.as_deref()),
        (Unit::Sentences, a.sentences.as_deref()),
        (Unit::Paragraphs, a.paragraphs.as_deref()),
    ]
    .into_iter()
    .filter_map(|(u, v)| v.map(|v| (u, v)))
    .collect();
    if given.len() > 1 {
        return Err(usage("Pilih tepat satu: --words, --sentences, atau --paragraphs."));
    }
    let (unit, value) = match given.first() {
        Some(&(u, v)) => (u, Some(v)),
        None => (Unit::Paragraphs, None),
    };
    let fallback = if unit == Unit::Paragraphs { 3 } else { 1 };
    let count = io::integer(value, fallback, 1, unit.limit(), unit.name())?;
    let mut rng = from_seed(a.seed.as_deref())?;
    let text = generate(&mut rng, unit, count as usize, a.classic);
    if a.json {
        io::print_json(&json!({ "unit": unit.name(), "count": count, "text": text }));
    } else {
        println!("{text}");
    }
    Ok(())
}
