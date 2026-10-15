pub(crate) mod bigint;
pub(crate) mod jstext;
mod radix;

use std::ffi::OsString;

use clap::Parser;
use serde_json::{Map, Value, json};

use crate::errors::{CliError, Result, usage};
use crate::io::{choice, integer, parse, print_json, read_input};
use crate::tool::Tool;
use bigint::BigUint;
use jstext::js_trim;
use radix::{BASES, FormatOptions, WIDTHS, format_base, parse_number, twos_complement};

pub static TOOL: Tool = Tool {
    name: "number",
    aliases: &["num"],
    description: "Ubah bilangan antar basis 2/8/10/16 dengan ukuran bebas",
    category: "Matematika",
    run,
};

const HELP: &str = r#"dt number <nilai> [options]

Ubah bilangan bulat antar basis 2, 8, 10, 16 (ukuran bebas, BigInt).
Nilai: desimal atau berprefix 0x (hex), 0b (biner), 0o (oktal); boleh bertanda
- atau +, dan pemisah _ di antara digit (0xFF_FF). Tanpa argumen dibaca dari stdin.
  -f, --from <basis>      auto, 2, 8, 10, 16 (default: auto = menurut prefix)
  -t, --to <basis>        Cetak hanya basis itu: 2, 8, 10, 16 (tanpa prefix)
      --prefix            Dengan --to: sertakan prefix (0x, 0b, 0o)
  -g, --group             Kelompokkan biner per nibble (4 bit) dan hex per byte
                          dengan _, diisi nol di kiri
  -w, --width <bit>       Tampilkan juga komplemen dua: 8, 16, 32, 64. Nilai harus
                          muat dalam rentang bertanda atau tak bertanda
      --json              Keluaran JSON (tidak bisa dengan --to/--prefix/--group)
  -h, --help              Tampilkan bantuan

Bilangan negatif ditampilkan sebagai -0x.. Nilai negatif seperti -5 bisa
langsung ditulis sebagai argumen.
Contoh: dt number 255   |   dt number -f 2 1010 -t 16   |   dt number -1 -w 16"#;

#[derive(Parser)]
#[command(
    name = "dt number",
    override_help = HELP,
    disable_version_flag = true,
    disable_help_flag = true,
    args_override_self = true
)]
struct Args {
    #[arg(short = 'f', long)]
    from: Option<String>,
    #[arg(short = 't', long)]
    to: Option<String>,
    #[arg(long)]
    prefix: bool,
    #[arg(short = 'g', long)]
    group: bool,
    #[arg(short = 'w', long)]
    width: Option<String>,
    #[arg(long)]
    json: bool,
    #[arg(short = 'h', long)]
    help: bool,
    positionals: Vec<String>,
}

/// `-<digit>` (a negative number rather than an option).
fn looks_negative(arg: &str) -> bool {
    let b = arg.as_bytes();
    b.len() >= 2 && b[0] == b'-' && b[1].is_ascii_digit()
}

/// Lifts negative positionals out, but keeps negative option values attached for the parser.
fn lift_negatives(args: Vec<OsString>) -> (Vec<OsString>, Vec<String>) {
    let mut rest: Vec<OsString> = Vec::new();
    let mut negatives: Vec<String> = Vec::new();
    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        let text = arg.to_string_lossy().into_owned();
        if text == "--" {
            rest.push(arg);
            rest.extend(iter);
            break;
        }
        let option = match text.as_str() {
            "-f" | "--from" => Some("from"),
            "-t" | "--to" => Some("to"),
            "-w" | "--width" => Some("width"),
            _ => None,
        };
        let next_negative = iter.peek().is_some_and(|n| looks_negative(&n.to_string_lossy()));
        if let (Some(option), true) = (option, next_negative) {
            let value = iter
                .next()
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_default();
            rest.push(OsString::from(format!("--{option}={value}")));
        } else if looks_negative(&text) {
            negatives.push(text);
        } else {
            rest.push(arg);
        }
    }
    (rest, negatives)
}

fn pad_for(base: u32, width: u32) -> usize {
    let bits = match base {
        2 => 1,
        8 => 3,
        16 => 4,
        _ => return 0,
    };
    width.div_ceil(bits) as usize
}

fn base_label(base: u32) -> &'static str {
    match base {
        2 => "binary",
        8 => "octal",
        10 => "decimal",
        _ => "hex",
    }
}

fn run(args: Vec<OsString>) -> Result<()> {
    let (rest, negatives) = lift_negatives(args);
    let args: Args = parse("number", rest)?;
    if args.help {
        println!("{HELP}");
        return Err(CliError::Exit(0));
    }
    let from: Option<u32> = choice(
        args.from.as_deref(),
        &[
            ("auto", None),
            ("2", Some(2)),
            ("8", Some(8)),
            ("10", Some(10)),
            ("16", Some(16)),
        ],
        None,
        "from",
    )?;
    let to: Option<u32> = match args.to.as_deref() {
        None => None,
        Some(v) => Some(choice(
            Some(v),
            &[("2", 2), ("8", 8), ("10", 10), ("16", 16)],
            10,
            "to",
        )?),
    };
    let width: Option<u32> = match args.width.as_deref() {
        None => None,
        Some(v) => Some(integer(Some(v), 0, 1, 64, "width")? as u32),
    };
    if let Some(w) = width
        && !WIDTHS.contains(&w)
    {
        let list: Vec<String> = WIDTHS.iter().map(u32::to_string).collect();
        return Err(usage(format!(
            "Nilai --width harus salah satu dari {}: {}",
            list.join(", "),
            args.width.as_deref().unwrap_or_default()
        )));
    }
    if args.prefix && to.is_none() {
        return Err(usage("--prefix memerlukan --to."));
    }
    if args.json && (to.is_some() || args.prefix || args.group) {
        return Err(usage("--json tidak bisa digabung dengan --to, --prefix, atau --group."));
    }
    let words: Vec<String> = negatives.into_iter().chain(args.positionals).collect();
    if words.len() > 1 {
        return Err(usage(format!(
            "Hanya satu nilai yang diperbolehkan, ada {}.",
            words.len()
        )));
    }
    let text = match words.into_iter().next() {
        Some(w) => w,
        None => String::from_utf8_lossy(&read_input(&[], None, "Nilai:")?).into_owned(),
    };
    let parsed = parse_number(&text, from)?;
    let pattern: Option<BigUint> = match width {
        Some(w) => Some(twos_complement(parsed.negative, &parsed.magnitude, w)?),
        None => None,
    };

    if let Some(to) = to {
        let line = match (&pattern, width) {
            (Some(p), Some(w)) => format_base(
                false,
                p,
                to,
                FormatOptions {
                    prefix: args.prefix,
                    group: args.group,
                    pad: pad_for(to, w),
                },
            ),
            _ => format_base(
                parsed.negative,
                &parsed.magnitude,
                to,
                FormatOptions {
                    prefix: args.prefix,
                    group: args.group,
                    pad: 0,
                },
            ),
        };
        println!("{line}");
        return Ok(());
    }

    // (label, formatted) per base, in BASES order.
    let all = |negative: bool, n: &BigUint, pad: Option<[usize; 4]>| -> Vec<(&'static str, String)> {
        BASES
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                let options = FormatOptions {
                    prefix: true,
                    group: args.group && b != 10,
                    pad: pad.map_or(0, |p| p[i]),
                };
                (base_label(b), format_base(negative, n, b, options))
            })
            .collect()
    };
    let main = all(parsed.negative, &parsed.magnitude, None);
    let twos = match (&pattern, width) {
        (Some(p), Some(w)) => Some((w, all(false, p, Some([w as usize, pad_for(8, w), 0, pad_for(16, w)])))),
        _ => None,
    };
    if args.json {
        let mut doc = Map::new();
        doc.insert("input".into(), json!(js_trim(&text)));
        doc.insert("base".into(), json!(parsed.base));
        doc.insert("negative".into(), json!(parsed.negative));
        for (label, v) in &main {
            doc.insert((*label).into(), Value::String(v.clone()));
        }
        if let Some((w, rows)) = &twos {
            let mut t = Map::new();
            t.insert("width".into(), json!(w));
            for (label, v) in rows {
                t.insert((*label).into(), Value::String(v.clone()));
            }
            doc.insert("twosComplement".into(), Value::Object(t));
        }
        print_json(&Value::Object(doc));
        return Ok(());
    }
    let lines = |rows: &[(&'static str, String)]| -> Vec<String> {
        let get = |label: &str| {
            rows.iter()
                .find(|(l, _)| *l == label)
                .map(|(_, v)| v.as_str())
                .unwrap_or_default()
        };
        vec![
            format!("Desimal: {}", get("decimal")),
            format!("Hex:     {}", get("hex")),
            format!("Biner:   {}", get("binary")),
            format!("Oktal:   {}", get("octal")),
        ]
    };
    let mut out = lines(&main);
    if let Some((w, rows)) = &twos {
        out.push(String::new());
        out.push(format!("Komplemen dua ({w} bit):"));
        out.extend(lines(rows));
    }
    println!("{}", out.join("\n"));
    Ok(())
}
