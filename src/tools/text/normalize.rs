//! Canonical Unicode normalization (NFD / NFC), the same operations as JS `String.prototype.normalize`.
//! Private implementation (no extra crate); tables live in `norm_data.rs`.

use super::norm_data::{CCC, COMPOSE, DECOMP};

const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

fn ccc(c: char) -> u8 {
    let cp = c as u32;
    if cp < 0x300 {
        return 0;
    }
    match CCC.binary_search_by(|&(lo, hi, _)| {
        if cp < lo {
            std::cmp::Ordering::Greater
        } else if cp > hi {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }) {
        Ok(i) => CCC[i].2,
        Err(_) => 0,
    }
}

fn push_decomposed(c: char, out: &mut Vec<char>) {
    let cp = c as u32;
    if cp < 0xC0 {
        out.push(c);
        return;
    }
    if (S_BASE..S_BASE + S_COUNT).contains(&cp) {
        let s = cp - S_BASE;
        let parts = [L_BASE + s / N_COUNT, V_BASE + (s % N_COUNT) / T_COUNT];
        out.extend(parts.iter().filter_map(|&p| char::from_u32(p)));
        let t = T_BASE + s % T_COUNT;
        if t != T_BASE {
            out.extend(char::from_u32(t));
        }
        return;
    }
    match DECOMP.binary_search_by_key(&cp, |&(k, _)| k) {
        Ok(i) => out.extend(DECOMP[i].1.iter().filter_map(|&p| char::from_u32(p))),
        Err(_) => out.push(c),
    }
}

/// Canonical decomposition with canonical ordering of combining marks.
pub fn nfd(text: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        push_decomposed(c, &mut out);
    }
    let mut i = 0;
    while i < out.len() {
        if ccc(out[i]) == 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < out.len() && ccc(out[i]) != 0 {
            i += 1;
        }
        out[start..i].sort_by_key(|&c| ccc(c));
    }
    out
}

fn compose_pair(a: char, b: char) -> Option<char> {
    let (x, y) = (a as u32, b as u32);
    if (L_BASE..L_BASE + L_COUNT).contains(&x) && (V_BASE..V_BASE + V_COUNT).contains(&y) {
        return char::from_u32(S_BASE + ((x - L_BASE) * V_COUNT + (y - V_BASE)) * T_COUNT);
    }
    if (S_BASE..S_BASE + S_COUNT).contains(&x)
        && (x - S_BASE).is_multiple_of(T_COUNT)
        && (T_BASE + 1..T_BASE + T_COUNT).contains(&y)
    {
        return char::from_u32(x + (y - T_BASE));
    }
    COMPOSE
        .binary_search_by_key(&(x, y), |&(k, _)| k)
        .ok()
        .and_then(|i| char::from_u32(COMPOSE[i].1))
}

/// Canonical composition of already decomposed (and ordered) characters.
pub fn compose(chars: &[char]) -> String {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut starter: Option<usize> = None;
    let mut last_cc = 0u8;
    for &c in chars {
        let cc = ccc(c);
        if let Some(si) = starter {
            let blocked = out.len() - 1 > si && last_cc >= cc;
            if !blocked && let Some(composed) = compose_pair(out[si], c) {
                out[si] = composed;
                continue;
            }
        }
        if cc == 0 {
            starter = Some(out.len());
        }
        out.push(c);
        last_cc = cc;
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nfc(s: &str) -> String {
        compose(&nfd(s))
    }

    #[test]
    fn decomposes_and_recomposes() {
        assert_eq!(nfd("é").iter().collect::<String>(), "e\u{301}");
        assert_eq!(nfc("e\u{301}"), "é");
        assert_eq!(nfc("カ\u{3099}"), "ガ");
        assert_eq!(nfd("ガ").len(), 2);
        assert_eq!(nfc("한"), "한");
        assert_eq!(nfd("한").len(), 3);
        // composition-excluded character stays decomposed
        assert_eq!(nfc("\u{958}"), "\u{915}\u{93C}");
        // singleton decomposes to a letter
        assert_eq!(nfc("\u{212B}"), "\u{C5}");
    }

    #[test]
    fn reorders_marks_by_class() {
        assert_eq!(nfd("a\u{301}\u{323}").iter().collect::<String>(), "a\u{323}\u{301}");
        assert_eq!(nfc("a\u{301}\u{323}"), "\u{1EA1}\u{301}");
        assert_eq!(nfd("a\u{1acf}\u{1add}").iter().collect::<String>(), "a\u{1add}\u{1acf}");
        assert_eq!(
            nfd("a\u{1ae0}\u{10efa}").iter().collect::<String>(),
            "a\u{10efa}\u{1ae0}"
        );
    }
}
