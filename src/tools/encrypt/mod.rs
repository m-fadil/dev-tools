mod cipher;

use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::PathBuf;

use clap::Parser;

use crate::errors::{Result, usage};
use crate::io::{parse, read_input, write_output};
use crate::tool::Tool;
use cipher::{decrypt, encrypt};

pub static TOOL: Tool = Tool {
    name: "encrypt",
    aliases: &[],
    description: "Enkripsi/dekripsi teks atau file dengan password (AES-256-GCM)",
    category: "Keamanan & Kripto",
    run,
};

const HELP: &str = r##"dt encrypt [options] [teks]

Enkripsi AES-256-GCM (kunci dari password via scrypt); hasil base64url.
Tanpa teks/--file/stdin: prompt (terminal).
  -d, --decrypt           Dekripsi data hasil encrypt, bukan mengenkripsi
  -p, --password <teks>   Password (juga: env DEV_TOOL_PASSWORD; default: prompt)
  -f, --file <path>       Baca input dari file
  -h, --help              Tampilkan bantuan

--password terlihat di daftar proses dan history shell; di skrip, pakai
DEV_TOOL_PASSWORD. Password salah atau data diubah: exit 1.
Contoh: dt encrypt -p rahasia "halo" | dt encrypt -d -p rahasia
"##;

#[derive(Parser)]
#[command(name = "dt encrypt", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'd', long, overrides_with = "decrypt")]
    decrypt: bool,
    #[arg(short = 'p', long, overrides_with = "password")]
    password: Option<String>,
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    positionals: Vec<String>,
}

fn resolve_password(option: Option<String>) -> Result<String> {
    if option.as_deref() == Some("") {
        return Err(usage("Password tidak boleh kosong."));
    }
    let value = option.or_else(|| std::env::var("DEV_TOOL_PASSWORD").ok());
    if let Some(v) = value.filter(|v| !v.is_empty()) {
        return Ok(v);
    }
    if !std::io::stdin().is_terminal() {
        return Err(usage(
            "Password diperlukan: gunakan --password atau env DEV_TOOL_PASSWORD.",
        ));
    }
    let prompted = inquire::Password::new("Password")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()?;
    if prompted.is_empty() {
        return Err(usage("Password tidak boleh kosong."));
    }
    Ok(prompted)
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("encrypt", args)?;
    // Resolve the password first so a stdin-backed prompt is not starved by piped input.
    let password = resolve_password(a.password)?;
    let prompt = if a.decrypt {
        "Data terenkripsi"
    } else {
        "Teks yang akan dienkripsi"
    };
    let data = read_input(&a.positionals, a.file.as_deref(), prompt)?;
    if a.decrypt {
        write_output(&decrypt(&String::from_utf8_lossy(&data), &password)?);
    } else {
        write_output(format!("{}\n", encrypt(&data, &password)?).as_bytes());
    }
    Ok(())
}
