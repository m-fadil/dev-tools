mod data;
mod parse;
mod table;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;
use serde_json::{Map, Value, json};

use crate::errors::{CliError, Result, usage};
use crate::io::{choice, integer, parse as parse_args, print_json, read_input};
use crate::tool::Tool;

use data::{build_table, find_duplicate, js_trim, resolve_select};
use parse::{detect_delimiter, parse_csv, resolve_delimiter, to_csv_line};
use table::render_table;

pub static TOOL: Tool = Tool {
    name: "csv",
    aliases: &[],
    description: "Baca CSV: tabel, JSON, TSV, pilih kolom, statistik",
    category: "Data",
    run,
};

const HELP: &str = r#"dt csv [options] [teks]

Baca CSV (RFC 4180) dan tampilkan sebagai tabel, JSON, TSV, atau CSV.
Input dari argumen, --file, atau stdin. Satu BOM awal dibuang; CRLF/LF diterima.
  -d, --delimiter <d>     comma | semicolon | tab | pipe | satu karakter (default: comma)
      --detect            Tebak delimiter dari rekaman pertama; seri/tidak ada = koma (tidak dengan -d)
      --no-header         Baris pertama adalah data; kolom diberi nama c1..cn
  -F, --format <fmt>      table | json | tsv | csv (default: table)
      --max-width <n>     Lebar maks. sel pada tabel, 1-1000 (default: 40), lebihnya dipotong …
      --select <kolom,..> Pilih kolom menurut nama header atau nomor (mulai 1)
      --limit <n>         Tampilkan hanya n baris data pertama
      --stats             Cetak jumlah baris/kolom dan sel kosong per kolom
      --strict            Baris dengan jumlah kolom berbeda = error (default: peringatan)
      --json              Sama dengan -F json
      --file <path>       Baca CSV dari berkas
  -h, --help              Tampilkan bantuan

JSON: array objek (nilai selalu string), atau array array dengan --no-header.
Baris kosong dilewati. Baris tidak rata diberi peringatan di stderr (nomor baris
= urutan rekaman, header = 1) dan dilengkapi sel kosong. Keluaran csv selalu
ber-delimiter koma; tsv me-escape tab/newline sebagai \t \n \r \\.
Exit: 0 sukses, 1 CSV tidak valid / kolom tidak ada / --strict dilanggar, 2 penggunaan salah.
Contoh: dt csv --detect --file data.csv   |   printf 'a,b\n1,2\n' | dt csv --json"#;

#[derive(Parser)]
#[command(name = "dt csv", override_help = HELP, disable_version_flag = true, args_override_self = true)]
struct Args {
    #[arg(short = 'd', long)]
    delimiter: Option<String>,
    #[arg(long)]
    detect: bool,
    #[arg(long)]
    no_header: bool,
    #[arg(short = 'F', long)]
    format: Option<String>,
    #[arg(long)]
    max_width: Option<String>,
    #[arg(long)]
    select: Option<String>,
    #[arg(long)]
    limit: Option<String>,
    #[arg(long)]
    stats: bool,
    #[arg(long)]
    strict: bool,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    file: Option<PathBuf>,
    positionals: Vec<String>,
}

fn tsv_cell(cell: &str) -> String {
    cell.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

// Object.fromEntries/JSON.stringify enumerate canonical array-index keys before other keys.
fn object_column_order(names: &[String]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by_key(|&i| {
        let index = names[i]
            .parse::<u32>()
            .ok()
            .filter(|&n| n != u32::MAX && n.to_string() == names[i]);
        (index.is_none(), index.unwrap_or(0))
    });
    order
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse_args("csv", args)?;
    if a.detect && a.delimiter.is_some() {
        return Err(usage("--detect tidak bisa digabung dengan --delimiter."));
    }
    let mut format = choice(
        a.format.as_deref(),
        &[("table", "table"), ("json", "json"), ("tsv", "tsv"), ("csv", "csv")],
        "table",
        "format",
    )?;
    if a.json {
        if a.format.is_some() && format != "json" {
            return Err(usage(format!("--json bertentangan dengan --format {format}.")));
        }
        format = "json";
    }
    let mut delimiter = ',';
    if let Some(value) = &a.delimiter {
        delimiter = resolve_delimiter(value).ok_or_else(|| usage(format!(
            "Nilai --delimiter tidak valid: {}. Pakai comma, semicolon, tab, pipe, atau satu karakter selain \" dan newline.",
            json!(value)
        )))?;
    }
    let max_width = integer(a.max_width.as_deref(), 40, 1, 1000, "max-width")? as usize;
    let limit = a
        .limit
        .as_deref()
        .map(|v| integer(Some(v), 0, 1, 9_007_199_254_740_991, "limit"))
        .transpose()?;
    if a.select.as_deref().is_some_and(|s| js_trim(s).is_empty()) {
        return Err(usage("--select tidak boleh kosong."));
    }
    let input = read_input(&a.positionals, a.file.as_deref(), "CSV:")?;
    let text = String::from_utf8_lossy(&input);
    if a.detect {
        delimiter = detect_delimiter(&text);
        eprintln!(
            "Delimiter terdeteksi: {}",
            if delimiter == '\t' {
                "tab".to_string()
            } else {
                delimiter.to_string()
            }
        );
    }
    let has_header = !a.no_header;
    let table = build_table(parse_csv(&text, delimiter).map_err(CliError::Failed)?, has_header);
    if let Some(first) = table.ragged.first() {
        if a.strict {
            return Err(CliError::Failed(format!(
                "Baris {} punya {} kolom, diharapkan {} (--strict).",
                first.row, first.cols, table.expected
            )));
        }
        for r in table.ragged.iter().take(20) {
            eprintln!(
                "peringatan: baris {} punya {} kolom, diharapkan {}",
                r.row, r.cols, table.expected
            );
        }
        if table.ragged.len() > 20 {
            eprintln!("peringatan: {} baris tidak rata lainnya", table.ragged.len() - 20);
        }
    }
    let mut names = table.names;
    let mut rows = table.rows;
    if let Some(spec) = &a.select {
        let picked = resolve_select(spec, &names)?;
        names = picked.iter().map(|&i| names[i].clone()).collect();
        rows = rows
            .into_iter()
            .map(|r| picked.iter().map(|&i| r[i].clone()).collect())
            .collect();
    }
    if a.stats {
        let mut empty = vec![0usize; names.len()];
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                empty[i] += usize::from(cell.is_empty());
            }
        }
        if format == "json" {
            let column_stats: Vec<Value> = names
                .iter()
                .enumerate()
                .map(|(i, name)| json!({ "index": i + 1, "name": name, "empty": empty[i] }))
                .collect();
            print_json(
                &json!({ "rows": rows.len(), "columns": names.len(), "ragged": table.ragged.iter().map(|r| r.row).collect::<Vec<_>>(), "columnStats": column_stats }),
            );
        } else {
            println!("Baris data: {}\nKolom: {}", rows.len(), names.len());
            if !table.ragged.is_empty() {
                println!(
                    "Baris tidak rata: {}",
                    table
                        .ragged
                        .iter()
                        .map(|r| r.row.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            println!("Sel kosong per kolom:");
            for (i, name) in names.iter().enumerate() {
                println!("  {} {}: {}", i + 1, json!(name), empty[i]);
            }
        }
        return Ok(());
    }
    if let Some(limit) = limit {
        rows.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    }
    match format {
        "json" => {
            if !has_header {
                print_json(&json!(rows));
            } else {
                if let Some(duplicate) = find_duplicate(&names) {
                    return Err(CliError::Failed(format!(
                        "Header duplikat {duplicate}; JSON objek tidak bisa dibuat. Pakai --select atau --no-header."
                    )));
                }
                let order = object_column_order(&names);
                let values: Vec<Value> = rows
                    .into_iter()
                    .map(|row| {
                        let mut object = Map::with_capacity(names.len());
                        for &i in &order {
                            object.insert(names[i].clone(), Value::String(row[i].clone()));
                        }
                        Value::Object(object)
                    })
                    .collect();
                print_json(&Value::Array(values));
            }
        }
        "table" => {
            if !names.is_empty() {
                println!("{}", render_table(&names, &rows, max_width));
            }
        }
        _ => {
            let line = |r: &[String]| {
                if format == "tsv" {
                    r.iter().map(|s| tsv_cell(s)).collect::<Vec<_>>().join("\t")
                } else {
                    to_csv_line(r, ',')
                }
            };
            if has_header {
                println!("{}", line(&names));
            }
            for row in &rows {
                println!("{}", line(row));
            }
        }
    }
    Ok(())
}
