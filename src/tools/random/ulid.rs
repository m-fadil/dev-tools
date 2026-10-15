use crate::errors::Result;
use crate::random;

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const RANDOM_MASK: u128 = (1 << 80) - 1;

/// ULID: 48-bit unix ms + 80 random bits, Crockford base32. Output is strictly increasing within a generator:
/// in the same millisecond (or on a backwards clock) the random part is incremented; on 80-bit overflow the
/// timestamp advances by 1 ms with a fresh random part.
pub struct UlidGenerator<N: FnMut() -> i64> {
    now: N,
    last_ms: i64,
    rand: u128,
}

fn fresh() -> Result<u128> {
    let b = random::bytes(10)?;
    Ok(b.iter().fold(0u128, |acc, &x| (acc << 8) | u128::from(x)))
}

impl<N: FnMut() -> i64> UlidGenerator<N> {
    pub fn new(now: N) -> Self {
        UlidGenerator {
            now,
            last_ms: -1,
            rand: 0,
        }
    }

    pub fn next(&mut self, lowercase: bool) -> Result<String> {
        let ms = (self.now)();
        if ms > self.last_ms {
            self.last_ms = ms;
            self.rand = fresh()?;
        } else if self.rand == RANDOM_MASK {
            self.last_ms += 1;
            self.rand = fresh()?;
        } else {
            self.rand += 1;
        }
        let value = (u128::from(self.last_ms as u64 & 0xffff_ffff_ffff) << 80) | self.rand;
        let mut out = String::with_capacity(26);
        // 26 chars * 5 bits = 130 bits; the top 2 bits are zero.
        for i in (0..26).rev() {
            let c = CROCKFORD[((value >> (i * 5)) & 31) as usize];
            out.push(if lowercase { c.to_ascii_lowercase() } else { c } as char);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increments_in_same_ms_and_rolls_over() {
        let mut g = UlidGenerator::new(|| 1000);
        let a = g.next(false).unwrap();
        let b = g.next(false).unwrap();
        assert!(a < b);
        g.rand = RANDOM_MASK;
        let c = g.next(false).unwrap();
        assert!(b < c);
        assert_eq!(g.last_ms, 1001);
    }

    #[test]
    fn backwards_clock_stays_increasing() {
        let mut t = [5000, 4000].into_iter();
        let mut g = UlidGenerator::new(move || t.next().unwrap());
        let a = g.next(false).unwrap();
        let b = g.next(false).unwrap();
        assert!(a < b);
    }
}
