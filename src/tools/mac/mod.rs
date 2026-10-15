mod address;

use crate::errors::{Result, usage};
use crate::io;
use crate::tool::Tool;
use address::{FORMAT_CHOICES, format_mac, mac_flags, parse_mac};
use clap::Parser;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "mac",
    aliases: &[],
    description: "Parse dan konversi alamat MAC (colon, hyphen, dot, plain), flag, dan alamat acak",
    category: "Jaringan",
    run,
};

const HELP: &str = r#"dt mac [options] [address]

Parse dan konversi alamat MAC (EUI-48). Tanpa address, dibaca dari stdin.
Bentuk input: aa:bb:cc:dd:ee:ff, aa-bb-cc-dd-ee-ff, aabb.ccdd.eeff (Cisco), aabbccddeeff.
  -f, --format <jenis>  Cetak satu bentuk saja: colon, hyphen, dot, plain
      --upper           Huruf kapital
      --random          Buat alamat acak: unicast, locally administered (tanpa address)
      --json            Keluaran JSON
  -h, --help            Tampilkan bantuan

Tanpa --format dicetak semua bentuk beserta flag unicast/multicast (bit I/G) dan
universally/locally administered (bit U/L). Alamat tidak valid: exit 2.
Contoh: dt mac AA-BB-CC-DD-EE-FF   |   dt mac -f dot 00:1a:2b:3c:4d:5e   |   dt mac --random
"#;

#[derive(Parser)]
#[command(name = "dt mac", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'f', long)]
    format: Option<String>,
    #[arg(long)]
    upper: bool,
    #[arg(long)]
    random: bool,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = io::parse("mac", args)?;
    let format = match a.format.as_deref() {
        None => None,
        v => Some(io::choice(v, &FORMAT_CHOICES, FORMAT_CHOICES[0].1, "format")?),
    };
    let upper = a.upper;

    let hex = if a.random {
        if !a.positionals.is_empty() {
            return Err(usage("--random tidak menerima address."));
        }
        let mut bytes = crate::random::bytes(6)?;
        // Clear the multicast bit, set the locally administered bit.
        bytes[0] = (bytes[0] & 0xfe) | 0x02;
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    } else {
        if a.positionals.len() > 1 {
            return Err(usage(format!("Argumen berlebih: {}", a.positionals[1])));
        }
        let raw = match a.positionals.first() {
            Some(p) => p.clone(),
            None => String::from_utf8_lossy(&io::read_input(&[], None, "Alamat MAC")?).into_owned(),
        };
        parse_mac(&raw)?
    };

    let flags = mac_flags(&hex);
    let default_format = FORMAT_CHOICES[0].1;
    if a.json {
        let mut formats = serde_json::Map::new();
        for (name, f) in FORMAT_CHOICES {
            formats.insert(name.to_string(), format_mac(&hex, f, upper).into());
        }
        io::print_json(&serde_json::json!({
            "address": format_mac(&hex, format.unwrap_or(default_format), upper),
            "formats": formats,
            "multicast": flags.multicast,
            "locallyAdministered": flags.locally_administered,
        }));
        return Ok(());
    }
    if let Some(f) = format {
        println!("{}", format_mac(&hex, f, upper));
        return Ok(());
    }
    let mut lines: Vec<String> = FORMAT_CHOICES
        .iter()
        .map(|(n, f)| format!("{n:<7} {}", format_mac(&hex, *f, upper)))
        .collect();
    lines.push(format!(
        "{:<7} {}",
        "tipe",
        if flags.multicast { "multicast" } else { "unicast" }
    ));
    lines.push(format!(
        "{:<7} {}",
        "admin",
        if flags.locally_administered {
            "locally administered"
        } else {
            "universally administered"
        }
    ));
    println!("{}", lines.join("\n"));
    Ok(())
}
