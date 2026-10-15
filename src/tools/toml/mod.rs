use super::yaml::common::{self, Format, Options, invalid, locate, push_index, push_key, show};
use crate::errors::{CliError, Result};
use crate::io::parse;
use crate::tool::Tool;
use crate::tools::json::document::Segment;
use clap::Parser;
use serde_json::Value as Json;
use std::borrow::Cow;
use std::ffi::OsString;
use toml::Value as Toml;

pub static TOOL: Tool = Tool {
    name: "toml",
    aliases: &[],
    description: "Validasi, format, query, dan konversi TOML dari/ke JSON",
    category: "Data",
    run,
};

const HELP: &str = r#"dt toml [options] [teks]

Format, validasi, query, atau konversi TOML dari teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
Keluaran dinormalisasi (komentar tidak dipertahankan; nilai sederhana ditulis sebelum sub-tabel).
      --validate       Validasi saja: cetak "valid" (exit 0) atau error baris:kolom (exit 1)
      --to json        Keluarkan JSON (pretty; -m untuk satu baris); datetime jadi string RFC 3339
      --from json      Masukan berupa JSON, keluaran TOML (harus objek di tingkat atas; null ditolak)
  -m, --minify         Dengan --to json: satu baris tanpa spasi
  -s, --sort-keys      Urutkan key tabel secara rekursif (default: urutan input)
  -q, --query <path>   Ambil nilai, mis. a.b[0].c, $.a, ["key.dengan.titik"] (sintaks sama dengan dt json)
  -r, --raw            Dengan --query: cetak string tanpa tanda kutip
  -f, --file <path>    Baca masukan dari file
  -h, --help           Tampilkan bantuan

Exit: 0 sukses, 1 TOML/JSON tidak valid atau path tidak ditemukan atau nilai tak bisa dikonversi, 2 penggunaan salah.
Bilangan bulat dibatasi i64 (TOML); di luar itu ditolak. NaN/inf tak bisa jadi JSON. Hasil query non-tabel dicetak sebagai nilai TOML.
Contoh: dt toml -f Cargo.toml   |   dt toml --to json -q package.name -f Cargo.toml   |   dt toml --from json -f x.json
"#;

#[derive(Parser)]
#[command(name = "dt toml", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[command(flatten)]
    options: Options,
}

fn run(args: Vec<OsString>) -> Result<()> {
    common::on_big_stack("dt-toml", move || {
        let args: Args = parse("toml", args)?;
        common::execute::<Tml>(args.options, false)
    })
}

struct Tml;

fn kind(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "boolean",
        Json::Number(_) => "angka",
        Json::String(_) => "string",
        Json::Array(_) => "array",
        Json::Object(_) => "objek",
    }
}

impl Format for Tml {
    type Value = Toml;
    const NAME: &'static str = "TOML";

    fn parse(text: &str, _all: bool) -> Result<Vec<Toml>> {
        match toml::from_str::<toml::Table>(text) {
            Ok(table) => Ok(vec![Toml::Table(table)]),
            Err(e) => {
                let message = e.message();
                // TOML integers are i64; a wider one surfaces as a type error.
                let message = match message
                    .split("integer `")
                    .nth(1)
                    .and_then(|rest| rest.split('`').next())
                {
                    Some(digits) if message.contains(" as i128,") || message.contains(" as u128,") => {
                        Cow::Owned(format!("bilangan bulat di luar jangkauan 64-bit (i64): {digits}"))
                    }
                    _ if message.contains("integer number overflowed") => {
                        Cow::Borrowed("bilangan bulat di luar jangkauan 64-bit (i64)")
                    }
                    _ => Cow::Borrowed(message),
                };
                Err(invalid(e.span().map(|s| locate(text, s.start)), &message))
            }
        }
    }

    fn to_json(value: &Toml, path: &mut String) -> Result<Json> {
        Ok(match value {
            Toml::String(s) => Json::String(s.clone()),
            Toml::Integer(i) => Json::from(*i),
            Toml::Float(f) => Json::Number(
                serde_json::Number::from_f64(*f)
                    .ok_or_else(|| CliError::Failed(format!("{f} di {} tak bisa jadi JSON", show(path))))?,
            ),
            Toml::Boolean(b) => Json::Bool(*b),
            Toml::Datetime(d) => Json::String(d.to_string()),
            Toml::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    let len = push_index(path, i);
                    out.push(Self::to_json(item, path)?);
                    path.truncate(len);
                }
                Json::Array(out)
            }
            Toml::Table(table) => {
                let mut out = serde_json::Map::with_capacity(table.len());
                for (key, item) in table {
                    let len = push_key(path, key);
                    out.insert(key.clone(), Self::to_json(item, path)?);
                    path.truncate(len);
                }
                Json::Object(out)
            }
        })
    }

    fn from_json(value: Json, path: &mut String) -> Result<Toml> {
        Ok(match value {
            Json::Null => {
                return Err(CliError::Failed(format!(
                    "TOML tidak punya null: nilai null di {}",
                    show(path)
                )));
            }
            Json::Bool(b) => Toml::Boolean(b),
            Json::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Toml::Integer(i)
                } else if !n.as_str().contains(['.', 'e', 'E']) {
                    return Err(CliError::Failed(format!(
                        "bilangan bulat di {} di luar jangkauan TOML (i64): {}",
                        show(path),
                        n.as_str()
                    )));
                } else {
                    match n.as_f64() {
                        Some(f) if f.is_finite() => Toml::Float(f),
                        _ => {
                            return Err(CliError::Failed(format!(
                                "angka di {} di luar jangkauan: {}",
                                show(path),
                                n.as_str()
                            )));
                        }
                    }
                }
            }
            Json::String(s) => Toml::String(s),
            Json::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.into_iter().enumerate() {
                    let len = push_index(path, i);
                    out.push(Self::from_json(item, path)?);
                    path.truncate(len);
                }
                Toml::Array(out)
            }
            Json::Object(map) => {
                let mut out = toml::Table::new();
                for (key, item) in map {
                    let len = push_key(path, &key);
                    let value = Self::from_json(item, path)?;
                    out.insert(key, value);
                    path.truncate(len);
                }
                Toml::Table(out)
            }
        })
    }

    fn check_root(value: &Json) -> Result<()> {
        if value.is_object() {
            Ok(())
        } else {
            Err(CliError::Failed(format!(
                "TOML harus berupa tabel (objek) di tingkat atas, bukan {}",
                kind(value)
            )))
        }
    }

    fn get<'a>(value: &'a Toml, segments: &[Segment]) -> Option<&'a Toml> {
        let mut value = value;
        for segment in segments {
            value = match (segment, value) {
                (Segment::Key(key), Toml::Table(table)) => table.get(key.as_str())?,
                (Segment::Index(index), Toml::Array(items)) => items.get(usize::try_from(*index).ok()?)?,
                _ => return None,
            };
        }
        Some(value)
    }

    fn sort_keys(value: &mut Toml) {
        match value {
            Toml::Array(items) => items.iter_mut().for_each(Self::sort_keys),
            Toml::Table(table) => {
                let mut entries: Vec<(String, Toml)> = std::mem::take(table).into_iter().collect();
                entries.iter_mut().for_each(|(_, v)| Self::sort_keys(v));
                entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                table.extend(entries);
            }
            _ => {}
        }
    }

    fn as_str(value: &Toml) -> Option<&str> {
        value.as_str()
    }

    fn render(value: &Toml) -> Result<String> {
        match value {
            Toml::Table(table) => toml::to_string(table).map_err(|e| CliError::Failed(e.to_string())),
            other => Ok(format!("{other}\n")),
        }
    }
}
