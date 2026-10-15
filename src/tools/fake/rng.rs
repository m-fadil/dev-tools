use crate::errors::Result;
use crate::io::integer;
use crate::random;

/// mulberry32: small seeded PRNG returning floats in [0, 1).
pub struct Rng {
    a: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng { a: seed }
    }

    /// Next float in [0, 1).
    pub fn next(&mut self) -> f64 {
        self.a = self.a.wrapping_add(0x6d2b_79f5);
        let mut t = self.a;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    /// Uniform integer in [min, max]; callers must keep max - min below 2^32 (the rng has 32 bits).
    pub fn rand_int(&mut self, min: i64, max: i64) -> i64 {
        min + (self.next() * ((max - min + 1) as f64)).floor() as i64
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let i = (self.next() * items.len() as f64).floor() as usize;
        &items[i.min(items.len() - 1)]
    }
}

/// RNG from the --seed option (0-4294967295); without it the seed comes from the OS.
pub fn from_seed(seed: Option<&str>) -> Result<Rng> {
    let n = match seed {
        None => {
            let b = random::bytes(4)?;
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        }
        Some(_) => integer(seed, 0, 0, 4_294_967_295, "seed")? as u32,
    };
    Ok(Rng::new(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_matches_legacy() {
        // values produced by legacy mulberry32(1)
        let mut r = Rng::new(1);
        assert_eq!(r.next(), 0.6270739405881613);
        assert_eq!(r.next(), 0.002735721180215478);
        assert_eq!(r.next(), 0.5274470399599522);
        assert_eq!(Rng::new(7).next(), Rng::new(7).next());
    }

    #[test]
    fn wrapping_seed_extremes_match_legacy() {
        for (seed, expected) in [
            (0, [0.26642920868471265, 0.0003297457005828619, 0.2232720274478197]),
            (42, [0.6011037519201636, 0.44829055899754167, 0.8524657934904099]),
            (u32::MAX, [0.8964226141106337, 0.189478256739676, 0.7156526781618595]),
        ] {
            let mut rng = Rng::new(seed);
            for value in expected {
                assert_eq!(rng.next(), value);
            }
        }
    }

    #[test]
    fn rand_int_is_inclusive() {
        let mut r = Rng::new(3);
        for _ in 0..1000 {
            let n = r.rand_int(-2, 2);
            assert!((-2..=2).contains(&n));
        }
        assert_eq!(r.rand_int(7, 7), 7);
    }
}
