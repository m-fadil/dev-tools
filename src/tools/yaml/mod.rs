pub(crate) mod common;
use self::common::{Format, Options, invalid, push_index, push_key, show};
use crate::errors::{CliError, Result};
use crate::io::parse;
use crate::tool::Tool;
use crate::tools::json::document::Segment;
use clap::Parser;
use serde::Deserialize;
use serde_json::Value as Json;
use serde_yaml_ng::Value as Yaml;
use std::borrow::Cow;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "yaml",
    aliases: &[],
    description: "Validasi, format, query, dan konversi YAML dari/ke JSON",
    category: "Data",
    run,
};

const HELP: &str = r#"dt yaml [options] [teks]

Format, validasi, query, atau konversi YAML dari teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
Keluaran dinormalisasi (komentar tidak dipertahankan; anchor/alias dan merge key <<: diurai jadi nilai).
      --validate       Validasi saja: cetak "valid" (exit 0) atau error baris:kolom (exit 1); semua dokumen diperiksa
      --to json        Keluarkan JSON (pretty; -m untuk satu baris)
      --from json      Masukan berupa JSON, keluaran YAML
  -m, --minify         Dengan --to json: satu baris tanpa spasi
      --all            Proses semua dokumen (dipisah ---); --to json: array. Default: dokumen pertama saja
  -s, --sort-keys      Urutkan key mapping secara rekursif (default: urutan input)
  -q, --query <path>   Ambil nilai, mis. a.b[0].c, $.a, ["key.dengan.titik"] (sintaks sama dengan dt json)
  -r, --raw            Dengan --query: cetak string tanpa tanda kutip (tidak bisa dengan --all)
  -f, --file <path>    Baca masukan dari file
  -h, --help           Tampilkan bantuan

Exit: 0 sukses, 1 YAML/JSON tidak valid atau path tidak ditemukan atau nilai tak bisa dikonversi, 2 penggunaan salah.
Bilangan bulat sampai 64-bit (i64/u64) dipertahankan persis; di atasnya YAML maupun JSON ditolak (exit 1).
Key bukan string (1, true, null) jadi string di JSON; tag (!x) dibuang; NaN/Infinity tak bisa jadi JSON.
Contoh: dt yaml -f config.yml   |   dt yaml --to json -q a.b -f x.yml   |   dt yaml --from json -f x.json
"#;

#[derive(Parser)]
#[command(name = "dt yaml", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(long)]
    all: bool,
    #[command(flatten)]
    options: Options,
}

fn run(args: Vec<OsString>) -> Result<()> {
    common::on_big_stack("dt-yaml", move || {
        let args: Args = parse("yaml", args)?;
        common::execute::<Yml>(args.options, args.all)
    })
}

struct Yml;

fn failure(error: serde_yaml_ng::Error) -> CliError {
    let message = error.to_string();
    // serde_yaml_ng reads integers only up to 64 bits; wider ones surface as a type error.
    let message = match message
        .split("integer `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
    {
        Some(digits) if message.contains(" as u128,") || message.contains(" as i128,") => {
            let prefix = message.split("invalid type").next().unwrap_or("");
            format!("{prefix}bilangan bulat di luar jangkauan 64-bit (i64/u64): {digits}")
        }
        _ => message,
    };
    match error.location() {
        Some(at) => {
            let mark = format!(" at line {} column {}", at.line(), at.column());
            invalid(Some((at.line(), at.column())), &message.replacen(&mark, "", 1))
        }
        None => invalid(None, &message),
    }
}

/// Text of a scalar key, as JSON object keys must be strings.
fn key_text(key: &Yaml) -> Option<Cow<'_, str>> {
    match key {
        Yaml::String(s) => Some(Cow::Borrowed(s)),
        Yaml::Bool(b) => Some(Cow::Borrowed(if *b { "true" } else { "false" })),
        Yaml::Null => Some(Cow::Borrowed("null")),
        Yaml::Number(n) => Some(Cow::Owned(n.to_string())),
        Yaml::Tagged(tagged) => key_text(&tagged.value),
        Yaml::Sequence(_) | Yaml::Mapping(_) => None,
    }
}

/// Applies `<<` merge keys: explicit keys win, then earlier sources; own key order is kept
/// (`Value::apply_merge` would swap the last key into the removed `<<` slot).
fn merge(value: &mut Yaml) -> Result<()> {
    match value {
        Yaml::Sequence(items) => items.iter_mut().try_for_each(merge),
        Yaml::Tagged(tagged) => merge(&mut tagged.value),
        Yaml::Mapping(map) => {
            map.values_mut().try_for_each(merge)?;
            if !map.contains_key("<<") {
                return Ok(());
            }
            let mut sources = Vec::new();
            for (key, item) in std::mem::take(map) {
                if key.as_str() == Some("<<") {
                    sources.push(item);
                } else {
                    map.insert(key, item);
                }
            }
            let bad = || invalid(None, "merge key << harus berisi mapping atau daftar mapping");
            for source in sources {
                let parts = match source {
                    Yaml::Mapping(part) => vec![part],
                    Yaml::Sequence(parts) => parts
                        .into_iter()
                        .map(|part| {
                            if let Yaml::Mapping(part) = part {
                                Ok(part)
                            } else {
                                Err(bad())
                            }
                        })
                        .collect::<Result<_>>()?,
                    _ => return Err(bad()),
                };
                for part in parts {
                    for (key, item) in part {
                        map.entry(key).or_insert(item);
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn untag(mut value: &Yaml) -> &Yaml {
    while let Yaml::Tagged(tagged) = value {
        value = &tagged.value;
    }
    value
}

fn out_of_range(path: &str, what: &str, raw: &str) -> CliError {
    CliError::Failed(format!("{what} di {} di luar jangkauan: {raw}", show(path)))
}

impl Format for Yml {
    type Value = Yaml;
    const NAME: &'static str = "YAML";

    fn parse(text: &str, all: bool) -> Result<Vec<Yaml>> {
        let mut docs = Vec::new();
        for document in serde_yaml_ng::Deserializer::from_str(text) {
            let mut value = Yaml::deserialize(document).map_err(failure)?;
            merge(&mut value)?;
            docs.push(value);
            if !all {
                break;
            }
        }
        Ok(docs)
    }

    fn to_json(value: &Yaml, path: &mut String) -> Result<Json> {
        Ok(match value {
            Yaml::Null => Json::Null,
            Yaml::Bool(b) => Json::Bool(*b),
            Yaml::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Json::from(i)
                } else if let Some(u) = n.as_u64() {
                    Json::from(u)
                } else {
                    let f = n.as_f64().unwrap_or(f64::NAN);
                    Json::Number(
                        serde_json::Number::from_f64(f)
                            .ok_or_else(|| CliError::Failed(format!("{n} di {} tak bisa jadi JSON", show(path))))?,
                    )
                }
            }
            Yaml::String(s) => Json::String(s.clone()),
            Yaml::Sequence(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    let len = push_index(path, i);
                    out.push(Self::to_json(item, path)?);
                    path.truncate(len);
                }
                Json::Array(out)
            }
            Yaml::Mapping(map) => {
                let mut out = serde_json::Map::with_capacity(map.len());
                for (key, item) in map {
                    let Some(name) = key_text(key) else {
                        return Err(CliError::Failed(format!(
                            "key di {} berupa urutan/mapping, JSON hanya menerima key string",
                            show(path)
                        )));
                    };
                    let len = push_key(path, &name);
                    if out.contains_key(name.as_ref()) {
                        return Err(CliError::Failed(format!(
                            "key ganda setelah dikonversi ke string JSON: {}",
                            show(path)
                        )));
                    }
                    out.insert(name.into_owned(), Self::to_json(item, path)?);
                    path.truncate(len);
                }
                Json::Object(out)
            }
            Yaml::Tagged(tagged) => Self::to_json(&tagged.value, path)?,
        })
    }

    fn from_json(value: Json, path: &mut String) -> Result<Yaml> {
        Ok(match value {
            Json::Null => Yaml::Null,
            Json::Bool(b) => Yaml::Bool(b),
            Json::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Yaml::Number(i.into())
                } else if let Some(u) = n.as_u64() {
                    Yaml::Number(u.into())
                } else if !n.as_str().contains(['.', 'e', 'E']) {
                    return Err(out_of_range(path, "bilangan bulat", n.as_str()));
                } else {
                    match n.as_f64() {
                        Some(f) if f.is_finite() => Yaml::Number(f.into()),
                        _ => return Err(out_of_range(path, "angka", n.as_str())),
                    }
                }
            }
            Json::String(s) => Yaml::String(s),
            Json::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for (i, item) in items.into_iter().enumerate() {
                    let len = push_index(path, i);
                    out.push(Self::from_json(item, path)?);
                    path.truncate(len);
                }
                Yaml::Sequence(out)
            }
            Json::Object(map) => {
                let mut out = serde_yaml_ng::Mapping::with_capacity(map.len());
                for (key, item) in map {
                    let len = push_key(path, &key);
                    out.insert(Yaml::String(key), Self::from_json(item, path)?);
                    path.truncate(len);
                }
                Yaml::Mapping(out)
            }
        })
    }

    fn get<'a>(value: &'a Yaml, segments: &[Segment]) -> Option<&'a Yaml> {
        let mut value = value;
        for segment in segments {
            value = match (segment, untag(value)) {
                (Segment::Key(key), Yaml::Mapping(map)) => map.get(key.as_str()).or_else(|| {
                    // `1: a` has a number key; `q 1` should still find it.
                    map.iter()
                        .find(|(k, _)| !k.is_string() && key_text(k).is_some_and(|t| t == key.as_str()))
                        .map(|(_, v)| v)
                })?,
                (Segment::Index(index), Yaml::Sequence(items)) => items.get(usize::try_from(*index).ok()?)?,
                _ => return None,
            };
        }
        Some(value)
    }

    fn sort_keys(value: &mut Yaml) {
        match value {
            Yaml::Sequence(items) => items.iter_mut().for_each(Self::sort_keys),
            Yaml::Tagged(tagged) => Self::sort_keys(&mut tagged.value),
            Yaml::Mapping(map) => {
                let mut entries: Vec<(Yaml, Yaml)> = std::mem::take(map).into_iter().collect();
                entries.iter_mut().for_each(|(_, v)| Self::sort_keys(v));
                entries.sort_by(|(a, _), (b, _)| key_text(a).cmp(&key_text(b)));
                map.extend(entries);
            }
            _ => {}
        }
    }

    fn as_str(value: &Yaml) -> Option<&str> {
        value.as_str()
    }

    fn render(value: &Yaml) -> Result<String> {
        serde_yaml_ng::to_string(value).map_err(|e| CliError::Failed(e.to_string()))
    }
}
