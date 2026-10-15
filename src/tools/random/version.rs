use crate::errors::Result;
use crate::random;

pub const UUID_VERSIONS: &[(&str, u8)] = &[("1", 1), ("4", 4), ("7", 7)];

/// Milliseconds between the Gregorian epoch (1582-10-15) and the Unix epoch.
const GREGORIAN_OFFSET_MS: i64 = 12_219_292_800_000;

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn format(bytes: &mut [u8; 16], version: u8) -> String {
    bytes[6] = (bytes[6] & 0x0f) | (version << 4);
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h = hex::encode(bytes);
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

fn random16() -> Result<[u8; 16]> {
    let b = random::bytes(16)?;
    let mut out = [0u8; 16];
    out.copy_from_slice(&b);
    Ok(out)
}

fn random_u16() -> Result<u16> {
    let b = random::bytes(2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

/// UUID v4: 122 random bits.
pub fn v4() -> Result<String> {
    Ok(format(&mut random16()?, 4))
}

/// UUID v7 (RFC 9562 5.7): 48-bit unix ms, then a 12-bit rand_a used as a counter (method 1, RFC 9562 6.2) and
/// 62 random bits. Output is strictly increasing; on counter overflow or a backwards clock the timestamp is bumped.
pub struct V7Generator<N: FnMut() -> i64> {
    now: N,
    last_ms: i64,
    counter: u16,
}

impl<N: FnMut() -> i64> V7Generator<N> {
    pub fn new(now: N) -> Self {
        V7Generator {
            now,
            last_ms: -1,
            counter: 0,
        }
    }

    /// Seed with the top bit clear so a same-millisecond burst has headroom before overflowing.
    fn seed() -> Result<u16> {
        Ok(random_u16()? & 0x7ff)
    }

    pub fn next(&mut self) -> Result<String> {
        let ms = (self.now)();
        if ms > self.last_ms {
            self.last_ms = ms;
            self.counter = Self::seed()?;
        } else {
            self.counter += 1;
            if self.counter > 0xfff {
                self.last_ms += 1;
                self.counter = Self::seed()?;
            }
        }
        let mut bytes = random16()?;
        bytes[..6].copy_from_slice(&(self.last_ms as u64).to_be_bytes()[2..]);
        bytes[6..8].copy_from_slice(&self.counter.to_be_bytes());
        Ok(format(&mut bytes, 7))
    }
}

/// UUID v1 (RFC 9562 5.1): 60-bit count of 100ns ticks since 1582-10-15, random 14-bit clock sequence, and a random
/// 48-bit node id with the multicast bit set (RFC 9562 6.10); no real MAC address is ever read. Timestamps are
/// strictly increasing within a generator, so values never repeat.
pub struct V1Generator<N: FnMut() -> i64> {
    now: N,
    node: [u8; 6],
    clock_seq: u16,
    last: i64,
}

impl<N: FnMut() -> i64> V1Generator<N> {
    pub fn new(now: N) -> Result<Self> {
        let b = random::bytes(6)?;
        let mut node = [0u8; 6];
        node.copy_from_slice(&b);
        node[0] |= 0x01;
        Ok(V1Generator {
            now,
            node,
            clock_seq: random_u16()? & 0x3fff,
            last: -1,
        })
    }

    pub fn next(&mut self) -> Result<String> {
        let mut ticks = ((self.now)() + GREGORIAN_OFFSET_MS) * 10_000;
        if ticks <= self.last {
            ticks = self.last + 1;
        }
        self.last = ticks;
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&((ticks & 0xffff_ffff) as u32).to_be_bytes());
        bytes[4..6].copy_from_slice(&(((ticks >> 32) & 0xffff) as u16).to_be_bytes());
        bytes[6..8].copy_from_slice(&(((ticks >> 48) & 0x0fff) as u16).to_be_bytes());
        bytes[8..10].copy_from_slice(&self.clock_seq.to_be_bytes());
        bytes[10..].copy_from_slice(&self.node);
        Ok(format(&mut bytes, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(u: &str) -> u128 {
        u128::from_str_radix(&u.replace('-', ""), 16).unwrap_or(0)
    }

    #[test]
    fn v7_stays_strictly_increasing_past_counter_overflow_with_a_frozen_clock() {
        let frozen = 1_700_000_000_000i64;
        let mut g = V7Generator::new(|| frozen);
        let mut previous = g.next().unwrap_or_default();
        assert_eq!((bits(&previous) >> 80) as i64, frozen);
        let mut overflows = 0;
        for _ in 1..10_000 {
            let current = g.next().unwrap_or_default();
            assert!(current > previous, "{previous} < {current}");
            let prev_ms = (bits(&previous) >> 80) as i64;
            let cur_ms = (bits(&current) >> 80) as i64;
            if cur_ms != prev_ms {
                assert_eq!(cur_ms, prev_ms + 1);
                assert_eq!((bits(&previous) >> 64) & 0xfff, 4095);
                overflows += 1;
            }
            previous = current;
        }
        assert!(overflows >= 2);
        assert!((bits(&previous) >> 80) as i64 > frozen);
    }

    #[test]
    fn v7_stays_strictly_increasing_with_a_decreasing_clock() {
        let mut clock = 1_700_000_000_000i64;
        let mut g = V7Generator::new(|| {
            let v = clock;
            clock -= 1;
            v
        });
        let mut previous = g.next().unwrap_or_default();
        for _ in 1..10_000 {
            let current = g.next().unwrap_or_default();
            assert!(current > previous, "{previous} < {current}");
            assert!(bits(&current) >> 80 >= bits(&previous) >> 80);
            previous = current;
        }
    }

    #[test]
    fn v1_advances_its_timestamp_with_frozen_and_decreasing_clocks() {
        for decrement in [0i64, 1] {
            let mut clock = 1_700_000_000_000i64;
            let Ok(mut g) = V1Generator::new(|| {
                let v = clock;
                clock -= decrement;
                v
            }) else {
                panic!("rng");
            };
            let mut previous_ticks = -1i128;
            let mut suffix = String::new();
            for i in 0..10_000 {
                let current = g.next().unwrap_or_default();
                let parts: Vec<&str> = current.split('-').collect();
                let ticks =
                    i128::from_str_radix(&format!("{}{}{}", &parts[2][1..], parts[1], parts[0]), 16).unwrap_or(-5);
                if i == 0 {
                    assert_eq!(ticks, (1_700_000_000_000i128 + 12_219_292_800_000) * 10_000);
                    suffix = current[19..].to_string();
                } else {
                    assert_eq!(ticks, previous_ticks + 1);
                    assert_eq!(&current[19..], suffix);
                }
                previous_ticks = ticks;
            }
        }
    }

    #[test]
    fn v4_layout() {
        let u = v4().unwrap_or_default();
        assert_eq!(u.len(), 36);
        assert_eq!(&u[14..15], "4");
        assert!("89ab".contains(&u[19..20]));
    }
}
