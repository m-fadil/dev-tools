use super::bytes::{Parsed, Res, Src, err};
use super::exif::parse_exif;
use serde_json::{Map, Value, json};

const DENSITY_UNITS: [&str; 3] = ["none", "dpi", "dpcm"];

struct Sof {
    marker: u8,
    precision: u8,
    height: u16,
    width: u16,
    components: u8,
}

pub fn parse_jpeg(s: &Src) -> Res<Parsed> {
    let mut off = 2usize;
    let mut sof: Option<Sof> = None;
    let mut jfif: Option<Value> = None;
    let mut exif: Option<Value> = None;
    let mut warnings: Vec<String> = vec![];
    let mut truncated = false;

    loop {
        if !s.has(off, 2) {
            // Out of data before SOS/EOI: only acceptable when the read limit (not the file) ended it.
            if sof.is_none() || s.complete {
                return err(if sof.is_some() {
                    "file terpotong (tidak ada SOS/EOI)"
                } else {
                    "file terpotong (SOF tidak ditemukan)"
                });
            }
            truncated = true;
            break;
        }
        if s.u8(off)? != 0xff {
            return err(format!("marker JPEG tidak valid pada offset {off}"));
        }
        let marker = s.u8(off + 1)?;
        if marker == 0xff {
            off += 1; // fill byte
            continue;
        }
        if marker == 0x00 {
            return err(format!("marker JPEG tidak valid pada offset {off}"));
        }
        off += 2;
        if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if marker == 0xd9 {
            break; // EOI
        }
        if marker == 0xda {
            break; // SOS: entropy coded data follows, no more metadata
        }

        if !s.has(off, 2) {
            if sof.is_none() || s.complete {
                return err("file terpotong (panjang segmen JPEG)");
            }
            truncated = true;
            break;
        }

        let length = s.u16w(off, false, "panjang segmen")? as usize;
        if length < 2 {
            return err(format!("panjang segmen JPEG tidak valid pada offset {off}"));
        }
        let start = off + 2;
        let end = off + length;
        if end > s.length() {
            if sof.is_none() || s.complete {
                return err(if s.complete {
                    "file terpotong (segmen JPEG)"
                } else {
                    "segmen JPEG melewati batas baca sebelum SOF"
                });
            }
            truncated = true;
            break;
        }
        let size = length - 2;

        let is_sof = (0xc0..=0xcf).contains(&marker) && marker != 0xc4 && marker != 0xc8 && marker != 0xcc;
        if is_sof && sof.is_none() {
            if size < 6 {
                return err("segmen SOF terlalu pendek");
            }
            let components = s.u8(start + 5)?;
            if components == 0 || size < 6 + 3 * components as usize {
                return err("segmen SOF tidak valid");
            }
            let height = s.u16(start + 1, false)?;
            let width = s.u16(start + 3, false)?;
            if width == 0 || height == 0 {
                return err(format!(
                    "ukuran JPEG tidak valid: {width}x{height} (tinggi 0/DNL tidak didukung)"
                ));
            }
            sof = Some(Sof {
                marker,
                precision: s.u8(start)?,
                height,
                width,
                components,
            });
        } else if marker == 0xe0 && jfif.is_none() && size >= 14 && s.bytes(start, 5, "data")? == b"JFIF\0" {
            let unit = s.u8(start + 7)?;
            if unit <= 2 {
                jfif = Some(json!({
                    "version": format!("{}.{:02}", s.u8(start + 5)?, s.u8(start + 6)?),
                    "density": {
                        "x": s.u16(start + 8, false)?,
                        "y": s.u16(start + 10, false)?,
                        "unit": DENSITY_UNITS[unit as usize],
                    },
                }));
            } else {
                warnings.push(format!("JFIF: satuan densitas tidak valid: {unit}"));
            }
        } else if marker == 0xe1 && exif.is_none() && size >= 6 && s.bytes(start, 6, "data")? == b"Exif\0\0" {
            let (parsed, w) = parse_exif(&s.b[start + 6..end]);
            exif = Some(json!({
                "orientation": parsed.orientation,
                "make": parsed.make,
                "model": parsed.model,
                "software": parsed.software,
                "dateTimeOriginal": parsed.date_time_original,
            }));
            warnings.extend(w);
        }
        off = end;
    }

    let Some(sof) = sof else {
        return err("SOF tidak ditemukan (bukan JPEG yang utuh)");
    };
    if truncated {
        warnings.push(s.limit_warning("segmen setelahnya (mis. EXIF) mungkin tidak terbaca"));
    }
    let mut details = Map::new();
    details.insert("precision".into(), json!(sof.precision));
    details.insert("components".into(), json!(sof.components));
    details.insert("sof".into(), json!(format!("SOF{}", sof.marker - 0xc0)));
    details.insert(
        "progressive".into(),
        json!(matches!(sof.marker, 0xc2 | 0xc6 | 0xca | 0xce)),
    );
    details.insert("jfif".into(), jfif.unwrap_or(Value::Null));
    details.insert("exif".into(), exif.unwrap_or(Value::Null));
    Ok(Parsed {
        width: sof.width as u64,
        height: sof.height as u64,
        truncated,
        warnings,
        details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_segment_lengths_keep_known_dimensions() {
        let prefix = [255, 216, 255, 192, 0, 11, 8, 0, 2, 0, 3, 1, 0, 0, 0];
        for tail in [&[255, 225][..], &[255, 225, 0][..]] {
            let data = [prefix.as_slice(), tail].concat();
            let parsed = parse_jpeg(&Src::new(&data, data.len() as u64 + 10)).unwrap();
            assert_eq!(parsed.width, 3);
            assert!(parsed.truncated);
            assert!(parsed.warnings[0].contains("pertama"));
            assert!(
                parse_jpeg(&Src::new(&data, data.len() as u64))
                    .err()
                    .unwrap()
                    .message
                    .contains("panjang segmen")
            );
            let no_sof = [&[255, 216][..], tail].concat();
            assert!(
                parse_jpeg(&Src::new(&no_sof, no_sof.len() as u64 + 10))
                    .err()
                    .unwrap()
                    .message
                    .contains("panjang segmen")
            );
        }
    }
}
