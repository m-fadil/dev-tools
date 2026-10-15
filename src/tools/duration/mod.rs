mod format;
mod parse;

use crate::errors::{Result, usage};
use crate::io::{choice, parse as parse_args, print_json};
use crate::tool::Tool;
use crate::tools::time::instant::{Zone, parse_instant, utc_iso};
use clap::Parser;
use format::{UNITS, compact, iso, total};
use parse::{MS_PER_SECOND, Span};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::io::Write;

pub static TOOL: Tool = Tool {
    name: "duration",
    aliases: &[],
    description: "Konversi durasi (1h30m, detik, ISO-8601) dan hitung selisih dua waktu",
    category: "Utilitas",
    run,
};

const HELP: &str = "dt duration [options] <durasi...>
dt duration diff [options] <a> <b>

Normalkan dan konversi durasi, atau hitung selisih b - a dari dua waktu.
  -u, --unit <ms|s|m|h|d|w>  Cetak hanya total dalam satuan itu
      --json                 Keluaran satu dokumen JSON
  -h, --help                 Tampilkan bantuan

Durasi (beberapa argumen digabung dengan spasi):
  Ringkas   1h30m, \"1d 2h\", 90m, 1w2d, 1.5h, 250ms. Satuan: w d h m s ms.
            d = 24 jam, w = 7 hari. Urutan bebas, satuan tidak boleh berulang.
  Angka     tanpa satuan = detik (90 = 90s)
  ISO-8601  PT1H30M, P1DT2H, P2W (urutan W, D, T, H, M, S; huruf besar/kecil)
  Tanda -   di depan membuat durasi negatif: dt duration -90m, dt duration diff -86400 0
Tahun dan bulan ditolak karena panjangnya tidak tetap. Presisi maksimum 1 ms;
pecahan yang tidak habis dalam milidetik ditolak. Maksimum 9223372036854775807 ms.

diff: a dan b berupa ISO-8601 (2026-01-02T10:00:00+07:00, 2026-01-02T03:00:00Z) atau
epoch detik/milidetik (< 1e11 detik, selain itu milidetik). Tanpa offset dibaca UTC.

--unit: hasil tepat bila desimalnya berhingga, selain itu dibulatkan ke 9 digit.
Keluaran: Ringkas, ISO-8601, Detik, Milidetik (diff menambah Tanda: +, - atau 0).
JSON: {input, sign, compact, iso, seconds, milliseconds}; diff memakai {a, b, ...}.
Dengan --unit, JSON menambah unit dan value.
Exit: 0 sukses, 1 gagal, 2 penggunaan salah atau nilai tidak valid.
Contoh: dt duration 1h30m --unit h   |   dt duration diff 2026-01-02T00:00:00Z 2026-01-03T06:30:00Z
";

#[derive(Parser)]
#[command(name = "dt duration", disable_help_flag = true, disable_version_flag = true)]
struct Args {
    #[arg(short = 'u', long, overrides_with = "unit")]
    unit: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    positionals: Vec<String>,
}

/// Decimal text as a JSON number (kept verbatim by serde_json's arbitrary precision).
fn number(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

fn print_text(rows: &[(&str, String)]) {
    let width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0) + 2;
    let mut out = std::io::stdout().lock();
    for (label, value) in rows {
        let _ = writeln!(out, "{label}:{}{value}", " ".repeat(width - label.len() - 1));
    }
}

/// Output selection shared by both commands.
struct Output<'a> {
    unit: Option<(&'a str, u128)>,
    json: bool,
}

/// Prints a span. `json_head` and `text_head` precede the common fields (JSON object / text rows).
fn report(span: Span, out: &Output, json_head: Vec<(&str, Value)>, text_head: Vec<(&str, String)>) {
    let seconds = total(span, MS_PER_SECOND);
    let value = out.unit.map(|(name, per)| (name, total(span, per)));
    if !out.json {
        if let Some((_, value)) = value {
            let _ = writeln!(std::io::stdout(), "{value}");
            return;
        }
        let mut rows = text_head;
        rows.push(("Ringkas", compact(span)));
        rows.push(("ISO-8601", iso(span)));
        rows.push(("Detik", seconds));
        rows.push(("Milidetik", signed_ms(span)));
        print_text(&rows);
        return;
    }
    let mut doc = serde_json::Map::new();
    for (key, v) in json_head {
        doc.insert(key.to_string(), v);
    }
    doc.insert("sign".into(), json!(span.sign()));
    doc.insert("compact".into(), Value::String(compact(span)));
    doc.insert("iso".into(), Value::String(iso(span)));
    doc.insert("seconds".into(), number(&seconds));
    doc.insert("milliseconds".into(), number(&signed_ms(span)));
    if let Some((name, value)) = value {
        doc.insert("unit".into(), Value::String(name.to_string()));
        doc.insert("value".into(), number(&value));
    }
    print_json(&Value::Object(doc));
}

fn signed_ms(span: Span) -> String {
    if span.negative {
        format!("-{}", span.ms)
    } else {
        span.ms.to_string()
    }
}

fn run_convert(text: &str, out: &Output) -> Result<()> {
    let span = parse::parse(text)?;
    report(span, out, vec![("input", Value::String(text.to_string()))], Vec::new());
    Ok(())
}

fn run_diff(a: &str, b: &str, out: &Output) -> Result<()> {
    let utc = Zone::resolve(Some("+00:00"))?;
    let from = parse_instant(a, &utc, None)?;
    let to = parse_instant(b, &utc, None)?;
    let span = Span::from_signed(i128::from(to) - i128::from(from));
    let sign = match span.sign() {
        1 => "+",
        -1 => "-",
        _ => "0",
    };
    report(
        span,
        out,
        vec![("a", Value::String(utc_iso(from))), ("b", Value::String(utc_iso(to)))],
        vec![("Tanda", sign.to_string())],
    );
    Ok(())
}

/// Moves values that start with `-` but are negative durations or epochs (`-1h30m`, `-PT5M`,
/// `-86400`) behind `--`, so clap does not read them as flags. Everything else keeps its place.
fn separate_negative_values(args: Vec<OsString>) -> Vec<OsString> {
    let mut flags = Vec::new();
    let mut values = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        let text = arg.to_str().unwrap_or("");
        if text == "--" {
            values.extend(iter.by_ref());
            break;
        }
        let negative_value = text
            .strip_prefix('-')
            .and_then(|rest| rest.bytes().next())
            .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'P' | b'p'));
        if negative_value || !text.starts_with('-') || text == "-" {
            values.push(arg);
        } else {
            let takes_value = matches!(text, "-u" | "--unit");
            flags.push(arg);
            if takes_value && let Some(value) = iter.next() {
                flags.push(value);
            }
        }
    }
    if !values.is_empty() {
        flags.push(OsString::from("--"));
        flags.append(&mut values);
    }
    flags
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse_args("duration", separate_negative_values(args))?;
    if a.help {
        let _ = write!(std::io::stdout(), "{HELP}");
        return Ok(());
    }
    let unit = a
        .unit
        .as_deref()
        .map(|u| choice(Some(u), UNITS, 0, "unit").map(|per| (u, per)))
        .transpose()?;
    let out = Output { unit, json: a.json };
    let Some((first, rest)) = a.positionals.split_first() else {
        return Err(usage(
            "Durasi wajib diisi. Contoh: dt duration 1h30m (lihat dt duration -h).",
        ));
    };
    if first == "diff" {
        return match rest {
            [from, to] => run_diff(from, to, &out),
            _ => Err(usage(format!(
                "dt duration diff butuh tepat dua waktu, diterima {}.",
                rest.len()
            ))),
        };
    }
    run_convert(&a.positionals.join(" "), &out)
}
