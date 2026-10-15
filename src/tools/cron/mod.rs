mod schedule;
use crate::errors::{CliError, Result, usage};
use crate::io::{integer, parse, print_json};
use crate::tool::Tool;
use crate::tools::time::instant::Zone;
use clap::Parser;
use schedule::{explain, format_time, next_fires, parse_cron, parse_from};
use serde_json::json;
use std::ffi::OsString;

pub static TOOL: Tool = Tool {
    name: "cron",
    aliases: &[],
    description: "Jelaskan ekspresi cron dan tampilkan waktu eksekusi berikutnya",
    category: "Utilitas",
    run,
};
const HELP: &str = "dt cron [options] <ekspresi>

Jelaskan ekspresi cron 5 field (menit jam tanggal bulan hari) dan tampilkan
waktu eksekusi berikutnya. Ekspresi diberikan sebagai satu argumen dikutip,
atau 5 argumen terpisah.
  -n, --next <n>      Jumlah waktu berikutnya, 1-50 (default: 5)
      --from <waktu>  Mulai dihitung setelah waktu ISO-8601 ini (default: sekarang)
      --utc           Pakai UTC, bukan zona waktu sistem
      --json          Keluaran JSON tunggal (tanpa penjelasan teks)
  -h, --help          Tampilkan bantuan

Field: * | a | a,b | a-b | */n | a-b/n | a/n (a sampai maksimum). Nama bulan
jan-dec dan hari sun-sat boleh dipakai (tanpa membedakan huruf besar/kecil).
Hari 0-7 (0 dan 7 = Minggu); rentang tidak melingkar, tulis 5-7, bukan fri-sun.
Makro: @yearly @annually @monthly @weekly @daily @midnight @hourly.
Jika tanggal DAN hari sama-sama dibatasi (tidak diawali *), waktu cocok bila
salah satunya terpenuhi. --from tanpa offset dibaca di zona yang dipakai.
DST: jam lokal yang terlewat (maju) tidak dijalankan; jam yang berulang
(mundur) hanya dijalankan sekali, pada kemunculan pertama.
Exit: 0 sukses, 1 tidak ada waktu dalam 8 tahun (mis. 0 0 31 2 *), 2 penggunaan salah.
Contoh: dt cron \"*/15 9-17 * * mon-fri\"   |   dt cron -n 3 --utc @daily
";
#[derive(Parser)]
#[command(name="dt cron", override_help=HELP, disable_version_flag=true)]
struct Args {
    #[arg(short = 'n', long)]
    next: Option<String>,
    #[arg(long)]
    from: Option<String>,
    #[arg(long)]
    utc: bool,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}
fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("cron", args)?;
    let tokens: Vec<_> = a
        .positionals
        .iter()
        .flat_map(|s| s.split(crate::tools::time::instant::whitespace))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if tokens.is_empty() {
        return Err(usage("Ekspresi cron wajib diisi, mis. \"*/5 * * * *\"."));
    }
    let count = integer(a.next.as_deref(), 5, 1, 50, "next")? as usize;
    let zone = if a.utc { Zone::utc() } else { Zone::local()? };
    let from = match a.from.as_deref() {
        Some(v) => parse_from(v, &zone)?,
        None => chrono::Utc::now().timestamp_millis(),
    };
    let s = parse_cron(&tokens)?;
    let (times, exhausted) = next_fires(&s, from, count, &zone);
    if a.json {
        print_json(
            &json!({"expression":tokens.join(" "),"fields":{"minute":s.minute,"hour":s.hour,"dayOfMonth":s.dom,"month":s.month,"dayOfWeek":s.dow},"from":format_time(from,&zone,a.utc),"timezone":if a.utc {"utc"} else {"local"},"next":times.iter().map(|t|format_time(*t,&zone,a.utc)).collect::<Vec<_>>()}),
        );
    } else {
        println!("{}\n", explain(&s).join("\n"));
        if !times.is_empty() {
            println!(
                "{}",
                times
                    .iter()
                    .map(|t| format_time(*t, &zone, a.utc))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
    if exhausted {
        return Err(CliError::Failed(if times.is_empty() {
            format!(
                "Tidak ada waktu yang cocok dalam 8 tahun sejak {}.",
                format_time(from, &zone, a.utc)
            )
        } else {
            format!(
                "Hanya {} waktu ditemukan; tidak ada lagi dalam 8 tahun setelahnya.",
                times.len()
            )
        }));
    }
    Ok(())
}
