//! Minimal arbitrary-size unsigned integer (little-endian u32 limbs), shared by number, bytes and math.

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigUint {
    limbs: Vec<u32>,
}

impl BigUint {
    pub fn zero() -> Self {
        BigUint { limbs: Vec::new() }
    }

    pub fn from_u64(v: u64) -> Self {
        let mut n = BigUint {
            limbs: vec![v as u32, (v >> 32) as u32],
        };
        n.trim();
        n
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    pub fn mul_u64(&mut self, m: u64) {
        let mut carry: u128 = 0;
        for limb in &mut self.limbs {
            let cur = u128::from(*limb) * u128::from(m) + carry;
            *limb = cur as u32;
            carry = cur >> 32;
        }
        while carry > 0 {
            self.limbs.push(carry as u32);
            carry >>= 32;
        }
        self.trim();
    }

    pub fn add_u64(&mut self, a: u64) {
        let mut carry = u128::from(a);
        for limb in &mut self.limbs {
            if carry == 0 {
                return;
            }
            let cur = u128::from(*limb) + carry;
            *limb = cur as u32;
            carry = cur >> 32;
        }
        while carry > 0 {
            self.limbs.push(carry as u32);
            carry >>= 32;
        }
    }

    /// Divides in place by `d` (non-zero) and returns the remainder.
    pub fn divmod_u64(&mut self, d: u64) -> u64 {
        let mut rem: u128 = 0;
        for limb in self.limbs.iter_mut().rev() {
            let cur = (rem << 32) | u128::from(*limb);
            *limb = (cur / u128::from(d)) as u32;
            rem = cur % u128::from(d);
        }
        self.trim();
        rem as u64
    }

    pub fn shl(&mut self, bits: usize) {
        if self.is_zero() {
            return;
        }
        let (words, rem) = (bits / 32, bits % 32);
        if rem > 0 {
            let mut carry = 0u32;
            for limb in &mut self.limbs {
                let next = *limb >> (32 - rem);
                *limb = (*limb << rem) | carry;
                carry = next;
            }
            if carry > 0 {
                self.limbs.push(carry);
            }
        }
        if words > 0 {
            self.limbs.splice(0..0, std::iter::repeat_n(0, words));
        }
    }

    pub fn to_u128(&self) -> Option<u128> {
        if self.limbs.len() > 4 {
            return None;
        }
        Some(
            self.limbs
                .iter()
                .rev()
                .fold(0u128, |acc, &l| (acc << 32) | u128::from(l)),
        )
    }

    pub fn cmp_u64(&self, v: u64) -> Ordering {
        match self.to_u128() {
            Some(x) => x.cmp(&u128::from(v)),
            None => Ordering::Greater,
        }
    }

    /// Parses digits of `radix` (2..=16); `None` on an invalid digit or empty input.
    pub fn parse_radix(digits: &str, radix: u32) -> Option<Self> {
        if digits.is_empty() {
            return None;
        }
        let chunk = chunk_digits(radix);
        let bytes = digits.as_bytes();
        let mut n = BigUint::zero();
        for part in bytes.chunks(chunk) {
            let mut value: u64 = 0;
            for &b in part {
                value = value * u64::from(radix) + u64::from((b as char).to_digit(radix)?);
            }
            n.mul_u64(u64::from(radix).pow(part.len() as u32));
            n.add_u64(value);
        }
        Some(n)
    }

    /// Lowercase digits in `radix` (2..=16), no prefix.
    pub fn to_radix_string(&self, radix: u32) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let chunk = chunk_digits(radix);
        let divisor = u64::from(radix).pow(chunk as u32);
        let mut work = self.clone();
        let mut parts: Vec<String> = Vec::new();
        while !work.is_zero() {
            let mut rem = work.divmod_u64(divisor);
            let mut part = Vec::with_capacity(chunk);
            for _ in 0..chunk {
                part.push(char::from_digit((rem % u64::from(radix)) as u32, radix).unwrap_or('0'));
                rem /= u64::from(radix);
            }
            part.reverse();
            parts.push(part.into_iter().collect());
        }
        let mut out = String::new();
        for (i, part) in parts.iter().rev().enumerate() {
            if i == 0 {
                out.push_str(part.trim_start_matches('0'));
            } else {
                out.push_str(part);
            }
        }
        out
    }
}

/// Largest digit count whose radix power still fits in a u32.
fn chunk_digits(radix: u32) -> usize {
    let mut n = 0;
    let mut p: u64 = 1;
    while p * u64::from(radix) <= u64::from(u32::MAX) {
        p *= u64::from(radix);
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radix_round_trip() {
        let n = BigUint::parse_radix("123456789012345678901234567890", 10).unwrap();
        assert_eq!(n.to_radix_string(16), "18ee90ff6c373e0ee4e3f0ad2");
        assert_eq!(n.to_radix_string(10), "123456789012345678901234567890");
        let m = BigUint::parse_radix("18ee90ff6c373e0ee4e3f0ad2", 16).unwrap();
        assert_eq!(m, n);
        assert_eq!(BigUint::zero().to_radix_string(2), "0");
        assert_eq!(BigUint::parse_radix("000", 10).unwrap().to_radix_string(10), "0");
        assert_eq!(BigUint::from_u64(255).to_radix_string(2), "11111111");
        assert!(BigUint::parse_radix("12", 2).is_none());
    }

    #[test]
    fn shifts_and_division() {
        let mut n = BigUint::from_u64(1);
        n.shl(70);
        assert_eq!(n.to_radix_string(10), "1180591620717411303424");
        let r = n.divmod_u64(1000);
        assert_eq!(r, 424);
        assert_eq!(n.to_radix_string(10), "1180591620717411303");
        assert_eq!(n.to_u128(), Some(1180591620717411303));
        assert_eq!(BigUint::from_u64(5).cmp_u64(5), Ordering::Equal);
    }
}
