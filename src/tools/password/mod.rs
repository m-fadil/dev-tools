mod generate;

pub use generate::CryptoInts;

use crate::errors::{Result, usage};
use crate::io::{choice, integer, parse, print_json};
use crate::numfmt::js_number;
use crate::tool::Tool;
use clap::Parser;
use generate::{ONLY_SETS, PasswordOptions, character_classes, entropy_bits, generate_password};
use std::ffi::OsString;
use std::io::Write;

pub static TOOL: Tool = Tool {
    name: "password",
    aliases: &["pw"],
    description: "Buat password acak yang aman secara kriptografis",
    category: "Keamanan & Kripto",
    run,
};

const HELP: &str = "dt password [options]

Buat password acak dari CSPRNG (node:crypto). Default: huruf kecil, huruf besar,
dan angka; tiap jenis karakter aktif dijamin muncul minimal satu kali.
  -l, --length <n>        Panjang password, 4-1024 (default: 20)
  -c, --count <n>         Jumlah password, 1-1000 (default: 1), satu per baris
  -s, --symbols           Tambahkan simbol !@#$%^&*()-_=+[]{};:,.?
      --only <set>        Batasi karakter: lower, upper, digits, alnum, hex
  -u, --unambiguous       Buang karakter yang mirip: 0 O 1 l I |
  -e, --entropy           Tampilkan estimasi entropi ke stderr
      --json              Keluaran JSON: {values, entropyBits?} (entropi masuk ke JSON, bukan stderr)
  -h, --help              Tampilkan bantuan

--only tidak bisa dipakai bersama --symbols. alnum = huruf besar/kecil + angka;
hex = 0-9a-f (tanpa jaminan jenis karakter). --length harus minimal sebanyak
jenis karakter aktif. Entropi (panjang * log2 ukuran alfabet) adalah batas
atas: jaminan satu karakter per jenis menguranginya sedikit.
Contoh: dt password -s -l 32   |   dt password -c 5 --only alnum -u -e
";

#[derive(Parser)]
#[command(name = "dt password", override_help = HELP, disable_help_flag = true, disable_version_flag = true)]
struct Args {
    #[arg(short = 'l', long, overrides_with = "length")]
    length: Option<String>,
    #[arg(short = 'c', long, overrides_with = "count")]
    count: Option<String>,
    #[arg(short = 's', long, overrides_with = "symbols")]
    symbols: bool,
    #[arg(long, overrides_with = "only")]
    only: Option<String>,
    #[arg(short = 'u', long, overrides_with = "unambiguous")]
    unambiguous: bool,
    #[arg(short = 'e', long, overrides_with = "entropy")]
    entropy: bool,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    positionals: Vec<String>,
}

/// A JSON number printed exactly like JavaScript's `JSON.stringify` would.
fn js_json_number(v: f64) -> serde_json::Value {
    serde_json::from_str(&js_number(v)).unwrap_or(serde_json::Value::Null)
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("password", args)?;
    if a.help {
        let _ = write!(std::io::stdout(), "{HELP}");
        return Ok(());
    }
    if let Some(p) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {p}")));
    }
    let length = integer(a.length.as_deref(), 20, 4, 1024, "length")? as usize;
    let count = integer(a.count.as_deref(), 1, 1, 1000, "count")? as usize;
    let only = match a.only.as_deref() {
        None => None,
        v => Some(choice(v, ONLY_SETS, generate::OnlySet::Alnum, "only")?),
    };
    let classes = character_classes(&PasswordOptions {
        symbols: a.symbols,
        only,
        unambiguous: a.unambiguous,
    })?;
    let mut rng = CryptoInts::new();
    let mut next = |max: usize| rng.below(max);
    let mut passwords = Vec::with_capacity(count);
    for _ in 0..count {
        passwords.push(generate_password(&classes, length, &mut next)?);
    }
    if a.json {
        let mut doc = serde_json::Map::new();
        doc.insert("values".into(), serde_json::json!(passwords));
        if a.entropy {
            doc.insert("entropyBits".into(), js_json_number(entropy_bits(&classes, length)));
        }
        print_json(&serde_json::Value::Object(doc));
        return Ok(());
    }
    let _ = writeln!(std::io::stdout(), "{}", passwords.join("\n"));
    if a.entropy {
        eprintln!("Entropi: ~{} bit", (entropy_bits(&classes, length) + 0.5).floor());
    }
    Ok(())
}
