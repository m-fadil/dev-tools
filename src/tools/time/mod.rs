pub(crate) mod instant;
mod zones;
use crate::errors::{CliError, Result, usage};
use crate::io::{choice, parse, print_json};
use crate::tool::Tool;
use clap::Parser;
use instant::{Zone, format_in_zone, parse_instant, relative, utc_iso};
use serde_json::json;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "time",
    aliases: &["ts"],
    description: "Konversi epoch/ISO-8601 ke UTC, zona waktu, dan waktu relatif",
    category: "Utilitas",
    run,
};
const HELP: &str = "dt time [options] [nilai]

Tampilkan satu waktu dalam UTC, zona pilihan, epoch, dan waktu relatif.
Tanpa nilai: waktu sekarang.
  -z, --timezone <IANA>   Zona waktu, mis. Asia/Jakarta (default: zona sistem)
  -u, --unit <s|ms>       Satuan epoch, juga untuk --now (default: deteksi, < 1e11 detik, selain itu milidetik).
                          Ditolak untuk nilai ISO atau bila tidak ada epoch
      --now <waktu>       Ganti jam sekarang untuk hitungan relatif (ISO atau epoch)
      --json              Keluaran satu dokumen JSON (input, utc, zone, epochSeconds, epochMillis, relative)
  -h, --help              Tampilkan bantuan

Nilai: epoch bulat (boleh negatif) atau ISO-8601: 2026-01-02, 2026-01-02T10:04:05,
2026-01-02T10:04:05.123+07:00, 2026-01-02T03:04:05Z. Tanpa offset, waktu dibaca
di zona --timezone (tanggal saja = tengah malam zona itu). Epoch negatif perlu
\"--\" di depannya: dt time -- -86400
Rentang: ±8,64e15 ms. Input tidak valid keluar dengan kode 2.
Contoh: dt time 1767323045   |   dt time -z Asia/Jakarta 2026-01-02T10:04:05
";
#[derive(Parser)]
#[command(name="dt time", override_help=HELP, disable_version_flag=true)]
struct Args {
    #[arg(short = 'z', long)]
    timezone: Option<String>,
    #[arg(short = 'u', long)]
    unit: Option<String>,
    #[arg(long)]
    now: Option<String>,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}
fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("time", args)?;
    if a.positionals.len() > 1 {
        return Err(usage(format!("Terlalu banyak argumen: {}", a.positionals[1])));
    }
    let zone = Zone::resolve(a.timezone.as_deref())?;
    let unit = a
        .unit
        .as_deref()
        .map(|v| choice(Some(v), &[("s", "s"), ("ms", "ms")], "s", "unit"))
        .transpose()?;
    if unit.is_some() && a.positionals.is_empty() && a.now.is_none() {
        return Err(usage("--unit hanya berlaku untuk nilai epoch"));
    }
    let now = match a.now.as_deref() {
        Some(v) => parse_instant(v, &zone, unit)?,
        None => instant::check_range(chrono::Utc::now().timestamp_millis() as f64, "Waktu sekarang")?,
    };
    let ms = match a.positionals.first() {
        Some(v) => parse_instant(v, &zone, unit)?,
        None => now,
    };
    if zone.invalid_default {
        return Err(CliError::Failed("Invalid time zone specified: Etc/Unknown".into()));
    }
    let utc = utc_iso(ms);
    let iso = format_in_zone(ms, &zone);
    let rel = relative(ms, now);
    if a.json {
        let zone_json = if zone.missing_name {
            json!({"iso":iso})
        } else {
            json!({"name":zone.name,"iso":iso})
        };
        print_json(
            &json!({"input": a.positionals.first(), "utc": utc, "zone": zone_json, "epochSeconds": ms.div_euclid(1000), "epochMillis": ms, "relative": rel}),
        );
    } else {
        if zone.missing_name {
            return Err(CliError::Failed(
                "Cannot read properties of undefined (reading 'length')".into(),
            ));
        }
        let rows = [
            ("UTC", utc),
            (zone.name.as_str(), iso),
            ("Epoch (s)", ms.div_euclid(1000).to_string()),
            ("Epoch (ms)", ms.to_string()),
            ("Relatif", rel),
        ];
        let width = rows.iter().map(|(s, _)| s.encode_utf16().count()).max().unwrap_or(0) + 2;
        for (label, value) in rows {
            println!(
                "{label}:{}{value}",
                " ".repeat(width.saturating_sub(label.encode_utf16().count() + 1))
            );
        }
    }
    Ok(())
}
