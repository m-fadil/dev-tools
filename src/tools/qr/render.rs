use std::fmt::Write;

#[derive(Clone, Copy)]
pub enum Format {
    Unicode,
    Ascii,
    Svg,
}

/// Module at (x, y) in margin-inclusive coordinates; everything outside the symbol is light.
fn is_dark(dark: &[bool], width: usize, margin: usize, x: usize, y: usize) -> bool {
    let (Some(x), Some(y)) = (x.checked_sub(margin), y.checked_sub(margin)) else {
        return false;
    };
    x < width && y < width && dark[y * width + x]
}

/// Default (dark terminal): light modules are drawn filled. `invert`: dark modules are filled.
pub fn unicode(dark: &[bool], width: usize, margin: usize, invert: bool) -> String {
    let side = width + 2 * margin;
    let mut out = String::with_capacity((side / 2 + 1) * (side * 3 + 1));
    for y in (0..side).step_by(2) {
        for x in 0..side {
            let top = is_dark(dark, width, margin, x, y) == invert;
            let bottom = is_dark(dark, width, margin, x, y + 1) == invert;
            out.push(match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        out.push('\n');
    }
    out
}

/// Two characters per module: `##` for filled, two spaces otherwise (same polarity as `unicode`).
pub fn ascii(dark: &[bool], width: usize, margin: usize, invert: bool) -> String {
    let side = width + 2 * margin;
    let mut out = String::with_capacity(side * (side * 2 + 1));
    for y in 0..side {
        for x in 0..side {
            out.push_str(if is_dark(dark, width, margin, x, y) == invert {
                "##"
            } else {
                "  "
            });
        }
        out.push('\n');
    }
    out
}

/// Black modules on a white background, one `<rect>` per dark module, 1 unit = 1 module.
pub fn svg(dark: &[bool], width: usize, margin: usize) -> String {
    let side = width + 2 * margin;
    let mut out = String::new();
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {side} {side}\" width=\"{px}\" height=\"{px}\" shape-rendering=\"crispEdges\">\n<rect width=\"{side}\" height=\"{side}\" fill=\"#fff\"/>\n<g fill=\"#000\">\n",
        px = side * 8
    );
    for (i, _) in dark.iter().enumerate().filter(|(_, d)| **d) {
        let _ = writeln!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\"/>",
            i % width + margin,
            i / width + margin
        );
    }
    out.push_str("</g>\n</svg>\n");
    out
}
