use super::bytes::{Res, Src, err, js_trim, replace_cc};

#[derive(Default)]
pub struct Exif {
    pub orientation: Option<u16>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub software: Option<String>,
    pub date_time_original: Option<String>,
}

const TYPE_ASCII: u16 = 2;
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;

struct Ctx<'a> {
    s: Src<'a>,
    little: bool,
    exif: Exif,
    warnings: Vec<String>,
    exif_ifd: usize,
}

impl Ctx<'_> {
    fn ascii(&self, ty: u16, count: usize, entry: usize) -> Res<Option<String>> {
        if ty != TYPE_ASCII || count == 0 {
            return Ok(None);
        }
        let start = if count <= 4 {
            entry + 8
        } else {
            self.s.u32(entry + 8, self.little)? as usize
        };
        self.s.need(start, count, "string EXIF")?;
        let raw = &self.s.b[start..start + count];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let text = replace_cc(&String::from_utf8_lossy(&raw[..end]));
        let trimmed = js_trim(&text);
        Ok(if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        })
    }

    /// `main` selects the 0th IFD (all wanted tags) or the Exif sub-IFD (DateTimeOriginal only).
    fn on_tag(&mut self, main: bool, tag: u16, ty: u16, count: usize, entry: usize) -> Res<()> {
        if !main {
            if tag == 0x9003 {
                self.exif.date_time_original = self.ascii(ty, count, entry)?;
            }
            return Ok(());
        }
        match tag {
            0x010f => self.exif.make = self.ascii(ty, count, entry)?,
            0x0110 => self.exif.model = self.ascii(ty, count, entry)?,
            0x0131 => self.exif.software = self.ascii(ty, count, entry)?,
            0x0112 if ty == TYPE_SHORT && count == 1 => {
                let value = self.s.u16(entry + 8, self.little)?;
                if (1..=8).contains(&value) {
                    self.exif.orientation = Some(value);
                } else {
                    self.warnings
                        .push(format!("EXIF: nilai orientasi tidak valid: {value}"));
                }
            }
            0x8769 if ty == TYPE_LONG && count == 1 => self.exif_ifd = self.s.u32(entry + 8, self.little)? as usize,
            _ => {}
        }
        Ok(())
    }

    fn read_ifd(&mut self, main: bool, offset: usize) {
        let count = match self.s.u16w(offset, self.little, "IFD") {
            Ok(c) => c as usize,
            Err(e) => {
                self.warnings.push(format!("EXIF rusak: {}", e.message));
                return;
            }
        };
        for i in 0..count {
            let entry = offset + 2 + i * 12;
            if !self.s.has(entry, 12) {
                self.warnings.push("EXIF rusak: file terpotong (entri IFD)".to_string());
                break;
            }
            let little = self.little;
            let fields = (|| {
                Ok((
                    self.s.u16(entry, little)?,
                    self.s.u16(entry + 2, little)?,
                    self.s.u32(entry + 4, little)?,
                ))
            })();
            let result = fields.and_then(|(tag, ty, count)| self.on_tag(main, tag, ty, count as usize, entry));
            if let Err(e) = result {
                self.warnings.push(format!("EXIF rusak: {}", e.message));
            }
        }
    }

    fn run(&mut self) -> Res<()> {
        let order = self.s.latin1w(0, 2, "header TIFF")?;
        if order != "II" && order != "MM" {
            return err("urutan byte TIFF tidak dikenal");
        }
        self.little = order == "II";
        if self.s.u16(2, self.little)? != 42 {
            return err("magic TIFF bukan 42");
        }
        let first = self.s.u32(4, self.little)? as usize;
        self.read_ifd(true, first);
        if self.exif_ifd != 0 {
            self.read_ifd(false, self.exif_ifd);
        }
        Ok(())
    }
}

/// Reads the few wanted tags from a TIFF structure (APP1 payload after "Exif\0\0").
/// Both byte orders are supported. Damage never fails the image: it becomes a warning and keeps what was read.
pub fn parse_exif(tiff: &[u8]) -> (Exif, Vec<String>) {
    // Complete: the TIFF block is the whole universe, any overrun is damage.
    let mut ctx = Ctx {
        s: Src::new(tiff, tiff.len() as u64),
        little: false,
        exif: Exif::default(),
        warnings: vec![],
        exif_ifd: 0,
    };
    if let Err(e) = ctx.run() {
        ctx.warnings.push(format!("EXIF rusak: {}", e.message));
    }
    (ctx.exif, ctx.warnings)
}
