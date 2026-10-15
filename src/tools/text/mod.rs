mod analyze;
mod norm_data;
mod normalize;

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, parser::ValueSource};
use serde_json::{Value, json};

use crate::errors::{CliError, Result, usage};
use crate::io::{integer, parse, print_json, read_input};
use crate::numfmt::js_number;
use crate::tool::Tool;

use analyze::{
    FreqOptions, LineOptions, body, frequency, is_js_space, is_letter_or_number, process_lines, reverse_text, slugify,
    stats,
};

pub static TOOL: Tool = Tool {
    name: "text",
    aliases: &[],
    description: "Alat teks: statistik, balik, slug, frekuensi kata, olah baris",
    category: "Data",
    run,
};

const SUBCOMMANDS: [&str; 5] = ["stats", "reverse", "slug", "freq", "lines"];

/// Options each subcommand accepts besides --file/--json/--help.
fn specific(command: &str) -> &'static [&'static str] {
    match command {
        "reverse" => &["lines"],
        "slug" => &["sep"],
        "freq" => &["top", "case-sensitive", "min-length"],
        "lines" => &["sort", "unique", "numbered", "reverse", "trim"],
        _ => &[],
    }
}

/// (clap id, option name) of every option besides the generic ones.
const OPTION_IDS: [(&str, &str); 12] = [
    ("file", "file"),
    ("json", "json"),
    ("lines", "lines"),
    ("sep", "sep"),
    ("top", "top"),
    ("case_sensitive", "case-sensitive"),
    ("min_length", "min-length"),
    ("sort", "sort"),
    ("unique", "unique"),
    ("numbered", "numbered"),
    ("reverse", "reverse"),
    ("trim", "trim"),
];

const HELP: &str = r#"dt text <perintah> [opsi] [teks]

Alat teks. Tanpa teks: baca stdin atau --file. Satu newline di akhir input diabaikan.
Perintah:
  stats     Statistik: karakter (code point), grafem, byte input asli, kata, baris,
            kalimat (sederhana), baris terpanjang, rata-rata panjang kata
  reverse   Balik tiap baris per grafem
              --lines             Balik urutan baris saja
  slug      Slug URL: hilangkan diakritik U+0300–U+036F, huruf kecil
              --sep <char>        Pemisah satu karakter (default: -)
  freq      Frekuensi kata
              --top <n>           Tampilkan n teratas (default: 10)
              --case-sensitive    Bedakan huruf besar/kecil (default: tidak)
              --min-length <n>    Abaikan kata lebih pendek dari n (default: 1)
  lines     Olah baris (urutan: trim, unique, sort, reverse, lalu numbered)
              --trim              Pangkas spasi di ujung tiap baris
              --unique            Hapus duplikat (kemunculan pertama)
              --sort              Urutkan menurut code point Unicode
              --reverse           Balik urutan
              --numbered          Beri nomor baris (tidak dengan --json)
Opsi umum:
  -f, --file <path>   Baca teks dari file
      --json          Keluaran JSON
  -h, --help          Tampilkan bantuan

Slug kosong menghasilkan galat (exit 1).
Contoh: dt text slug "Héllo Wörld!"   |   cat a.txt | dt text freq --top 5"#;

#[derive(Parser)]
#[command(name = "dt text", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    lines: bool,
    #[arg(long)]
    sep: Option<String>,
    #[arg(long)]
    top: Option<String>,
    #[arg(long = "case-sensitive")]
    case_sensitive: bool,
    #[arg(long = "min-length")]
    min_length: Option<String>,
    #[arg(long)]
    sort: bool,
    #[arg(long)]
    unique: bool,
    #[arg(long)]
    numbered: bool,
    #[arg(long)]
    reverse: bool,
    #[arg(long)]
    trim: bool,
    positionals: Vec<String>,
}

/// Names of the options present on the command line, in order of first appearance (like parseArgs `values`).
fn given_options(args: &[OsString]) -> Vec<&'static str> {
    let argv = std::iter::once(OsString::from("dt text")).chain(args.iter().cloned());
    let Ok(matches) = Args::command().try_get_matches_from(argv) else {
        return Vec::new();
    };
    let mut found: Vec<(usize, &'static str)> = OPTION_IDS
        .iter()
        .filter(|(id, _)| matches.value_source(id) == Some(ValueSource::CommandLine))
        .map(|(id, name)| (matches.index_of(id).unwrap_or(usize::MAX), *name))
        .collect();
    found.sort_by_key(|&(i, _)| i);
    found.into_iter().map(|(_, n)| n).collect()
}

fn emit(line: impl AsRef<str>) {
    let _ = writeln!(std::io::stdout().lock(), "{}", line.as_ref());
}

fn js_json_number(n: f64) -> Value {
    serde_json::from_str(&js_number(n)).unwrap_or(Value::Null)
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("text", args.clone())?;
    let mut positionals = a.positionals.clone();
    if positionals.is_empty() {
        return Err(usage(format!("Perintah wajib: {}", SUBCOMMANDS.join(", "))));
    }
    let sub = positionals.remove(0);
    if !SUBCOMMANDS.contains(&sub.as_str()) {
        return Err(usage(format!(
            "Perintah tidak dikenal: {sub}. Pilihan: {}",
            SUBCOMMANDS.join(", ")
        )));
    }
    let command = sub.as_str();
    for name in given_options(&args) {
        if !["file", "json", "help"].contains(&name) && !specific(command).contains(&name) {
            return Err(usage(format!("Opsi --{name} tidak berlaku untuk \"{command}\".")));
        }
    }
    if a.json && a.numbered {
        return Err(usage("--numbered tidak dapat digabung dengan --json."));
    }

    // Validate options before reading input so mistakes fail fast.
    let mut sep = "-".to_string();
    if command == "slug"
        && let Some(value) = &a.sep
    {
        sep = value.clone();
        let mut it = sep.chars();
        let single = matches!((it.next(), it.next()), (Some(_), None));
        if !single || sep.chars().any(|c| is_letter_or_number(c) || is_js_space(c)) {
            let quoted = serde_json::to_string(&sep).unwrap_or_default();
            return Err(usage(format!(
                "--sep harus satu karakter selain huruf, angka, atau spasi: {quoted}"
            )));
        }
    }
    let top = integer(a.top.as_deref(), 10, 1, 1_000_000, "top")? as usize;
    let min_length = integer(a.min_length.as_deref(), 1, 1, 1_000_000, "min-length")? as usize;

    let input = read_input(&positionals, a.file.as_deref(), "Teks")?;
    let raw = String::from_utf8_lossy(&input).into_owned();
    match command {
        "stats" => {
            let s = stats(&raw, &input);
            if a.json {
                print_json(&json!({
                    "characters": s.characters,
                    "graphemes": s.graphemes,
                    "bytes": s.bytes,
                    "words": s.words,
                    "lines": s.lines,
                    "sentences": s.sentences,
                    "longestLine": { "number": s.longest_line.number, "length": s.longest_line.length },
                    "averageWordLength": js_json_number(s.average_word_length),
                }));
                return Ok(());
            }
            emit(format!("karakter            {}", s.characters));
            emit(format!("grafem              {}", s.graphemes));
            emit(format!("byte (UTF-8)        {}", s.bytes));
            emit(format!("kata                {}", s.words));
            emit(format!("baris               {}", s.lines));
            emit(format!("kalimat             {}", s.sentences));
            let suffix = if s.longest_line.number > 0 {
                format!(" grafem (baris {})", s.longest_line.number)
            } else {
                String::new()
            };
            emit(format!("baris terpanjang    {}{suffix}", s.longest_line.length));
            emit(format!("rata-rata kata      {}", js_number(s.average_word_length)));
        }
        "reverse" => {
            let out = reverse_text(&raw, a.lines);
            if a.json {
                print_json(&json!({ "lines": out }));
            } else if !out.is_empty() {
                emit(out.join("\n"));
            }
        }
        "slug" => {
            let slug = slugify(body(&raw), &sep);
            if slug.is_empty() {
                return Err(CliError::Failed(
                    "Slug kosong: teks tidak mengandung huruf atau angka.".into(),
                ));
            }
            if a.json {
                print_json(&json!({ "slug": slug }));
            } else {
                emit(slug);
            }
        }
        "freq" => {
            let r = frequency(
                &raw,
                &FreqOptions {
                    top,
                    case_sensitive: a.case_sensitive,
                    min_length,
                },
            );
            if a.json {
                let items: Vec<Value> = r
                    .top
                    .iter()
                    .map(|(word, count)| json!({ "word": word, "count": count }))
                    .collect();
                print_json(&json!({ "total": r.total, "unique": r.unique, "top": items }));
                return Ok(());
            }
            let width = r.top.first().map_or(0, |(_, c)| c.to_string().len());
            for (word, count) in &r.top {
                emit(format!("{count:>width$}  {word}"));
            }
        }
        _ => {
            let out = process_lines(
                &raw,
                &LineOptions {
                    trim: a.trim,
                    unique: a.unique,
                    sort: a.sort,
                    reverse: a.reverse,
                },
            );
            if a.json {
                print_json(&json!({ "count": out.len(), "lines": out }));
                return Ok(());
            }
            let width = out.len().to_string().len();
            let shown: Vec<String> = if a.numbered {
                out.iter()
                    .enumerate()
                    .map(|(i, l)| format!("{:>width$}  {l}", i + 1))
                    .collect()
            } else {
                out
            };
            if !shown.is_empty() {
                emit(shown.join("\n"));
            }
        }
    }
    Ok(())
}
