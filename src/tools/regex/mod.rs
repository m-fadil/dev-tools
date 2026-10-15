mod match_;

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use serde_json::{Map, Value, json};

use crate::errors::{CliError, Result, usage};
use crate::io::{parse, print_json, read_input};
use crate::tool::Tool;
use match_::{MatchJob, MatchRecord, build, normalize_flags, run_match};

pub static TOOL: Tool = Tool {
    name: "regex",
    aliases: &["re"],
    description: "Uji regex JavaScript: daftar kecocokan, grup, dan penggantian",
    category: "Data",
    run,
};

const HELP: &str = r##"dt regex [options] <pattern> [teks...]

Cari semua kecocokan regex (sintaks crate Rust regex) pada teks. Teks dari argumen
setelah pattern, --file, atau stdin (dipakai apa adanya, newline akhir ikut).
  -f, --flags <flags>     Flag regex: i m s x (pencocokan global selalu aktif)
  -r, --replace <templ>   Cetak juga hasil penggantian ($1, ${nama}, $0, $$)
      --first             Berhenti di kecocokan pertama (juga membatasi --replace)
      --test              Tanpa keluaran; exit 0 bila cocok, 1 bila tidak
      --json              Keluaran JSON tunggal
      --file <path>       Baca teks dari berkas
  -h, --help              Tampilkan bantuan

Setiap kecocokan: #n index=<posisi byte> "<teks>", lalu grup bernomor ($1..) dan
bernama; grup yang tidak ikut cocok ditampilkan -. Pattern yang diawali "-"
perlu didahului "--". Lookaround dan backreference tidak didukung.
Exit: 0 sukses, 1 (--test) tidak cocok, 2 pattern/penggunaan salah.
Contoh: dt regex '(\d+)-(?<b>\w+)' "12-ab 7-z"   |   echo 'a.b' | dt regex -r '$0!' '\.'
"##;

#[derive(Parser)]
#[command(name = "dt regex", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'f', long, overrides_with = "flags")]
    flags: Option<String>,
    #[arg(short = 'r', long, overrides_with = "replace")]
    replace: Option<String>,
    #[arg(long, overrides_with = "first")]
    first: bool,
    #[arg(long, overrides_with = "test")]
    test: bool,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(long, overrides_with = "file")]
    file: Option<PathBuf>,
    positionals: Vec<String>,
}

fn quote(value: &Option<String>) -> String {
    match value {
        None => "-".to_string(),
        Some(v) => serde_json::to_string(v).unwrap_or_default(),
    }
}

fn format_match(m: &MatchRecord, n: usize) -> String {
    let q = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let mut lines = vec![format!("#{n} index={} {}", m.index, q(&m.text))];
    for (i, g) in m.groups.iter().enumerate() {
        lines.push(format!("  ${} {}", i + 1, quote(g)));
    }
    for (name, g) in &m.named {
        lines.push(format!("  <{name}> {}", quote(g)));
    }
    lines.join("\n")
}

fn record_json(m: &MatchRecord) -> Value {
    let opt = |g: &Option<String>| g.as_ref().map_or(Value::Null, |s| Value::String(s.clone()));
    let mut named = Map::new();
    for (name, g) in &m.named {
        named.insert(name.clone(), opt(g));
    }
    json!({
        "index": m.index,
        "match": m.text,
        "groups": m.groups.iter().map(opt).collect::<Vec<_>>(),
        "named": named,
    })
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("regex", args)?;
    let Some((pattern, rest)) = a.positionals.split_first() else {
        return Err(usage("Pattern wajib diisi."));
    };
    if a.test && (a.json || a.replace.is_some()) {
        return Err(usage("--test tidak bisa digabung dengan --json atau --replace."));
    }
    let flags = normalize_flags(a.flags.as_deref().unwrap_or("")).map_err(usage)?;
    let re = build(pattern, &flags).map_err(|e| usage(format!("Regex tidak valid: {e}")))?;

    let data = read_input(rest, a.file.as_deref(), "Teks:")?;
    let text = String::from_utf8_lossy(&data);
    let job = MatchJob {
        text: &text,
        first: a.first,
        replace: a.replace.as_deref(),
    };
    let result = run_match(&re, &job);

    if a.test {
        return if result.matches.is_empty() {
            Err(CliError::Exit(1))
        } else {
            Ok(())
        };
    }
    if a.json {
        let mut doc = Map::new();
        doc.insert("pattern".into(), json!(pattern));
        doc.insert("flags".into(), json!(flags));
        doc.insert(
            "matches".into(),
            Value::Array(result.matches.iter().map(record_json).collect()),
        );
        if let Some(r) = &result.replaced {
            doc.insert("replaced".into(), json!(r));
        }
        print_json(&Value::Object(doc));
        return Ok(());
    }
    if result.matches.is_empty() {
        let _ = writeln!(std::io::stderr(), "Tidak ada kecocokan.");
    } else {
        let body: Vec<String> = result
            .matches
            .iter()
            .enumerate()
            .map(|(i, m)| format_match(m, i + 1))
            .collect();
        let _ = writeln!(std::io::stdout(), "{}", body.join("\n"));
    }
    if let Some(r) = &result.replaced {
        let _ = writeln!(std::io::stdout(), "Hasil ganti:\n{r}");
    }
    Ok(())
}
