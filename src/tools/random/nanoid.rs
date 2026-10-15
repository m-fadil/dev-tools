use crate::errors::{Result, usage};
use crate::random;

pub const DEFAULT_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";

/// 2-94 distinct printable ASCII characters (no whitespace).
pub fn validate_alphabet(alphabet: &str) -> Result<&[u8]> {
    let bytes = alphabet.as_bytes();
    if !(2..=94).contains(&bytes.len()) {
        return Err(usage("Alphabet harus 2-94 karakter."));
    }
    let mut seen = [false; 128];
    for &b in bytes {
        if !b.is_ascii_graphic() {
            return Err(usage(
                "Alphabet hanya boleh berisi karakter ASCII yang dapat dicetak, tanpa spasi.",
            ));
        }
        if std::mem::replace(&mut seen[b as usize], true) {
            return Err(usage(format!("Alphabet berisi karakter ganda: {}", b as char)));
        }
    }
    Ok(bytes)
}

/// Unbiased NanoID: mask random bytes to the next power of two minus one and reject indexes outside the alphabet.
pub fn nanoid(alphabet: &[u8], length: usize) -> Result<String> {
    // len is 2..=94, so the next power of two is <= 128 and the mask fits a byte.
    let mask = (alphabet.len().next_power_of_two() - 1) as u8;
    let mut out = String::with_capacity(length);
    while out.len() < length {
        // Acceptance is > 50%; over-request so one round usually suffices.
        let want = (length - out.len()) * 2 + 8;
        for b in random::bytes(want)? {
            if let Some(&c) = alphabet.get((b & mask) as usize) {
                out.push(c as char);
                if out.len() == length {
                    break;
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_rules() {
        assert!(validate_alphabet("ab").is_ok());
        assert!(validate_alphabet("a").is_err());
        assert!(validate_alphabet("aa").is_err());
        assert!(validate_alphabet("a b").is_err());
        assert!(validate_alphabet("aé").is_err());
    }

    #[test]
    fn only_alphabet_chars() {
        for a in ["ab", "abc", DEFAULT_ALPHABET] {
            let s = nanoid(a.as_bytes(), 500).unwrap();
            assert_eq!(s.len(), 500);
            assert!(s.chars().all(|c| a.contains(c)));
        }
    }
}
