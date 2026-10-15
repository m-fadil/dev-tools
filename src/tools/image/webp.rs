use super::bytes::{Parsed, Res, Src, err, js_trim, num};
use serde_json::{Map, Value, json};

pub fn parse_webp(s: &Src) -> Res<Parsed> {
    let riff_size = s.u32w(4, true, "header RIFF")? as u64;
    let total = riff_size + 8;
    if riff_size < 4 + 8 {
        return err(format!("ukuran RIFF tidak valid: {riff_size}"));
    }
    if total > s.file_size {
        return err(format!("file terpotong (RIFF {total} B, file {} B)", s.file_size));
    }

    let ty = s.latin1w(12, 4, "chunk pertama")?;
    let size = s.u32w(16, true, "chunk pertama")? as usize;
    if 20 + size as u64 > total {
        return err(format!("chunk {} melewati akhir RIFF", js_trim(&ty)));
    }
    let body = 20usize;

    let width: u64;
    let height: u64;
    let variant: &str;
    let mut alpha = false;
    let mut animated = false;
    let mut icc = false;
    let mut exif = false;
    let mut xmp = false;
    let mut frames = 1u64;
    let mut loop_count: Option<u16> = None;
    let mut truncated = false;

    match ty.as_str() {
        "VP8 " => {
            variant = "VP8";
            if size < 10 {
                return err("chunk VP8 terlalu pendek");
            }
            if s.u8w(body, "frame VP8")? & 1 != 0 {
                return err("frame VP8 bukan keyframe");
            }
            if s.latin1(body + 3, 3)? != "\u{9d}\u{1}*" {
                return err("start code VP8 tidak valid");
            }
            width = (s.u16(body + 6, true)? & 0x3fff) as u64;
            height = (s.u16(body + 8, true)? & 0x3fff) as u64;
        }
        "VP8L" => {
            variant = "VP8L";
            if size < 5 {
                return err("chunk VP8L terlalu pendek");
            }
            if s.u8w(body, "header VP8L")? != 0x2f {
                return err("signature VP8L tidak valid");
            }
            let bits = s.u32(body + 1, true)?;
            width = ((bits & 0x3fff) + 1) as u64;
            height = (((bits >> 14) & 0x3fff) + 1) as u64;
            alpha = (bits >> 28) & 1 == 1;
            if bits >> 29 != 0 {
                return err(format!("versi VP8L tidak didukung: {}", bits >> 29));
            }
        }
        "VP8X" => {
            variant = "VP8X";
            if size < 10 {
                return err("chunk VP8X terlalu pendek");
            }
            let flags = s.u8w(body, "header VP8X")?;
            icc = flags & 0x20 != 0;
            alpha = flags & 0x10 != 0;
            exif = flags & 0x08 != 0;
            xmp = flags & 0x04 != 0;
            animated = flags & 0x02 != 0;
            s.need(body, 10, "header VP8X")?;
            let le24 = |o: usize| s.b[o] as u64 | (s.b[o + 1] as u64) << 8 | (s.b[o + 2] as u64) << 16;
            width = le24(body + 4) + 1;
            height = le24(body + 7) + 1;

            if animated {
                frames = 0;
                let mut off = (body + size + (size & 1)) as u64;
                while off < total {
                    let o = off as usize;
                    if !s.has(o, 8) {
                        if s.complete {
                            return err("file terpotong (header chunk WebP)");
                        }
                        truncated = true;
                        break;
                    }
                    let chunk = s.latin1(o, 4)?;
                    let length = s.u32(o + 4, true)? as u64;
                    let next = off + 8 + length + (length & 1);
                    if off + 8 + length > total {
                        return err(format!("chunk {chunk} melewati akhir RIFF"));
                    }
                    if chunk == "ANMF" {
                        frames += 1;
                    } else if chunk == "ANIM" && length >= 6 && s.has(o + 8, 6) {
                        loop_count = Some(s.u16(o + 12, true)?);
                    }
                    if off + 8 + length > s.length() as u64 {
                        truncated = true;
                        break;
                    }
                    off = next;
                }
                if frames == 0 && !truncated {
                    return err("WebP animasi tidak berisi frame (ANMF)");
                }
            }
        }
        _ => return err(format!("chunk pertama WebP tidak dikenal: {}", js_trim(&ty))),
    }

    if width == 0 || height == 0 {
        return err(format!("ukuran WebP tidak valid: {width}x{height}"));
    }
    let mut details = Map::new();
    details.insert("variant".into(), json!(variant));
    details.insert("animated".into(), json!(animated));
    details.insert("alpha".into(), json!(alpha));
    details.insert("icc".into(), json!(icc));
    details.insert("exif".into(), json!(exif));
    details.insert("xmp".into(), json!(xmp));
    details.insert("frames".into(), json!(frames));
    details.insert("framesAtLeast".into(), json!(truncated));
    details.insert(
        "loopCount".into(),
        if animated {
            loop_count.map_or(Value::Null, |n| num(n as f64))
        } else {
            Value::Null
        },
    );
    Ok(Parsed {
        width,
        height,
        truncated,
        warnings: if truncated {
            vec![s.limit_warning("jumlah frame minimal sebanyak yang terhitung")]
        } else {
            vec![]
        },
        details,
    })
}
