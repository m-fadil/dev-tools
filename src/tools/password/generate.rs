use crate::errors::{Result, usage};
use crate::random;

pub const ONLY_SETS: &[(&str, OnlySet)] = &[
    ("lower", OnlySet::Lower),
    ("upper", OnlySet::Upper),
    ("digits", OnlySet::Digits),
    ("alnum", OnlySet::Alnum),
    ("hex", OnlySet::Hex),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OnlySet {
    Lower,
    Upper,
    Digits,
    Alnum,
    Hex,
}

pub const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?";
const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const AMBIGUOUS: &str = "0O1lI|";

/// Uniform integer in [0, max).
pub type RandomInt<'a> = dyn FnMut(usize) -> Result<usize> + 'a;

/// Unbiased random integers from the OS CSPRNG (rejection sampling), buffered to limit syscalls.
pub struct CryptoInts {
    buf: Vec<u8>,
    pos: usize,
}

impl CryptoInts {
    pub fn new() -> Self {
        CryptoInts {
            buf: Vec::new(),
            pos: 0,
        }
    }

    fn next_u32(&mut self) -> Result<u32> {
        if self.pos + 4 > self.buf.len() {
            self.buf = random::bytes(4096)?;
            self.pos = 0;
        }
        let b = &self.buf[self.pos..self.pos + 4];
        self.pos += 4;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Uniform integer in [0, max); `max` must be in 1..=2^32.
    pub fn below(&mut self, max: usize) -> Result<usize> {
        let max = max as u64;
        let range = 1u64 << 32;
        let limit = range - range % max;
        loop {
            let v = u64::from(self.next_u32()?);
            if v < limit {
                return Ok((v % max) as usize);
            }
        }
    }
}

pub struct PasswordOptions {
    pub symbols: bool,
    pub only: Option<OnlySet>,
    pub unambiguous: bool,
}

/// Character classes of the active set; one character of each is guaranteed in every password.
pub fn character_classes(o: &PasswordOptions) -> Result<Vec<Vec<char>>> {
    if o.only.is_some() && o.symbols {
        return Err(usage("--only tidak bisa dipakai bersama --symbols."));
    }
    let hex = format!("{DIGITS}abcdef");
    let mut classes: Vec<String> = match o.only {
        Some(OnlySet::Lower) => vec![LOWER.into()],
        Some(OnlySet::Upper) => vec![UPPER.into()],
        Some(OnlySet::Digits) => vec![DIGITS.into()],
        // A single class: hex has no letter/digit guarantee.
        Some(OnlySet::Hex) => vec![hex],
        _ => {
            let mut c = vec![LOWER.to_string(), UPPER.into(), DIGITS.into()];
            if o.symbols {
                c.push(SYMBOLS.into());
            }
            c
        }
    };
    if o.unambiguous {
        for set in &mut classes {
            *set = set.chars().filter(|c| !AMBIGUOUS.contains(*c)).collect();
        }
    }
    Ok(classes.into_iter().map(|s| s.chars().collect()).collect())
}

/// Upper estimate in bits: length * log2(alphabet size); ignores the one-per-class guarantee.
pub fn entropy_bits(classes: &[Vec<char>], length: usize) -> f64 {
    let size: usize = classes.iter().map(Vec::len).sum();
    length as f64 * (size as f64).log2()
}

/// One required character per class, the rest from the whole alphabet, then a Fisher-Yates shuffle
/// so the required characters do not sit at fixed positions. Every pick is an unbiased random integer.
pub fn generate_password(classes: &[Vec<char>], length: usize, random_int: &mut RandomInt) -> Result<String> {
    if length < classes.len() {
        return Err(usage(format!(
            "--length minimal {} agar tiap jenis karakter terwakili.",
            classes.len()
        )));
    }
    let alphabet: Vec<char> = classes.iter().flatten().copied().collect();
    let mut chars: Vec<char> = Vec::with_capacity(length);
    for set in classes {
        chars.push(set[random_int(set.len())?]);
    }
    while chars.len() < length {
        chars.push(alphabet[random_int(alphabet.len())?]);
    }
    for i in (1..chars.len()).rev() {
        let j = random_int(i + 1)?;
        chars.swap(i, j);
    }
    Ok(chars.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cls(sets: &[&str]) -> Vec<Vec<char>> {
        sets.iter().map(|s| s.chars().collect()).collect()
    }

    #[test]
    fn placement_and_shuffle_are_deterministic_with_injected_random_int() {
        // random_int always 0: required 'a','A', fill 'a'; the shuffle then yields 'Aaa'
        let mut zero = |_max: usize| Ok(0);
        assert_eq!(
            generate_password(&cls(&["ab", "AB"]), 3, &mut zero).ok().as_deref(),
            Some("Aaa")
        );
        let o = PasswordOptions {
            symbols: true,
            only: None,
            unambiguous: false,
        };
        assert_eq!(character_classes(&o).map(|c| c.len()).ok(), Some(4));
    }

    #[test]
    fn random_int_is_only_asked_for_in_range_values() {
        let mut seen = Vec::new();
        let mut spy = |max: usize| {
            assert!(max >= 1);
            seen.push(max);
            Ok(max - 1)
        };
        let p = generate_password(&cls(&["xy", "12"]), 6, &mut spy).unwrap_or_default();
        assert_eq!(p.len(), 6);
        assert!(seen.iter().all(|m| *m <= 6));
    }

    #[test]
    fn length_below_the_number_of_classes_is_a_usage_error() {
        let mut zero = |_max: usize| Ok(0);
        let r = generate_password(&cls(&["a", "B", "1", "!"]), 3, &mut zero);
        assert!(matches!(r, Err(crate::errors::CliError::Usage(_))));
    }

    #[test]
    fn rejection_sampling_discards_the_incomplete_tail() {
        // For an alphabet of 62, 2^32 leaves a tail of four values.
        // All four must be rejected rather than biased toward indices 0..3.
        let words = [u32::MAX - 3, u32::MAX - 2, u32::MAX - 1, u32::MAX, 61, 62];
        let mut c = CryptoInts {
            buf: words.into_iter().flat_map(u32::to_le_bytes).collect(),
            pos: 0,
        };
        assert_eq!(c.below(62).ok(), Some(61));
        assert_eq!(c.pos, 20);
        assert_eq!(c.below(62).ok(), Some(0));
        assert_eq!(c.pos, 24);
    }

    #[test]
    fn crypto_ints_stay_below_max() {
        let mut c = CryptoInts::new();
        for max in [1usize, 2, 3, 62, 1000] {
            for _ in 0..200 {
                assert!(c.below(max).unwrap_or(usize::MAX) < max);
            }
        }
    }
}
