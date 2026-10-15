mod render;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;
use qrcode::types::{Color, QrError, Version};
use qrcode::{EcLevel, QrCode};

use crate::errors::{CliError, Result, usage};
use crate::io::{choice, integer, parse, print_json, read_input, write_output};
use crate::tool::Tool;
use render::{Format, ascii, svg, unicode};

pub static TOOL: Tool = Tool {
    name: "qr",
    aliases: &[],
    description: "Buat QR code (Unicode, ASCII, atau SVG) dari teks, file, atau stdin",
    category: "Utilitas",
    run,
};

const HELP: &str = r##"dt qr [options] [teks]

Buat QR code dari byte input (teks, --file, atau stdin; tanpa input: prompt di terminal).
  -f, --file <path>     Baca isi file
  -l, --level <L|M|Q|H> Tingkat koreksi kesalahan (default: M)
  -m, --margin <n>      Zona tenang dalam modul, 0-8 (default: 2)
      --format <jenis>  unicode, ascii, svg (default: unicode)
      --invert          Tukar polaritas unicode/ascii (untuk terminal terang)
      --json            Keluaran JSON: {text_bytes, version, level, width, modules}
  -h, --help            Tampilkan bantuan

Polaritas default untuk terminal GELAP: modul terang digambar sebagai blok
penuh, jadi kode tampak terang di atas latar gelap. Di terminal terang pakai
--invert. Format svg selalu hitam di atas putih dan mengabaikan --invert.
Input dipakai apa adanya (newline akhir dari stdin ikut dikodekan).
modules pada --json: baris berisi 0/1 (1 = gelap), tanpa zona tenang.
Exit: 0 sukses, 1 gagal (mis. input terlalu besar), 2 penggunaan salah.
Contoh: dt qr "https://example.com"
        dt qr --format svg -l H "halo" > halo.svg
"##;

#[derive(Parser)]
#[command(name = "dt qr", disable_help_flag = true, disable_version_flag = true)]
struct Args {
    #[arg(short = 'f', long, overrides_with = "file")]
    file: Option<PathBuf>,
    #[arg(short = 'l', long, overrides_with = "level")]
    level: Option<String>,
    #[arg(short = 'm', long, overrides_with = "margin")]
    margin: Option<String>,
    #[arg(long, overrides_with = "format")]
    format: Option<String>,
    #[arg(long, overrides_with = "invert")]
    invert: bool,
    #[arg(long, overrides_with = "json")]
    json: bool,
    #[arg(short = 'h', long, overrides_with = "help")]
    help: bool,
    positionals: Vec<String>,
}

const LEVELS: &[(&str, EcLevel)] = &[
    ("L", EcLevel::L),
    ("M", EcLevel::M),
    ("Q", EcLevel::Q),
    ("H", EcLevel::H),
];
const FORMATS: &[(&str, Format)] = &[
    ("unicode", Format::Unicode),
    ("ascii", Format::Ascii),
    ("svg", Format::Svg),
];

fn level_name(level: EcLevel) -> &'static str {
    match level {
        EcLevel::L => "L",
        EcLevel::M => "M",
        EcLevel::Q => "Q",
        EcLevel::H => "H",
    }
}

/// Maximum binary payload of a version-40 symbol (ISO/IEC 18004 byte mode).
fn capacity(level: EcLevel) -> usize {
    match level {
        EcLevel::L => 2953,
        EcLevel::M => 2331,
        EcLevel::Q => 1663,
        EcLevel::H => 1273,
    }
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a = parse::<Args>("qr", args)?;
    if a.help {
        write_output(HELP.as_bytes());
        return Ok(());
    }
    let level = choice(a.level.as_deref(), LEVELS, EcLevel::M, "level")?;
    let format = choice(a.format.as_deref(), FORMATS, Format::Unicode, "format")?;
    let margin = integer(a.margin.as_deref(), 2, 0, 8, "margin")? as usize;
    let data = read_input(&a.positionals, a.file.as_deref(), "Teks untuk QR code")?;
    if data.is_empty() {
        return Err(usage("Input kosong: tidak ada yang bisa dikodekan."));
    }
    let code = QrCode::with_error_correction_level(&data, level).map_err(|e| match e {
        QrError::DataTooLong => CliError::Failed(format!(
            "Input terlalu besar: {} byte; kapasitas maksimum level {} adalah {} byte.",
            data.len(),
            level_name(level),
            capacity(level)
        )),
        other => CliError::Failed(format!("Gagal membuat QR code: {other}")),
    })?;
    let version = match code.version() {
        Version::Normal(v) | Version::Micro(v) => v,
    };
    let width = code.width();
    let dark: Vec<bool> = code.into_colors().into_iter().map(|c| c == Color::Dark).collect();

    if a.json {
        let modules: Vec<String> = dark
            .chunks(width)
            .map(|row| row.iter().map(|&d| if d { '1' } else { '0' }).collect())
            .collect();
        print_json(&serde_json::json!({
            "text_bytes": data.len(),
            "version": version,
            "level": level_name(level),
            "width": width,
            "modules": modules,
        }));
        return Ok(());
    }
    let out = match format {
        Format::Unicode => unicode(&dark, width, margin, a.invert),
        Format::Ascii => ascii(&dark, width, margin, a.invert),
        Format::Svg => svg(&dark, width, margin),
    };
    write_output(out.as_bytes());
    Ok(())
}
