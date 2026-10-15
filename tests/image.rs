mod common;
use common::dt;
use serde_json::{Value, json};
use std::io::Write;

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}
fn crc(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in data {
        c ^= u32::from(*b);
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !c
}
fn chunk(ty: &str, data: &[u8]) -> Vec<u8> {
    let body = cat(&[ty.as_bytes(), data]);
    cat(&[&(data.len() as u32).to_be_bytes(), &body, &crc(&body).to_be_bytes()])
}
fn ihdr(w: u32, h: u32, depth: u8, ct: u8, interlace: u8) -> Vec<u8> {
    chunk(
        "IHDR",
        &cat(&[&w.to_be_bytes(), &h.to_be_bytes(), &[depth, ct, 0, 0, interlace]]),
    )
}
fn png(parts: &[Vec<u8>]) -> Vec<u8> {
    cat(&[b"\x89PNG\r\n\x1a\n", &parts.concat()])
}
fn idat() -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&[0; 5]).unwrap();
    chunk("IDAT", &z.finish().unwrap())
}
fn segment(marker: u8, data: &[u8]) -> Vec<u8> {
    cat(&[&[255, marker], &((data.len() + 2) as u16).to_be_bytes(), data])
}
fn sof(marker: u8, w: u16, h: u16, components: u8) -> Vec<u8> {
    segment(
        marker,
        &cat(&[
            &[8],
            &h.to_be_bytes(),
            &w.to_be_bytes(),
            &[components],
            &vec![0; components as usize * 3],
        ]),
    )
}
fn sos() -> Vec<u8> {
    segment(0xda, &[1, 1, 0, 0, 63, 0])
}
fn jpeg(parts: &[Vec<u8>]) -> Vec<u8> {
    cat(&[&[255, 216], &parts.concat(), &[18, 52, 255, 217]])
}
fn u16b(n: u16, le: bool) -> [u8; 2] {
    if le { n.to_le_bytes() } else { n.to_be_bytes() }
}
fn u32b(n: u32, le: bool) -> [u8; 4] {
    if le { n.to_le_bytes() } else { n.to_be_bytes() }
}
fn entry(tag: u16, ty: u16, count: u32, value: &[u8], le: bool) -> Vec<u8> {
    cat(&[&u16b(tag, le), &u16b(ty, le), &u32b(count, le), value])
}
fn tiff(le: bool, orientation: u16) -> Vec<u8> {
    let mut strings = Vec::new();
    let mut text = |tag, s: &str| {
        let raw = cat(&[s.as_bytes(), &[0]]);
        let val = if raw.len() <= 4 {
            let mut v = raw.clone();
            v.resize(4, 0);
            v
        } else {
            let v = u32b(92 + strings.len() as u32, le).to_vec();
            strings.extend_from_slice(&raw);
            v
        };
        entry(tag, 2, raw.len() as u32, &val, le)
    };
    let make = text(0x10f, "Acme Corp");
    let model = text(0x110, "Cam 1");
    let software = text(0x131, "v1");
    let date = text(0x9003, "2024:01:02 03:04:05");
    cat(&[
        if le { b"II" } else { b"MM" },
        &u16b(42, le),
        &u32b(8, le),
        &u16b(5, le),
        &make,
        &model,
        &entry(0x112, 3, 1, &cat(&[&u16b(orientation, le), &[0, 0]]), le),
        &software,
        &entry(0x8769, 4, 1, &u32b(74, le), le),
        &[0; 4],
        &u16b(1, le),
        &date,
        &[0; 4],
        &strings,
    ])
}
fn exif(data: &[u8]) -> Vec<u8> {
    segment(0xe1, &cat(&[b"Exif\0\0", data]))
}
fn gif_header(version: &str, w: u16, h: u16, colors: u8) -> Vec<u8> {
    cat(&[
        b"GIF",
        version.as_bytes(),
        &w.to_le_bytes(),
        &h.to_le_bytes(),
        &[0x80 | (colors.ilog2() as u8 - 1), 0, 0],
        &vec![0; colors as usize * 3],
    ])
}
fn frame() -> Vec<u8> {
    vec![
        0x21, 0xf9, 4, 0, 10, 0, 0, 0, 0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 1, 0,
    ]
}
fn netscape(n: u16) -> Vec<u8> {
    cat(&[&[0x21, 255, 11], b"NETSCAPE2.0", &[3, 1], &n.to_le_bytes(), &[0]])
}
fn riff_chunk(ty: &str, data: &[u8]) -> Vec<u8> {
    cat(&[
        ty.as_bytes(),
        &(data.len() as u32).to_le_bytes(),
        data,
        &vec![0; data.len() & 1],
    ])
}
fn webp(parts: &[Vec<u8>]) -> Vec<u8> {
    let body = cat(&[b"WEBP", &parts.concat()]);
    cat(&[b"RIFF", &(body.len() as u32).to_le_bytes(), &body])
}
fn vp8l(w: u32, h: u32, alpha: bool) -> Vec<u8> {
    riff_chunk(
        "VP8L",
        &cat(&[
            &[0x2f],
            &((w - 1) | ((h - 1) << 14) | if alpha { 1 << 28 } else { 0 }).to_le_bytes(),
            &[0, 0],
        ]),
    )
}
fn vp8x(flags: u8, w: u32, h: u32) -> Vec<u8> {
    riff_chunk(
        "VP8X",
        &cat(&[
            &[flags, 0, 0, 0],
            &(w - 1).to_le_bytes()[..3],
            &(h - 1).to_le_bytes()[..3],
        ]),
    )
}
struct Fixtures(tempfile::TempDir);
impl Fixtures {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }
    fn put(&self, name: &str, data: &[u8]) -> String {
        let path = self.0.path().join(name);
        std::fs::write(&path, data).unwrap();
        path.to_str().unwrap().into()
    }
    fn inspect(&self, name: &str, data: &[u8]) -> Value {
        let path = self.put(name, data);
        let out = dt(&["image", "--json", &path]);
        serde_json::from_str::<Value>(&out.out).unwrap()["files"][0].clone()
    }
    fn bad(&self, name: &str, data: &[u8], reason: &str) {
        let path = self.put(name, data);
        let r = dt(&["image", &path]);
        assert_eq!(r.code, 1, "{name}");
        assert_eq!(r.out, "");
        assert!(r.err.contains(reason), "{name}: {}", r.err);
    }
}
#[test]
fn png_metadata_and_text() {
    let f = Fixtures::new();
    let data = png(&[
        ihdr(1920, 1080, 8, 6, 0),
        chunk("pHYs", &cat(&[&2835u32.to_be_bytes(), &2835u32.to_be_bytes(), &[1]])),
        chunk("tEXt", b"Title\0Halo"),
        chunk("iTXt", b"Author\0\0\0\0\0Budi"),
        idat(),
        chunk("IEND", &[]),
    ]);
    let i = f.inspect("full.png", &data);
    for (k, v) in [
        ("ok", json!(true)),
        ("format", json!("PNG")),
        ("mime", json!("image/png")),
        ("width", json!(1920)),
        ("height", json!(1080)),
        ("aspectRatio", json!("16:9")),
        ("bitDepth", json!(8)),
        ("colorType", json!("truecolor+alpha")),
        ("interlaced", json!(false)),
        ("alpha", json!(true)),
        ("dpi", json!({"x":72.01,"y":72.01})),
        ("textCount", json!(2)),
        ("textKeys", json!(["Title", "Author"])),
        ("truncated", json!(false)),
        ("warnings", json!([])),
        ("size", json!(data.len())),
    ] {
        assert_eq!(i[k], v, "{k}");
    }
    let p = f.put(
        "text.png",
        &png(&[ihdr(2, 2, 1, 3, 1), chunk("tRNS", &[0]), idat(), chunk("IEND", &[])]),
    );
    let r = dt(&["image", &p]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with(&format!("{p}:\n")));
    for s in ["PNG", "indexed", "ya", "1:1"] {
        assert!(r.out.contains(s));
    }
    let gray = f.inspect("gray.png", &png(&[ihdr(1, 1, 8, 0, 0), idat(), chunk("IEND", &[])]));
    assert_eq!(gray["colorType"], "grayscale");
    assert_eq!(gray["alpha"], false);
    assert_eq!(gray["dpi"], Value::Null);
}
#[test]
fn png_many_text_and_validation() {
    let f = Fixtures::new();
    let mut parts = vec![ihdr(1, 1, 8, 6, 0), chunk("tEXt", b"A\x1b\x7f\x85\0value")];
    parts.extend(std::iter::repeat_n(chunk("tEXt", &[]), 340000));
    parts.push(chunk("IEND", &[]));
    let data = png(&parts);
    let i = f.inspect("many.png", &data);
    assert_eq!(i["textCount"], 340001);
    assert_eq!(i["textKeys"].as_array().unwrap().len(), 100);
    assert_eq!(i["textKeys"][0], "A���");
    assert_eq!(i["truncated"], false);
    let out = dt(&["image", &f.put("many.png", &data)]);
    assert!(out.out.contains("A���"));
    assert!(!out.out.contains('\x1b'));
    for end in [chunk("IEND", &[]), chunk("IEND", &[1])] {
        assert_eq!(f.inspect("no-idat.png", &png(&[ihdr(1, 1, 8, 6, 0), end]))["ok"], true);
    }
    let mut bad = chunk("tEXt", b"Title\0value");
    *bad.last_mut().unwrap() ^= 1;
    f.bad(
        "crc.png",
        &png(&[ihdr(1, 1, 8, 6, 0), bad, chunk("IEND", &[])]),
        "CRC chunk tEXt salah",
    );
    let good = png(&[ihdr(1, 1, 8, 6, 0), idat(), chunk("IEND", &[])]);
    for (n, end, msg) in [
        ("sig", 8, "terpotong"),
        ("ihdr", 20, "terpotong"),
        ("idat", 43, "terpotong (chunk IDAT)"),
        ("no-end", good.len() - 12, "tidak ada chunk IEND"),
        ("cut-end", good.len() - 3, "terpotong"),
    ] {
        f.bad(n, &good[..end], msg);
    }
    let mut bad = good.clone();
    bad[29..33].fill(0);
    f.bad("ihdr-crc", &bad, "CRC chunk IHDR");
    for (n, data, msg) in [
        ("first", png(&[idat(), chunk("IEND", &[])]), "IHDR tidak ada di awal"),
        (
            "zero",
            png(&[ihdr(0, 1, 8, 6, 0), chunk("IEND", &[])]),
            "ukuran PNG tidak valid",
        ),
        (
            "depth",
            png(&[ihdr(1, 1, 16, 3, 0), chunk("IEND", &[])]),
            "kedalaman bit 16",
        ),
        (
            "type",
            png(&[ihdr(1, 1, 8, 5, 0), chunk("IEND", &[])]),
            "tipe warna PNG tidak valid: 5",
        ),
    ] {
        f.bad(n, &data, msg);
    }
    let i = f.inspect(
        "big.png",
        &png(&[
            ihdr(10, 20, 8, 6, 0),
            chunk("IDAT", &vec![7; 5 * 1024 * 1024]),
            chunk("pHYs", &[0; 9]),
            chunk("IEND", &[]),
        ]),
    );
    assert_eq!(i["ok"], true);
    assert_eq!(i["width"], 10);
    assert_eq!(i["truncated"], true);
    assert_eq!(i["dpi"], Value::Null);
    assert!(i["warnings"][0].as_str().unwrap().contains("4 MiB pertama"));
}
#[test]
fn jpeg_metadata_and_exif_recovery() {
    let f = Fixtures::new();
    let jfif = segment(0xe0, b"JFIF\0\x01\x02\x01\0H\0H\0\0");
    for le in [true, false] {
        let i = f.inspect(
            "exif.jpg",
            &jpeg(&[jfif.clone(), exif(&tiff(le, 6)), sof(0xc0, 640, 480, 3), sos()]),
        );
        for (k, v) in [
            ("ok", json!(true)),
            ("format", json!("JPEG")),
            ("mime", json!("image/jpeg")),
            ("width", json!(640)),
            ("height", json!(480)),
            ("aspectRatio", json!("4:3")),
            ("precision", json!(8)),
            ("components", json!(3)),
            ("progressive", json!(false)),
            ("jfif", json!({"version":"1.02","density":{"x":72,"y":72,"unit":"dpi"}})),
            (
                "exif",
                json!({"orientation":6,"make":"Acme Corp","model":"Cam 1","software":"v1","dateTimeOriginal":"2024:01:02 03:04:05"}),
            ),
            ("warnings", json!([])),
        ] {
            assert_eq!(i[k], v, "{k}");
        }
        let mut damaged = tiff(le, 6);
        let offset = damaged.len() as u32 + 100;
        damaged[18..22].copy_from_slice(&u32b(offset, le));
        let i = f.inspect("bad-make.jpg", &jpeg(&[exif(&damaged), sof(0xc0, 8, 8, 3), sos()]));
        assert_eq!(i["exif"]["make"], Value::Null);
        assert_eq!(i["exif"]["model"], "Cam 1");
        assert_eq!(i["exif"]["orientation"], 6);
        assert_eq!(i["exif"]["dateTimeOriginal"], "2024:01:02 03:04:05");
        assert!(i["warnings"][0].as_str().unwrap().contains("EXIF rusak"));
        let partial = cat(&[
            if le { b"II" } else { b"MM" },
            &u16b(42, le),
            &u32b(26, le),
            &u16b(1, le),
            &entry(0x9003, 2, 4, b"now\0", le),
            &[0; 4],
            &u16b(2, le),
            &entry(0x8769, 4, 1, &u32b(8, le), le),
        ]);
        let i = f.inspect("partial.jpg", &jpeg(&[exif(&partial), sof(0xc0, 1, 1, 3), sos()]));
        assert_eq!(i["exif"]["dateTimeOriginal"], "now");
        assert!(i["warnings"][0].as_str().unwrap().contains("EXIF rusak"));
    }
    let i = f.inspect("prog.jpg", &jpeg(&[sof(0xc2, 3, 1, 1), sos()]));
    assert_eq!(i["progressive"], true);
    assert_eq!(i["sof"], "SOF2");
    assert_eq!(i["exif"], Value::Null);
    assert_eq!(i["jfif"], Value::Null);
    assert_eq!(i["aspectRatio"], "3:1");
    let i = f.inspect(
        "damaged.jpg",
        &jpeg(&[exif(&tiff(true, 6)[..30]), sof(0xc0, 8, 8, 3), sos()]),
    );
    assert_eq!(i["ok"], true);
    assert!(i["warnings"][0].as_str().unwrap().contains("EXIF rusak"));
    let i = f.inspect(
        "orientation.jpg",
        &jpeg(&[exif(&tiff(true, 9)), sof(0xc0, 8, 8, 3), sos()]),
    );
    assert_eq!(i["exif"]["orientation"], Value::Null);
    assert!(i["warnings"][0].as_str().unwrap().contains("orientasi tidak valid: 9"));
    let mut data = tiff(true, 6);
    data[92..101].copy_from_slice(b"A\x1b\n\x7f Corp");
    let bytes = jpeg(&[exif(&data), sof(0xc0, 1, 1, 3), sos()]);
    assert_eq!(f.inspect("controls.jpg", &bytes)["exif"]["make"], "A��� Corp");
    let r = dt(&["image", &f.put("controls.jpg", &bytes)]);
    assert!(r.out.contains("A��� Corp"));
    assert!(!r.out.contains('\x1b'));
}
#[test]
fn jpeg_corruption_and_read_limit() {
    let f = Fixtures::new();
    let jfif = segment(0xe0, b"JFIF\0\x01\x02\x01\0H\0H\0\0");
    let good = jpeg(&[jfif.clone(), sof(0xc0, 640, 480, 3), sos()]);
    let at = 2 + jfif.len();
    for (name, data, msg) in [
        ("soi", good[..3].to_vec(), "terpotong"),
        ("before", good[..at].to_vec(), "SOF tidak ditemukan"),
        ("inside", good[..at + 6].to_vec(), "terpotong"),
        (
            "after",
            good[..at + sof(0xc0, 1, 1, 3).len()].to_vec(),
            "tidak ada SOS/EOI",
        ),
        ("eoi", cat(&[&[255, 216], &jfif, &[255, 217]]), "SOF tidak ditemukan"),
        (
            "marker",
            cat(&[&[255, 216], &jfif, &[17, 34]]),
            "marker JPEG tidak valid",
        ),
        ("length", vec![255, 216, 255, 224, 0, 1], "panjang segmen"),
        ("zero", jpeg(&[sof(0xc0, 0, 5, 3), sos()]), "ukuran JPEG tidak valid"),
    ] {
        f.bad(name, &data, msg);
    }
    let limit = 4 * 1024 * 1024;
    let mut data = vec![255; limit + 10];
    let prefix = cat(&[&[255, 216], &sof(0xc0, 3, 2, 3)]);
    data[..prefix.len()].copy_from_slice(&prefix);
    data[limit - 3..limit + 3].copy_from_slice(&[255, 225, 0, 2, 255, 217]);
    let i = f.inspect("limit.jpg", &data);
    assert_eq!(i["ok"], true);
    assert_eq!(i["truncated"], true);
    assert_eq!(i["width"], 3);
    assert!(i["warnings"][0].as_str().unwrap().contains("4 MiB pertama"));
}
#[test]
fn gif_metadata_and_corruption() {
    let f = Fixtures::new();
    let data = cat(&[&gif_header("89a", 3, 2, 4), &netscape(5), &frame(), &frame(), &[59]]);
    let i = f.inspect("anim.gif", &data);
    for (k, v) in [
        ("ok", json!(true)),
        ("format", json!("GIF")),
        ("mime", json!("image/gif")),
        ("version", json!("89a")),
        ("width", json!(3)),
        ("height", json!(2)),
        ("aspectRatio", json!("3:2")),
        ("colorTableSize", json!(4)),
        ("frames", json!(2)),
        ("framesAtLeast", json!(false)),
        ("animated", json!(true)),
        ("loopCount", json!(5)),
        ("truncated", json!(false)),
    ] {
        assert_eq!(i[k], v, "{k}");
    }
    let local = cat(&[&[44, 0, 0, 0, 0, 1, 0, 1, 0, 128], &[0; 6], &[2, 2, 68, 1, 0]]);
    let i = f.inspect("still.gif", &cat(&[&gif_header("87a", 1, 1, 2), &local, &[59]]));
    assert_eq!(i["version"], "87a");
    assert_eq!(i["frames"], 1);
    assert_eq!(i["animated"], false);
    assert_eq!(i["loopCount"], Value::Null);
    assert_eq!(
        f.inspect(
            "infinite.gif",
            &cat(&[&gif_header("89a", 1, 1, 2), &netscape(0), &frame(), &[59]])
        )["loopCount"],
        0
    );
    let good = cat(&[&gif_header("89a", 3, 2, 2), &frame(), &frame(), &[59]]);
    for (name, end) in [
        ("header", 8),
        ("table", 15),
        ("frame", good.len() - 6),
        ("trailer", good.len() - 1),
    ] {
        f.bad(name, &good[..end], "terpotong");
    }
    f.bad(
        "block",
        &cat(&[&gif_header("89a", 1, 1, 2), &[153, 59]]),
        "blok GIF tidak valid (0x99)",
    );
    f.bad(
        "empty",
        &cat(&[&gif_header("89a", 1, 1, 2), &[59]]),
        "tidak berisi frame",
    );
    f.bad(
        "zero",
        &cat(&[&gif_header("89a", 0, 1, 2), &frame(), &[59]]),
        "ukuran GIF tidak valid",
    );
}
#[test]
fn webp_variants_and_animation() {
    let f = Fixtures::new();
    let i = f.inspect("ll.webp", &webp(&[vp8l(100, 50, true)]));
    for (k, v) in [
        ("format", json!("WebP")),
        ("mime", json!("image/webp")),
        ("variant", json!("VP8L")),
        ("width", json!(100)),
        ("height", json!(50)),
        ("aspectRatio", json!("2:1")),
        ("alpha", json!(true)),
        ("animated", json!(false)),
        ("frames", json!(1)),
    ] {
        assert_eq!(i[k], v, "{k}");
    }
    assert_eq!(f.inspect("ll2.webp", &webp(&[vp8l(1, 1, false)]))["alpha"], false);
    let lossy = riff_chunk(
        "VP8 ",
        &cat(&[
            &[16, 0, 0, 157, 1, 42],
            &320u16.to_le_bytes(),
            &240u16.to_le_bytes(),
            &[0; 4],
        ]),
    );
    let i = f.inspect("lossy.webp", &webp(&[lossy]));
    assert_eq!(i["variant"], "VP8");
    assert_eq!(i["width"], 320);
    assert_eq!(i["height"], 240);
    assert_eq!(i["aspectRatio"], "4:3");
    assert_eq!(i["alpha"], false);
    let anim = riff_chunk("ANIM", &[0, 0, 0, 0, 3, 0]);
    let frame = riff_chunk("ANMF", &[0; 17]);
    let i = f.inspect(
        "anim.webp",
        &webp(&[vp8x(0x3a, 8388608, 3), anim, frame.clone(), frame.clone(), frame]),
    );
    for (k, v) in [
        ("variant", json!("VP8X")),
        ("width", json!(8388608)),
        ("height", json!(3)),
        ("animated", json!(true)),
        ("alpha", json!(true)),
        ("icc", json!(true)),
        ("exif", json!(true)),
        ("xmp", json!(false)),
        ("frames", json!(3)),
        ("framesAtLeast", json!(false)),
        ("loopCount", json!(3)),
    ] {
        assert_eq!(i[k], v, "{k}");
    }
    let i = f.inspect("x.webp", &webp(&[vp8x(4, 10, 10), vp8l(10, 10, false)]));
    assert_eq!(i["animated"], false);
    assert_eq!(i["xmp"], true);
    assert_eq!(i["loopCount"], Value::Null);
}
#[test]
fn webp_limits_and_corruption() {
    let f = Fixtures::new();
    let data = webp(&[
        vp8x(0x22, 10, 10),
        riff_chunk("ICCP", &vec![0; 5 * 1024 * 1024]),
        riff_chunk("ANIM", &[0; 6]),
        riff_chunk("ANMF", &[0; 17]),
    ]);
    let i = f.inspect("large.webp", &data);
    assert_eq!(i["ok"], true);
    assert_eq!(i["animated"], true);
    assert_eq!(i["frames"], 0);
    assert_eq!(i["framesAtLeast"], true);
    assert_eq!(i["truncated"], true);
    assert!(i["warnings"][0].as_str().unwrap().contains("4 MiB pertama"));
    let r = dt(&["image", &f.put("large.webp", &data)]);
    assert!(r.out.contains("minimal 0"));
    assert!(r.out.contains("peringatan:"));
    for (ty, flag) in [("ICCP", 32), ("EXIF", 8), ("XMP ", 4)] {
        let i = f.inspect(
            "still.webp",
            &webp(&[
                vp8x(flag, 10, 10),
                riff_chunk(ty, &vec![0; 5 * 1024 * 1024]),
                vp8l(10, 10, false),
            ]),
        );
        assert_eq!(i["ok"], true);
        assert_eq!(i["frames"], 1);
        assert_eq!(i["framesAtLeast"], false);
        assert_eq!(i["truncated"], false);
        assert_eq!(i["warnings"], json!([]));
    }
    let good = webp(&[vp8l(100, 50, false)]);
    let mut past = good.clone();
    past[16..20].copy_from_slice(&1000u32.to_le_bytes());
    for (n, data, msg) in [
        ("header", good[..12].to_vec(), "terpotong"),
        ("cut", good[..22].to_vec(), "terpotong (RIFF"),
        (
            "unknown",
            webp(&[riff_chunk("JUNK", &[0; 4])]),
            "chunk pertama WebP tidak dikenal: JUNK",
        ),
        (
            "sig",
            webp(&[riff_chunk("VP8L", &[1, 0, 0, 0, 0, 0])]),
            "signature VP8L",
        ),
        ("short", webp(&[riff_chunk("VP8 ", &[0; 4])]), "terlalu pendek"),
        ("no-frames", webp(&[vp8x(2, 5, 5)]), "tidak berisi frame"),
        ("past", past, "melewati akhir RIFF"),
    ] {
        f.bad(n, &data, msg);
    }
}
#[test]
fn cli_files_errors_and_usage() {
    let f = Fixtures::new();
    let gif = cat(&[&gif_header("89a", 1, 1, 2), &frame(), &[59]]);
    assert_eq!(f.inspect("actually.gif.png", &gif)["format"], "GIF");
    let a = f.put("a.png", &png(&[ihdr(4, 4, 8, 6, 0), idat(), chunk("IEND", &[])]));
    let b = f.put("b.bin", b"zzz");
    let c = f.put("c.gif", &gif);
    let r = dt(&["image", &b]);
    assert_eq!(r.code, 1);
    assert_eq!(r.out, "");
    assert_eq!(r.err.trim(), format!("format gambar tidak dikenal: {b}"));
    assert_eq!(dt(&["image", &f.put("empty", &[])]).code, 1);
    let missing = f.0.path().join("missing");
    let missing = missing.to_str().unwrap();
    assert_eq!(
        dt(&["image", missing]).err.trim(),
        format!("{missing}: file tidak ditemukan")
    );
    assert_eq!(dt(&["image", f.0.path().to_str().unwrap()]).code, 1);
    let r = dt(&["image", &a, &b, &c]);
    assert_eq!(r.code, 1);
    assert!(r.out.contains(&format!("{a}:\n")));
    assert!(r.out.contains(&format!("{c}:\n")));
    assert!(!r.out.contains(&b));
    assert_eq!(r.out.split("\n\n").count(), 2);
    assert_eq!(r.err.trim(), format!("format gambar tidak dikenal: {b}"));
    let r = dt(&["image", "--json", &a, &b, &c]);
    assert_eq!(r.code, 1);
    let j: Value = serde_json::from_str(&r.out).unwrap();
    for (i, path, ok) in [(0, &a, true), (1, &b, false), (2, &c, true)] {
        assert_eq!(j["files"][i]["path"], *path);
        assert_eq!(j["files"][i]["ok"], ok);
    }
    assert_eq!(j["files"][1]["error"], format!("format gambar tidak dikenal: {b}"));
    assert!(j["files"][1].get("format").is_none());
    assert_eq!(j["files"][0]["size"], std::fs::metadata(&a).unwrap().len());
    assert_eq!(r.err.trim(), format!("format gambar tidak dikenal: {b}"));
    assert_eq!(dt(&["image"]).code, 2);
    assert_eq!(dt(&["image", "--bogus"]).code, 2);
}
#[cfg(unix)]
#[test]
fn fifo_is_rejected_before_opening() {
    let f = Fixtures::new();
    let path = f.0.path().join("fifo");
    let result = std::process::Command::new("mkfifo").arg(&path).status();
    if result.as_ref().is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return;
    }
    assert!(result.unwrap().success());
    let r = dt(&["image", "--json", path.to_str().unwrap()]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("bukan file biasa"));
    let j: Value = serde_json::from_str(&r.out).unwrap();
    assert_eq!(j["files"][0]["ok"], false);
}

#[test]
fn fixed_legacy_metadata_vectors() {
    let f = Fixtures::new();
    // Metadata captured from legacy's parsers; these bytes are intentionally fixed.
    let vectors = [
        (
            "png",
            "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000049454e44ae426082",
            json!({"format":"PNG","mime":"image/png","width":1,"height":1,"aspectRatio":"1:1","bitDepth":8,"colorType":"truecolor+alpha","interlaced":false,"alpha":true,"dpi":null,"textCount":0,"textKeys":[],"truncated":false,"warnings":[]}),
        ),
        (
            "jpg",
            "ffd8ffc0000b080002000301000000ffda",
            json!({"format":"JPEG","mime":"image/jpeg","width":3,"height":2,"aspectRatio":"3:2","precision":8,"components":1,"sof":"SOF0","progressive":false,"jfif":null,"exif":null,"truncated":false,"warnings":[]}),
        ),
        (
            "gif",
            "474946383961010001008000000000000000002c00000000010001000002024401003b",
            json!({"format":"GIF","mime":"image/gif","width":1,"height":1,"aspectRatio":"1:1","version":"89a","colorTableSize":2,"frames":1,"framesAtLeast":false,"animated":false,"loopCount":null,"truncated":false,"warnings":[]}),
        ),
        (
            "webp",
            "5249464614000000574542505650384c070000002f63800c10000000",
            json!({"format":"WebP","mime":"image/webp","width":100,"height":51,"aspectRatio":"100:51","variant":"VP8L","animated":false,"alpha":true,"icc":false,"exif":false,"xmp":false,"frames":1,"framesAtLeast":false,"loopCount":null,"truncated":false,"warnings":[]}),
        ),
    ];
    for (name, raw, expected) in vectors {
        let data = hex::decode(raw).unwrap();
        let mut actual = f.inspect(name, &data);
        assert_eq!(actual["ok"], true);
        assert_eq!(actual["size"], data.len());
        let object = actual.as_object_mut().unwrap();
        object.remove("path");
        object.remove("ok");
        object.remove("size");
        assert_eq!(actual, expected, "{name}");
    }
}
