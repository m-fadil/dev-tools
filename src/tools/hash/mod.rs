mod crc32;
pub mod digest;

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use crate::codec::{Codec, encode};
use crate::errors::{Result, usage};
use crate::io::{choice, parse, print_json, read_input};
use crate::tool::Tool;

pub static TOOL: Tool = Tool {
    name: "hash",
    aliases: &[],
    description: "Hash atau HMAC teks/file (md5, sha1, sha2, sha3, blake2b, crc32)",
    category: "Keamanan & Kripto",
    run,
};

const HELP: &str = r##"dt hash [options] [teks]

Hash teks, file, atau stdin. Tanpa teks/--file/stdin: prompt (terminal).
  -a, --algorithm <nama>  md5, sha1, sha224, sha256, sha384, sha512, sha3-256, sha3-512, blake2b512, crc32 (default: sha256)
  -e, --encoding <jenis>  hex, base64, base64url (default: hex)
  -k, --key <kunci>       Hitung HMAC dengan kunci ini, bukan hash biasa (tidak untuk crc32)
  -f, --file <path>       Hash isi file
      --json              Keluaran JSON: {algorithm, encoding, hmac, digest}
  -h, --help              Tampilkan bantuan

crc32 adalah checksum 32-bit (hex 8 digit), bukan hash kriptografis.
Input stdin dipakai apa adanya, termasuk newline akhir (seperti sha256sum).
Contoh: dt hash -a sha512 "halo"   |   dt hash -f dist/src/cli.js
"##;

const ENCODINGS: &[(&str, Codec)] = &[
    ("hex", Codec::Hex),
    ("base64", Codec::Base64),
    ("base64url", Codec::Base64Url),
];

#[derive(Parser)]
#[command(name = "dt hash", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'a', long, overrides_with = "algorithm")]
    algorithm: Option<String>,
    #[arg(short = 'e', long, overrides_with = "encoding")]
    encoding: Option<String>,
    #[arg(short = 'k', long, overrides_with = "key")]
    key: Option<String>,
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    #[arg(long, overrides_with = "json")]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("hash", args)?;
    let mut algorithms: Vec<(&str, &str)> = digest::HASH_ALGORITHMS.iter().map(|n| (*n, *n)).collect();
    algorithms.push(("crc32", "crc32"));
    let algorithm = choice(a.algorithm.as_deref(), &algorithms, "sha256", "algorithm")?;
    let encoding = choice(a.encoding.as_deref(), ENCODINGS, Codec::Hex, "encoding")?;
    if algorithm == "crc32" && a.key.is_some() {
        return Err(usage(
            "crc32 bukan hash berkunci; --key tidak bisa dipakai dengan crc32.",
        ));
    }
    let data = read_input(&a.positionals, a.file.as_deref(), "Teks yang akan di-hash")?;
    let digest_bytes = if algorithm == "crc32" {
        crc32::crc32_bytes(&data).to_vec()
    } else {
        digest::hash(&data, algorithm, a.key.as_deref())?
    };
    let digest = encode(&digest_bytes, encoding);
    if a.json {
        let enc = ENCODINGS.iter().find(|(_, c)| *c == encoding).map_or("hex", |(n, _)| n);
        print_json(&serde_json::json!({
            "algorithm": algorithm,
            "encoding": enc,
            "hmac": a.key.is_some(),
            "digest": digest,
        }));
        return Ok(());
    }
    let _ = writeln!(std::io::stdout(), "{digest}");
    Ok(())
}
