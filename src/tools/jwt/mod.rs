mod token;
use crate::errors::{CliError, Result, usage};
use crate::io::{choice, parse, read_input};
use crate::tool::Tool;
use clap::Parser;
use serde_json::{Value, json};
use std::{ffi::OsString, path::PathBuf};
pub static TOOL: Tool = Tool {
    name: "jwt",
    aliases: &[],
    description: "Decode, verifikasi, dan buat JWT HMAC (HS256/384/512)",
    category: "Keamanan & Kripto",
    run,
};
const HELP: &str = r#"dt jwt [options] [token]
dt jwt --sign -k <secret> [options] [payload-json]

Decode JWT: tampilkan header dan payload sebagai JSON, serta waktu exp/iat/nbf
(ISO-8601 UTC beserta statusnya). Token dari argumen, --file, atau stdin;
spasi di tepi dan awalan "Bearer " dibuang.
  -k, --key <secret>      Verifikasi signature HS256/HS384/HS512 (HMAC); dengan --sign: kunci penanda tangan
      --now <waktu>       ISO-8601 (T/spasi; tanpa zona = UTC) atau epoch detik
  -f, --file <path>       Baca token (atau payload dengan --sign) dari file
      --json              Keluaran satu dokumen JSON
  -h, --help              Tampilkan bantuan

Tanpa --key, signature TIDAK diverifikasi: isi token tidak boleh dipercaya.
Dengan --key, semuanya dicetak lebih dulu, lalu exit 1 jika signature tidak
valid, alg bukan HS* (termasuk "none"), token kedaluwarsa, belum berlaku,
atau exp/nbf bukan angka. Tanpa --key, exp/nbf bukan angka memberi peringatan
di stderr; token kedaluwarsa tetap exit 0. Token rusak: exit 1.

Mode tanda tangan (-s/--sign): payload (objek JSON) dari argumen, --file, atau
stdin; tercetak hanya token kompak. --key wajib.
  -s, --sign              Buat token bertanda tangan HMAC
      --alg <alg>         HS256 (default), HS384, atau HS512
      --exp <durasi|epoch> Set exp: 30s, 15m, 1h, 7d, 2w dari sekarang, atau epoch detik
                          (menimpa exp di payload)
      --iat               Set iat = sekarang (menimpa iat di payload)
Dengan --json: decode => {header, payload, times, signature, valid}; sign => {token, header, payload}.
Contoh: dt jwt eyJhbGciOi...   |   dt jwt -k rahasia < token.txt
        dt jwt -s -k rahasia --exp 1h --iat '{"sub":"42"}'
"#;
#[derive(Parser)]
#[command(name = "dt jwt", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'k', long)]
    key: Option<String>,
    #[arg(long)]
    now: Option<String>,
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,
    #[arg(short = 's', long)]
    sign: bool,
    #[arg(long)]
    alg: Option<String>,
    #[arg(long)]
    exp: Option<String>,
    #[arg(long)]
    iat: bool,
    #[arg(long)]
    json: bool,
    text: Vec<String>,
}
fn run(args: Vec<OsString>) -> Result<()> {
    std::thread::Builder::new()
        .name("dt-jwt".into())
        .stack_size(256 * 1024 * 1024)
        .spawn(move || body(args))?
        .join()
        .map_err(|_| CliError::Failed("Pemrosesan JWT gagal.".into()))?
}
fn print(value: &Value) -> Result<()> {
    println!("{}", token::stringify(value, 2)?);
    Ok(())
}
fn body(args: Vec<OsString>) -> Result<()> {
    let args: Args = parse("jwt", args)?;
    if args.key.as_deref() == Some("") {
        return Err(usage("Nilai --key tidak boleh kosong."));
    }
    if !args.sign {
        for (option, present) in [
            ("alg", args.alg.is_some()),
            ("exp", args.exp.is_some()),
            ("iat", args.iat),
        ] {
            if present {
                return Err(usage(format!("--{option} hanya berlaku dengan --sign.")));
            }
        }
    }
    let now = if let Some(value) = args.now.as_deref() {
        token::now(value)?
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| CliError::Failed(e.to_string()))?
            .as_secs_f64()
    };
    if args.sign {
        return sign(&args, now);
    }
    let bytes = read_input(&args.text, args.file.as_deref(), "Token JWT:")?;
    let text = String::from_utf8_lossy(&bytes);
    let decoded = token::decode(token::normalize(&text))?;
    let claims: Vec<_> = ["exp", "iat", "nbf"]
        .iter()
        .filter_map(|name| decoded.payload.get(*name).and_then(|v| token::claim(name, v, now)))
        .collect();
    let mistyped: Vec<_> = ["exp", "nbf"]
        .into_iter()
        .filter(|name| decoded.payload.get(*name).is_some_and(|v| !v.is_number()))
        .collect();
    let claim_error = (!mistyped.is_empty()).then(|| format!("Klaim {} harus berupa angka.", mistyped.join(", ")));
    if !args.json {
        println!("Header:\n{}\n", token::stringify(&decoded.header, 2)?);
        println!("Payload:\n{}", token::stringify(&decoded.payload, 2)?);
        if !claims.is_empty() {
            println!("\nWaktu:");
            for claim in &claims {
                println!("  {}", claim.line());
            }
        }
        println!();
    }
    let times: Vec<_> = claims
        .iter()
        .map(|c| json!({"claim":c.name,"iso":c.iso,"status":c.status}))
        .collect();
    let Some(key) = args.key.as_deref() else {
        if let Some(error) = claim_error {
            eprintln!("Peringatan: {error}");
        }
        if args.json {
            print(
                &json!({"header":decoded.header,"payload":decoded.payload,"times":times,"signature":{"verified":null},"valid":null}),
            )?;
        } else {
            println!("Signature: TIDAK diverifikasi (tanpa --key)");
        }
        return Ok(());
    };
    let alg = decoded.header.get("alg").and_then(Value::as_str);
    if !matches!(alg, Some("HS256" | "HS384" | "HS512")) {
        if args.json {
            print(
                &json!({"header":decoded.header,"payload":decoded.payload,"times":times,"signature":{"verified":false},"valid":false}),
            )?;
        }
        return Err(CliError::Failed(format!(
            "Algoritma tidak didukung untuk --key: {}. Hanya HS256, HS384, HS512.",
            crate::tools::json::document::raw_string(alg.unwrap_or("(tidak ada)"))
        )));
    }
    let alg = alg.unwrap_or("HS256");
    let verified = token::verify(&decoded, alg, key)?;
    let expired = claims.iter().any(|c| c.name == "exp" && c.invalid);
    let early = claims.iter().any(|c| c.name == "nbf" && c.invalid);
    if args.json {
        print(
            &json!({"header":decoded.header,"payload":decoded.payload,"times":times,"signature":{"verified":verified,"algorithm":alg},"valid":verified && !expired && !early && claim_error.is_none()}),
        )?;
    } else if verified {
        println!("Signature: valid");
    }
    if !verified {
        return Err(CliError::Failed("Signature tidak valid".into()));
    }
    if expired {
        return Err(CliError::Failed("Token sudah kedaluwarsa.".into()));
    }
    if early {
        return Err(CliError::Failed("Token belum berlaku.".into()));
    }
    if let Some(error) = claim_error {
        return Err(CliError::Failed(error));
    }
    Ok(())
}
fn sign(args: &Args, now: f64) -> Result<()> {
    let key = args
        .key
        .as_deref()
        .ok_or_else(|| usage("--sign memerlukan --key <secret>."))?;
    let alg = choice(
        args.alg.as_deref(),
        &[("HS256", "HS256"), ("HS384", "HS384"), ("HS512", "HS512")],
        "HS256",
        "alg",
    )?;
    let exp = args.exp.as_deref().map(|v| token::expiry(v, now)).transpose()?;
    let bytes = read_input(&args.text, args.file.as_deref(), "Payload JSON:")?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_matches(token::space);
    let mut payload = token::parse_value(text).map_err(|_| {
        let message = crate::tools::json::document::legacy_error(text);
        usage(format!("Payload bukan JSON yang valid: {message}"))
    })?;
    let object = payload
        .as_object_mut()
        .ok_or_else(|| usage("Payload harus berupa objek JSON."))?;
    if args.iat {
        object.insert("iat".into(), json!(now.floor()));
    }
    if let Some(exp) = exp {
        object.insert("exp".into(), json!(exp));
    }
    let header = json!({"alg":alg,"typ":"JWT"});
    let token = token::sign(&header, &payload, alg, key)?;
    if args.json {
        print(&json!({"token":token,"header":header,"payload":payload}))?;
    } else {
        println!("{token}");
    }
    Ok(())
}
