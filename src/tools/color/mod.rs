mod names;
mod rgba;

use crate::errors::{Result, usage};
use crate::io;
use crate::numfmt::js_number;
use crate::tool::Tool;
use clap::{ArgAction, Parser};
use rgba::{
    Contrast, Rgba, contrast, luminance, name_of, parse_color, ratio_value, to_hex, to_hsl, to_hsl_string, to_hwb,
    to_hwb_string, to_rgb_string,
};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::str::FromStr;

pub static TOOL: Tool = Tool {
    name: "color",
    aliases: &[],
    description: "Konversi warna (hex, rgb, hsl, hwb, nama CSS) dan kontras WCAG",
    category: "Utilitas",
    run,
};

const FORMATS: [(&str, Format); 5] = [
    ("hex", Format::Hex),
    ("rgb", Format::Rgb),
    ("hsl", Format::Hsl),
    ("hwb", Format::Hwb),
    ("name", Format::Name),
];

#[derive(Clone, Copy)]
enum Format {
    Hex,
    Rgb,
    Hsl,
    Hwb,
    Name,
}

const HELP: &str = "dt color [options] <warna>

Konversi dan analisis warna: hex, rgb, hsl, hwb, nama CSS, luminans, dan kontras WCAG.
Warna dari argumen (digabung spasi) atau stdin. Format masukan:
  hex      #rgb  #rgba  #rrggbb  #rrggbbaa  (tanpa '#': hanya 6 atau 8 digit)
  rgb()    rgb(255, 127, 80)  rgba(255,127,80,.5)  rgb(255 127 80 / 50%)  rgb(100% 50% 0%)
  hsl()    hsl(16, 100%, 66%)  hsl(0.5turn 50% 50% / .3)  satuan hue: deg, grad, rad, turn
  hwb()    hwb(16 31% 0%)  (hanya sintaks spasi)
  nama     148 nama CSS (tanpa membedakan huruf besar/kecil) dan transparent
  -f, --format <jenis>    Cetak hanya satu nilai: hex, rgb, hsl, hwb, name
      --contrast <warna2> Rasio kontras antara <warna> dan <warna2>
      --json              Keluaran JSON
  -h, --help              Tampilkan bantuan

Pembulatan: kanal rgb dan hsl/hwb ke bilangan bulat; hsl/hwb dihitung dari rgb yang
sudah dibulatkan. Alpha dibulatkan sekali ke 3 desimal saat parsing; hex 8 digit bila
alpha hasil pembulatan < 1. Nama dengan '~' adalah nama terdekat (jarak RGB kuadrat);
alpha diabaikan untuk nama kecuali transparent (RGB hitam, alpha 0), luminans, dan kontras.
Rasio kontras dipotong (bukan dibulatkan) ke 2 desimal. Nilai AA/AAA memakai ambang
WCAG 2.x: teks normal 4.5 / 7, teks besar 3 / 4.5.
Contoh: dt color coral   |   dt color '#fff' --contrast '#767676'
";

#[derive(Parser)]
#[command(name = "dt color", override_help = HELP, disable_version_flag = true)]
struct Args {
    #[arg(short = 'f', long, overrides_with = "format")]
    format: Option<String>,
    #[arg(long, overrides_with = "contrast")]
    contrast: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    json: bool,
    #[arg()]
    positionals: Vec<String>,
}

fn verdict(ok: bool) -> &'static str {
    if ok { "lulus" } else { "gagal" }
}

/// A JS number as a JSON number (printed exactly as `JSON.stringify` would).
fn num(x: f64) -> Value {
    serde_json::Number::from_str(&js_number(x))
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn contrast_json(c: &Contrast) -> Value {
    json!({
        "ratio": num(ratio_value(c.ratio)),
        "aa": c.aa_normal,
        "aaa": c.aaa_normal,
        "aaLarge": c.aa_large,
        "aaaLarge": c.aaa_large,
    })
}

fn normal_verdicts(c: &Contrast) -> String {
    format!(
        "{}:1  AA {}, AAA {}",
        js_number(ratio_value(c.ratio)),
        verdict(c.aa_normal),
        verdict(c.aaa_normal)
    )
}

const WHITE: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 1.0,
};
const BLACK: Rgba = Rgba {
    r: 0,
    g: 0,
    b: 0,
    a: 1.0,
};

fn describe(input: &str, c: Rgba) -> Value {
    let hsl = to_hsl(c);
    let hwb = to_hwb(c);
    let (name, exact) = name_of(c);
    // Number(luminance.toFixed(6))
    let lum = format!("{:.6}", luminance(c)).parse::<f64>().unwrap_or(0.0);
    json!({
        "input": input,
        "hex": to_hex(c),
        "rgb": { "r": c.r, "g": c.g, "b": c.b },
        "hsl": { "h": hsl.h, "s": hsl.s, "l": hsl.l },
        "hwb": { "h": hwb.h, "w": hwb.w, "b": hwb.b },
        "css": { "rgb": to_rgb_string(c), "hsl": to_hsl_string(c), "hwb": to_hwb_string(c) },
        "alpha": num(c.a),
        "name": if exact { Value::String(name.to_string()) } else { Value::Null },
        "nearestName": name,
        "luminance": num(lum),
        "contrast": {
            "white": contrast_json(&contrast(c, WHITE)),
            "black": contrast_json(&contrast(c, BLACK)),
        },
    })
}

fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = io::parse("color", args)?;
    let format = match a.format.as_deref() {
        None => None,
        Some(v) => Some(io::choice(Some(v), &FORMATS, Format::Hex, "format")?),
    };
    if format.is_some() && a.contrast.is_some() {
        return Err(usage("--format tidak bisa dipakai bersama --contrast."));
    }
    if format.is_some() && a.json {
        return Err(usage("--json tidak bisa dipakai bersama --format."));
    }

    let data = io::read_input(&a.positionals, None, "Warna:")?;
    let text = String::from_utf8_lossy(&data);
    let text = rgba::js_trim(&text);
    let c = parse_color(text)?;

    if let Some(other_text) = &a.contrast {
        let other = parse_color(other_text)?;
        let k = contrast(c, other);
        if a.json {
            io::print_json(&json!({
                "foreground": { "input": text, "hex": to_hex(c) },
                "background": { "input": rgba::js_trim(other_text), "hex": to_hex(other) },
                "ratio": num(ratio_value(k.ratio)),
                "aa": k.aa_normal,
                "aaa": k.aaa_normal,
                "aaLarge": k.aa_large,
                "aaaLarge": k.aaa_large,
            }));
            return Ok(());
        }
        println!(
            "Rasio kontras {} vs {}: {}:1\n  Teks normal  AA (4.5): {}   AAA (7): {}\n  Teks besar   AA (3):   {}   AAA (4.5): {}",
            to_hex(c),
            to_hex(other),
            js_number(ratio_value(k.ratio)),
            verdict(k.aa_normal),
            verdict(k.aaa_normal),
            verdict(k.aa_large),
            verdict(k.aaa_large),
        );
        return Ok(());
    }

    if a.json {
        io::print_json(&describe(text, c));
        return Ok(());
    }
    let (name, exact) = name_of(c);
    let name_text = if exact { name.to_string() } else { format!("~{name}") };
    match format {
        Some(Format::Hex) => println!("{}", to_hex(c)),
        Some(Format::Rgb) => println!("{}", to_rgb_string(c)),
        Some(Format::Hsl) => println!("{}", to_hsl_string(c)),
        Some(Format::Hwb) => println!("{}", to_hwb_string(c)),
        Some(Format::Name) => println!("{name_text}"),
        None => println!(
            "hex        {}\nrgb        {}\nhsl        {}\nhwb        {}\nnama       {}\nalpha      {}\nluminans   {:.4}\nvs putih   {}\nvs hitam   {}",
            to_hex(c),
            to_rgb_string(c),
            to_hsl_string(c),
            to_hwb_string(c),
            name_text,
            js_number(c.a),
            luminance(c),
            normal_verdicts(&contrast(c, WHITE)),
            normal_verdicts(&contrast(c, BLACK)),
        ),
    }
    Ok(())
}
