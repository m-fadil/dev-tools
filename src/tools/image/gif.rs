use super::bytes::{Parsed, Res, Src, err, num};
use serde_json::{Map, Value, json};

struct Scan {
    frames: u64,
    loop_count: Option<u16>,
}

/// Skips a chain of data sub-blocks, returning the offset after the 0-length terminator.
fn skip_sub_blocks(s: &Src, from: usize) -> Res<usize> {
    let mut at = from;
    loop {
        let size = s.u8w(at, "sub-blok GIF")? as usize;
        at += 1;
        if size == 0 {
            return Ok(at);
        }
        s.need(at, size, "sub-blok GIF")?;
        at += size;
    }
}

fn scan(s: &Src, global_colors: usize, state: &mut Scan) -> Res<()> {
    let mut off = 13 + global_colors * 3;
    s.need(13, global_colors * 3, "tabel warna global")?;
    loop {
        let block = s.u8w(off, "trailer GIF")?;
        off += 1;
        if block == 0x3b {
            return Ok(());
        }
        if block == 0x21 {
            let label = s.u8w(off, "ekstensi GIF")?;
            off += 1;
            if label == 0xff && s.u8(off)? == 11 {
                let id = s.latin1(off + 1, 11)?;
                if id == "NETSCAPE2.0" || id == "ANIMEXTS1.0" {
                    let at = off + 12;
                    if s.u8(at)? >= 3 && s.u8(at + 1)? == 1 {
                        state.loop_count = Some(s.u16(at + 2, true)?);
                    }
                }
            }
            off = skip_sub_blocks(s, off)?;
        } else if block == 0x2c {
            let packed = s.u8w(off + 8, "deskriptor gambar")?;
            off += 9;
            if packed & 0x80 != 0 {
                off += 3 * (1usize << ((packed & 7) + 1));
            }
            s.u8w(off, "data gambar")?; // LZW minimum code size
            off = skip_sub_blocks(s, off + 1)?;
            state.frames += 1;
        } else {
            return err(format!("blok GIF tidak valid (0x{block:x}) pada offset {}", off - 1));
        }
    }
}

pub fn parse_gif(s: &Src) -> Res<Parsed> {
    let version = s.latin1(3, 3)?;
    let width = s.u16w(6, true, "layar logis")?;
    let height = s.u16w(8, true, "layar logis")?;
    if width == 0 || height == 0 {
        return err(format!("ukuran GIF tidak valid: {width}x{height}"));
    }
    let flags = s.u8(10)?;
    let global_colors: usize = if flags & 0x80 != 0 { 1 << ((flags & 7) + 1) } else { 0 };
    let mut state = Scan {
        frames: 0,
        loop_count: None,
    };
    let mut truncated = false;

    if let Err(e) = scan(s, global_colors, &mut state) {
        // Running out of bytes is only acceptable when the read limit (not the file) cut the scan short.
        if !e.short || s.complete {
            return Err(e);
        }
        truncated = true;
    }

    if state.frames == 0 && !truncated {
        return err("GIF tidak berisi frame");
    }
    let mut details = Map::new();
    details.insert("version".into(), json!(version));
    details.insert("colorTableSize".into(), json!(global_colors));
    details.insert("frames".into(), json!(state.frames));
    details.insert("framesAtLeast".into(), json!(truncated));
    details.insert(
        "animated".into(),
        if truncated && state.frames < 2 {
            Value::Null
        } else {
            json!(state.frames > 1)
        },
    );
    details.insert(
        "loopCount".into(),
        state.loop_count.map_or(Value::Null, |n| num(n as f64)),
    );
    Ok(Parsed {
        width: width as u64,
        height: height as u64,
        truncated,
        warnings: if truncated {
            vec![s.limit_warning("jumlah frame minimal sebanyak yang terhitung")]
        } else {
            vec![]
        },
        details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_frame_count_is_a_lower_bound() {
        let header = b"GIF89a\x02\0\x02\0\x80\0\0\0\0\0\0\0\0";
        let frame = [
            0x21, 0xf9, 4, 0, 10, 0, 0, 0, 0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 1, 0,
        ];
        let full = [
            header.as_slice(),
            &frame,
            &frame,
            &[0x21, 0xfe, 255],
            &[0; 255],
            &[0],
            &frame,
            &[0x3b],
        ]
        .concat();
        let length = header.len() + 2 * frame.len() + 10;
        let parsed = parse_gif(&Src::new(&full[..length], full.len() as u64)).unwrap();
        assert_eq!(parsed.details["frames"], 2);
        assert_eq!(parsed.details["framesAtLeast"], true);
        assert_eq!(parsed.details["animated"], true);
        assert!(parsed.truncated);
        assert!(parsed.warnings[0].contains("pertama"));
        assert!(parse_gif(&Src::new(&full[..length], length as u64)).is_err());
        assert_eq!(
            parse_gif(&Src::new(&full, full.len() as u64)).unwrap().details["frames"],
            3
        );
    }
}
