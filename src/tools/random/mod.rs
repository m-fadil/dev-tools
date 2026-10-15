mod nanoid;
mod token;
mod ulid;
mod version;

use crate::errors::{Result, usage};
use crate::io::{choice, integer, parse, print_json};
use crate::tool::Tool;
use crate::tools::password::CryptoInts;
use clap::Parser;
use nanoid::{DEFAULT_ALPHABET, nanoid, validate_alphabet};
use std::ffi::OsString;
use std::io::Write;
use token::{TOKEN_FORMATS, TokenFormat, token_by_bytes, token_by_length};
use ulid::UlidGenerator;
use version::{UUID_VERSIONS, V1Generator, V7Generator, now_ms};

pub static TOOL: Tool = Tool {
    name: "random",
    aliases: &[],
    description: "Buat UUID, token, ULID, atau NanoID acak melalui subcommand uuid/token/ulid/nanoid",
    category: "Utilitas",
    run,
};

const HELP: &str = "dt random <uuid|token|ulid|nanoid> [options]

  uuid    Buat identifier UUID v1/v4/v7 dengan format baku
  token   Buat token acak dengan ukuran dan encoding pilihan
  ulid    Buat ULID 26 karakter yang terurut waktu
  nanoid  Buat ID pendek acak dengan panjang dan alphabet pilihan

Gunakan dt random <subcommand> -h untuk opsi masing-masing.
Tanpa subcommand: tampilkan bantuan. Tidak menerima teks, file, atau stdin.
Exit: 0 sukses, 1 gagal, 2 penggunaan salah.
";

const UUID_HELP: &str = "dt random uuid [options]

Buat UUID; panjang selalu 36 karakter, tidak bisa diubah.
  -v, --version <n>  1, 4, 7 (default: 4)
  -c, --count <n>    Jumlah UUID, 1-1000 (default: 1), satu per baris
      --json         Keluaran JSON: {type: \"uuid\", version, values}
  -h, --help         Tampilkan bantuan

v4: 122 bit acak. v7: timestamp ms + acak, urut naik ketat dalam satu eksekusi.
v1: timestamp 100ns + node acak (bukan MAC asli), unik dalam satu eksekusi.
UUID adalah identifier, bukan token rahasia. Untuk token: dt random token.
Tidak menerima argumen teks atau opsi ukuran/format.
Contoh: dt random uuid -v 7 -c 3
";

const TOKEN_HELP: &str = "dt random token [options]

Buat token acak dari CSPRNG (node:crypto).
  -l, --length <n>     Panjang dalam karakter, 1-4096
  -b, --bytes <n>      Panjang dalam byte, 1-1024
      --bits <n>       Panjang dalam bit, kelipatan 8, 8-8192
  -f, --format <jenis> hex, base64, base64url, alnum (default: hex)
  -c, --count <n>      Jumlah token, 1-1000 (default: 1), satu per baris
      --json           Keluaran JSON: {type: \"token\", format, values}
  -h, --help           Tampilkan bantuan

Pilih salah satu: --length, --bytes, --bits. Default: 32 byte, atau 32 karakter
untuk alnum (A-Za-z0-9). alnum hanya mendukung --length.
--length menghasilkan jumlah karakter persis, tanpa padding \"=\".
Ukuran byte/bit menentukan data acak sebelum encoding, bukan jumlah karakter.
Tidak menerima argumen teks atau --version; tidak perlu flag --token.
Contoh: dt random token -l 24 -f alnum
        dt random token --bits 256 -f base64url
";

const ULID_HELP: &str = "dt random ulid [options]

Buat ULID: 26 karakter Crockford base32 (timestamp 48 bit ms + 80 bit acak).
  -c, --count <n>  Jumlah ULID, 1-1000 (default: 1), satu per baris
      --lowercase  Huruf kecil (default: huruf besar)
      --json       Keluaran JSON: {type: \"ulid\", values}
  -h, --help       Tampilkan bantuan

Urut naik ketat dalam satu eksekusi, juga dalam milidetik yang sama.
Tidak menerima argumen teks.
Contoh: dt random ulid -c 3
";

const NANOID_HELP: &str = "dt random nanoid [options]

Buat NanoID dari CSPRNG, tanpa bias modulo.
  -l, --length <n>      Panjang, 1-256 (default: 21)
      --alphabet <str>  2-94 karakter ASCII unik tanpa spasi
                        (default: A-Za-z0-9_-, 64 karakter)
  -c, --count <n>       Jumlah ID, 1-1000 (default: 1), satu per baris
      --json            Keluaran JSON: {type: \"nanoid\", values}
  -h, --help            Tampilkan bantuan

Tidak menerima argumen teks.
Contoh: dt random nanoid -l 12 --alphabet 0123456789abcdef
";

#[derive(Parser)]
#[command(name = "dt random uuid", disable_help_flag = true, disable_version_flag = true)]
struct UuidArgs {
    #[arg(short = 'c', long, overrides_with = "count")]
    count: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    #[arg(short = 'v', long, overrides_with = "version")]
    version: Option<String>,
    positionals: Vec<String>,
}

#[derive(Parser)]
#[command(name = "dt random token", disable_help_flag = true, disable_version_flag = true)]
struct TokenArgs {
    #[arg(short = 'c', long, overrides_with = "count")]
    count: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    #[arg(short = 'l', long, overrides_with = "length")]
    length: Option<String>,
    #[arg(short = 'b', long, overrides_with = "bytes")]
    bytes: Option<String>,
    #[arg(long, overrides_with = "bits")]
    bits: Option<String>,
    #[arg(short = 'f', long, overrides_with = "format")]
    format: Option<String>,
    positionals: Vec<String>,
}

#[derive(Parser)]
#[command(name = "dt random ulid", disable_help_flag = true, disable_version_flag = true)]
struct UlidArgs {
    #[arg(short = 'c', long, overrides_with = "count")]
    count: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    #[arg(long, overrides_with = "lowercase")]
    lowercase: bool,
    positionals: Vec<String>,
}

#[derive(Parser)]
#[command(name = "dt random nanoid", disable_help_flag = true, disable_version_flag = true)]
struct NanoidArgs {
    #[arg(short = 'c', long, overrides_with = "count")]
    count: Option<String>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    #[arg(short = 'l', long, overrides_with = "length")]
    length: Option<String>,
    #[arg(long, overrides_with = "alphabet", allow_hyphen_values = true)]
    alphabet: Option<String>,
    positionals: Vec<String>,
}

fn print_lines(list: &[String]) {
    let _ = writeln!(std::io::stdout(), "{}", list.join("\n"));
}

fn run_uuid(args: Vec<OsString>) -> Result<()> {
    let a: UuidArgs = parse("random uuid", args)?;
    if a.help {
        let _ = write!(std::io::stdout(), "{UUID_HELP}");
        return Ok(());
    }
    if let Some(p) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {p}")));
    }
    let count = integer(a.count.as_deref(), 1, 1, 1000, "count")? as usize;
    let version = choice(a.version.as_deref(), UUID_VERSIONS, 4, "version")?;
    let mut list = Vec::with_capacity(count);
    match version {
        7 => {
            let mut g = V7Generator::new(now_ms);
            for _ in 0..count {
                list.push(g.next()?);
            }
        }
        1 => {
            let mut g = V1Generator::new(now_ms)?;
            for _ in 0..count {
                list.push(g.next()?);
            }
        }
        _ => {
            for _ in 0..count {
                list.push(version::v4()?);
            }
        }
    }
    if a.json {
        print_json(&serde_json::json!({ "type": "uuid", "version": version, "values": list }));
    } else {
        print_lines(&list);
    }
    Ok(())
}

fn run_token(args: Vec<OsString>) -> Result<()> {
    let a: TokenArgs = parse("random token", args)?;
    if a.help {
        let _ = write!(std::io::stdout(), "{TOKEN_HELP}");
        return Ok(());
    }
    if let Some(p) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {p}")));
    }
    let count = integer(a.count.as_deref(), 1, 1, 1000, "count")? as usize;
    let sizes = [&a.length, &a.bytes, &a.bits].iter().filter(|v| v.is_some()).count();
    if sizes > 1 {
        return Err(usage("Pilih salah satu: --length, --bytes, atau --bits."));
    }
    let format = choice(a.format.as_deref(), TOKEN_FORMATS, TokenFormat::Hex, "format")?;
    if format == TokenFormat::Alnum && (a.bytes.is_some() || a.bits.is_some()) {
        return Err(usage("Format alnum hanya mendukung --length."));
    }
    let mut list = Vec::with_capacity(count);
    if a.length.is_some() || format == TokenFormat::Alnum {
        let length = integer(a.length.as_deref(), 32, 1, 4096, "length")? as usize;
        let mut ints = CryptoInts::new();
        for _ in 0..count {
            list.push(token_by_length(format, length, &mut ints)?);
        }
    } else {
        let bits = if a.bits.is_some() {
            Some(integer(a.bits.as_deref(), 0, 8, 8192, "bits")?)
        } else {
            None
        };
        if let Some(b) = bits
            && b % 8 != 0
        {
            return Err(usage(format!("Nilai --bits harus kelipatan 8: {b}")));
        }
        let bytes = match bits {
            Some(b) => (b / 8) as usize,
            None => integer(a.bytes.as_deref(), 32, 1, 1024, "bytes")? as usize,
        };
        for _ in 0..count {
            list.push(token_by_bytes(format, bytes)?);
        }
    }
    if a.json {
        print_json(&serde_json::json!({ "type": "token", "format": format.name(), "values": list }));
    } else {
        print_lines(&list);
    }
    Ok(())
}

fn run_ulid(args: Vec<OsString>) -> Result<()> {
    let a: UlidArgs = parse("random ulid", args)?;
    if a.help {
        let _ = write!(std::io::stdout(), "{ULID_HELP}");
        return Ok(());
    }
    if let Some(p) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {p}")));
    }
    let count = integer(a.count.as_deref(), 1, 1, 1000, "count")? as usize;
    let mut g = UlidGenerator::new(now_ms);
    let mut list = Vec::with_capacity(count);
    for _ in 0..count {
        list.push(g.next(a.lowercase)?);
    }
    if a.json {
        print_json(&serde_json::json!({ "type": "ulid", "values": list }));
    } else {
        print_lines(&list);
    }
    Ok(())
}

fn run_nanoid(args: Vec<OsString>) -> Result<()> {
    let a: NanoidArgs = parse("random nanoid", args)?;
    if a.help {
        let _ = write!(std::io::stdout(), "{NANOID_HELP}");
        return Ok(());
    }
    if let Some(p) = a.positionals.first() {
        return Err(usage(format!("Argumen tidak dikenal: {p}")));
    }
    let count = integer(a.count.as_deref(), 1, 1, 1000, "count")? as usize;
    let length = integer(a.length.as_deref(), 21, 1, 256, "length")? as usize;
    let alphabet = validate_alphabet(a.alphabet.as_deref().unwrap_or(DEFAULT_ALPHABET))?;
    let mut list = Vec::with_capacity(count);
    for _ in 0..count {
        list.push(nanoid(alphabet, length)?);
    }
    if a.json {
        print_json(&serde_json::json!({ "type": "nanoid", "values": list }));
    } else {
        print_lines(&list);
    }
    Ok(())
}

fn run(args: Vec<OsString>) -> Result<()> {
    let mut it = args.into_iter();
    let Some(command) = it.next() else {
        let _ = write!(std::io::stdout(), "{HELP}");
        return Ok(());
    };
    let rest: Vec<OsString> = it.collect();
    match command.to_string_lossy().as_ref() {
        "--help" | "-h" => {
            let _ = write!(std::io::stdout(), "{HELP}");
            Ok(())
        }
        "uuid" => run_uuid(rest),
        "ulid" => run_ulid(rest),
        "nanoid" => run_nanoid(rest),
        "token" => run_token(rest),
        other => Err(usage(format!(
            "Subcommand random tidak dikenal: {other}. Pilihan: uuid, token, ulid, nanoid."
        ))),
    }
}
