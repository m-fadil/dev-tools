use super::bytes::{Parsed, Res, Src, err, num, replace_cc};
use serde_json::{Map, Value, json};

/// (name, allowed bit depths) per PNG colour type.
fn color_type(t: u8) -> Option<(&'static str, &'static [u8])> {
    Some(match t {
        0 => ("grayscale", &[1, 2, 4, 8, 16]),
        2 => ("truecolor", &[8, 16]),
        3 => ("indexed", &[1, 2, 4, 8]),
        4 => ("grayscale+alpha", &[8, 16]),
        6 => ("truecolor+alpha", &[8, 16]),
        _ => return None,
    })
}

const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xedb88320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[n] = c;
        n += 1;
    }
    t
}

pub fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = crc_table();
    let mut c = 0xffff_ffffu32;
    for &byte in data {
        c = TABLE[((c ^ byte as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

struct Ihdr {
    width: u32,
    height: u32,
    depth: u8,
    color_type: u8,
    interlace: u8,
}

pub fn parse_png(s: &Src) -> Res<Parsed> {
    let mut off = 8usize;
    let mut ihdr: Option<Ihdr> = None;
    let mut trns = false;
    let mut dpi: Option<(f64, f64)> = None;
    let mut text_keys: Vec<String> = vec![];
    let mut text_count = 0u64;
    let mut ended = false;
    let mut truncated = false;

    loop {
        if !s.has(off, 8) {
            if s.complete {
                return err("file terpotong (tidak ada chunk IEND)");
            }
            truncated = true;
            break;
        }
        let length = s.u32(off, false)? as usize;
        let ty = s.latin1(off + 4, 4)?;
        if !ty.bytes().all(|b| b.is_ascii_alphabetic()) || ty.len() != 4 {
            return err(format!("tipe chunk PNG tidak valid pada offset {}", off + 4));
        }
        if ihdr.is_none() && ty != "IHDR" {
            return err("chunk IHDR tidak ada di awal file");
        }
        if length > 0x7fff_ffff {
            return err(format!("panjang chunk {ty} tidak valid: {length}"));
        }
        let data_start = off + 8;
        let next = data_start + length + 4;
        if next > s.length() {
            if s.complete {
                return err(format!("file terpotong (chunk {ty})"));
            }
            truncated = true;
            break;
        }
        if s.u32(data_start + length, false)? != crc32(&s.b[off + 4..data_start + length]) {
            return err(format!("CRC chunk {ty} salah"));
        }

        match ty.as_str() {
            "IHDR" => {
                if ihdr.is_some() {
                    return err("chunk IHDR ganda");
                }
                if length != 13 {
                    return err(format!("panjang IHDR harus 13, bukan {length}"));
                }
                let width = s.u32(data_start, false)?;
                let height = s.u32(data_start + 4, false)?;
                let depth = s.u8(data_start + 8)?;
                let ct = s.u8(data_start + 9)?;
                let interlace = s.u8(data_start + 12)?;
                if width == 0 || height == 0 || width > 0x7fff_ffff || height > 0x7fff_ffff {
                    return err(format!("ukuran PNG tidak valid: {width}x{height}"));
                }
                let Some((name, depths)) = color_type(ct) else {
                    return err(format!("tipe warna PNG tidak valid: {ct}"));
                };
                if !depths.contains(&depth) {
                    return err(format!("kedalaman bit {depth} tidak valid untuk tipe warna {name}"));
                }
                if s.u8(data_start + 10)? != 0 || s.u8(data_start + 11)? != 0 {
                    return err("metode kompresi/filter PNG tidak dikenal");
                }
                if interlace > 1 {
                    return err(format!("metode interlace PNG tidak valid: {interlace}"));
                }
                ihdr = Some(Ihdr {
                    width,
                    height,
                    depth,
                    color_type: ct,
                    interlace,
                });
            }
            "tRNS" => trns = true,
            "pHYs" => {
                if length != 9 {
                    return err(format!("panjang pHYs harus 9, bukan {length}"));
                }
                let x = s.u32(data_start, false)?;
                let y = s.u32(data_start + 4, false)?;
                // Unit 1 = metre; 0 means only an aspect ratio.
                if s.u8(data_start + 8)? == 1 && x > 0 && y > 0 {
                    dpi = Some(((x as f64 * 2.54).round() / 100.0, (y as f64 * 2.54).round() / 100.0));
                }
            }
            "tEXt" | "zTXt" | "iTXt" => {
                text_count += 1;
                if text_keys.len() < 100 {
                    let raw = &s.b[data_start..data_start + length];
                    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                    let key: String = raw[..end].iter().map(|&c| c as char).collect();
                    text_keys.push(replace_cc(&key));
                }
            }
            "IEND" => {
                ended = true;
                break;
            }
            _ => {}
        }
        off = next;
    }

    let Some(ihdr) = ihdr else {
        return err("chunk IHDR tidak ditemukan");
    };
    let mut warnings = vec![];
    if truncated && !ended {
        warnings.push(s.limit_warning("chunk setelahnya (mis. pHYs, teks) mungkin tidak terbaca"));
    }
    let name = color_type(ihdr.color_type).map_or("", |c| c.0);
    let mut details = Map::new();
    details.insert("bitDepth".into(), json!(ihdr.depth));
    details.insert("colorType".into(), json!(name));
    details.insert("interlaced".into(), json!(ihdr.interlace == 1));
    details.insert(
        "alpha".into(),
        json!(ihdr.color_type == 4 || ihdr.color_type == 6 || trns),
    );
    let dpi_value = match dpi {
        Some((x, y)) => {
            let mut m = Map::new();
            m.insert("x".into(), num(x));
            m.insert("y".into(), num(y));
            Value::Object(m)
        }
        None => Value::Null,
    };
    details.insert("dpi".into(), dpi_value);
    details.insert("textCount".into(), json!(text_count));
    details.insert("textKeys".into(), json!(text_keys));
    Ok(Parsed {
        width: ihdr.width as u64,
        height: ihdr.height as u64,
        truncated,
        warnings,
        details,
    })
}
