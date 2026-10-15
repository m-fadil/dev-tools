#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    V4,
    V6,
}

impl Version {
    pub fn number(self) -> u8 {
        match self {
            Version::V4 => 4,
            Version::V6 => 6,
        }
    }

    pub fn bits(self) -> u32 {
        match self {
            Version::V4 => 32,
            Version::V6 => 128,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Address {
    pub version: Version,
    pub value: u128,
}

/// Parse errors are plain messages; the caller turns them into usage errors.
type PResult<T> = std::result::Result<T, String>;

fn is_octet(s: &str) -> bool {
    // ^(0|[1-9]\d{0,2})$
    s == "0" || (matches!(s.len(), 1..=3) && s.as_bytes()[0] != b'0' && s.bytes().all(|b| b.is_ascii_digit()))
}

/// Strict dotted quad; leading zeros (010) are rejected as ambiguous (octal vs decimal).
pub fn parse_ipv4(text: &str) -> PResult<u128> {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() != 4 {
        return Err(format!("Alamat IPv4 tidak valid: {text}"));
    }
    let mut value: u128 = 0;
    for part in parts {
        if !is_octet(part) {
            let ambiguous = part.len() > 1 && part.starts_with('0') && part.bytes().all(|b| b.is_ascii_digit());
            let hint = if ambiguous { " (angka nol di depan ambigu)" } else { "" };
            return Err(format!("Oktet IPv4 tidak valid \"{part}\" pada {text}{hint}"));
        }
        let n: u32 = part.parse().unwrap_or(u32::MAX);
        if n > 255 {
            return Err(format!("Oktet IPv4 di luar 0-255 \"{part}\" pada {text}"));
        }
        value = (value << 8) | u128::from(n);
    }
    Ok(value)
}

fn is_hextet(s: &str) -> bool {
    (1..=4).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_groups(text: &str, whole: &str) -> PResult<Vec<u128>> {
    if text.is_empty() {
        return Ok(vec![]);
    }
    let mut groups = Vec::new();
    let parts: Vec<&str> = text.split(':').collect();
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 && part.contains('.') {
            let v4 = parse_ipv4(part)?;
            groups.push(v4 >> 16);
            groups.push(v4 & 0xffff);
        } else if is_hextet(part) {
            groups.push(u128::from_str_radix(part, 16).unwrap_or(0));
        } else {
            return Err(format!("Alamat IPv6 tidak valid: {whole}"));
        }
    }
    Ok(groups)
}

/// Strict RFC 4291 text form: one optional "::", 1-4 hex digits per group, optional dotted-quad tail, no zone id.
pub fn parse_ipv6(text: &str) -> PResult<u128> {
    let fail = || format!("Alamat IPv6 tidak valid: {text}");
    if !text.contains(':') || text.chars().any(|c| !(c.is_ascii_hexdigit() || c == ':' || c == '.')) {
        return Err(fail());
    }
    let halves: Vec<&str> = text.split("::").collect();
    if halves.len() > 2 {
        return Err(fail());
    }
    let groups: Vec<u128> = if halves.len() == 1 {
        let groups = parse_groups(text, text)?;
        if groups.len() != 8 {
            return Err(fail());
        }
        groups
    } else {
        let (head, tail) = (halves[0], halves[1]);
        // The dotted tail is only legal at the very end of the address.
        if head.contains('.') {
            return Err(fail());
        }
        let left = parse_groups(head, text)?;
        let right = parse_groups(tail, text)?;
        if left.len() + right.len() > 7 {
            return Err(fail());
        }
        let missing = 8 - left.len() - right.len();
        let mut g = left;
        g.extend(std::iter::repeat_n(0, missing));
        g.extend(right);
        g
    };
    Ok(groups.into_iter().fold(0u128, |acc, g| (acc << 16) | g))
}

pub fn parse_address(text: &str) -> PResult<Address> {
    if text.contains(':') {
        Ok(Address {
            version: Version::V6,
            value: parse_ipv6(text)?,
        })
    } else {
        Ok(Address {
            version: Version::V4,
            value: parse_ipv4(text)?,
        })
    }
}

pub fn format_ipv4(value: u128) -> String {
    [24, 16, 8, 0]
        .iter()
        .map(|shift| ((value >> shift) & 0xff).to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// RFC 5952: lowercase, longest zero run (>= 2 groups, first on tie) compressed; IPv4-mapped keeps a dotted tail.
pub fn format_ipv6(value: u128) -> String {
    if value >> 32 == 0xffff {
        return format!("::ffff:{}", format_ipv4(value & 0xffff_ffff));
    }
    let groups: Vec<u16> = (0..8).map(|i| ((value >> ((7 - i) * 16)) & 0xffff) as u16).collect();
    let (mut best_start, mut best_len) = (0usize, 0usize);
    let mut i = 0;
    while i < 8 {
        if groups[i] != 0 {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < 8 && groups[j] == 0 {
            j += 1;
        }
        if j - i > best_len {
            best_start = i;
            best_len = j - i;
        }
        i = j;
    }
    let hex = |g: &[u16]| g.iter().map(|n| format!("{n:x}")).collect::<Vec<_>>().join(":");
    if best_len < 2 {
        return hex(&groups);
    }
    format!(
        "{}::{}",
        hex(&groups[..best_start]),
        hex(&groups[best_start + best_len..])
    )
}

pub fn format_address(version: Version, value: u128) -> String {
    match version {
        Version::V4 => format_ipv4(value),
        Version::V6 => format_ipv6(value),
    }
}

/// All-ones value of `n` bits (n may be 0..=128).
fn ones(n: u32) -> u128 {
    if n >= 128 { u128::MAX } else { (1u128 << n) - 1 }
}

/// Network mask with `prefix` leading one bits.
pub fn mask_of(version: Version, prefix: u32) -> u128 {
    let bits = version.bits();
    ones(bits) ^ ones(bits - prefix)
}

/// Converts a contiguous dotted netmask to its prefix length.
pub fn prefix_from_netmask(text: &str) -> PResult<u32> {
    let mask = parse_ipv4(text)?;
    let inverted = mask ^ 0xffff_ffff;
    if inverted & (inverted + 1) != 0 {
        return Err(format!("Netmask tidak kontigu: {text}"));
    }
    Ok(32 - (128 - inverted.leading_zeros()))
}

#[derive(Debug, Clone, Copy)]
pub struct Network {
    pub version: Version,
    pub address: u128,
    pub prefix: u32,
    pub network: u128,
    pub last: u128,
}

pub fn make_network(version: Version, address: u128, prefix: u32) -> Network {
    let mask = mask_of(version, prefix);
    let network = address & mask;
    let last = network | (mask ^ ones(version.bits()));
    Network {
        version,
        address,
        prefix,
        network,
        last,
    }
}

fn parse_prefix(text: &str, version: Version) -> PResult<u32> {
    let n = if is_octet(text) { text.parse::<u32>().ok() } else { None };
    match n {
        Some(n) if n <= version.bits() => Ok(n),
        _ => Err(format!(
            "Prefix tidak valid \"{text}\": harus bilangan bulat 0-{} untuk IPv{}",
            version.bits(),
            version.number()
        )),
    }
}

/// Accepts ["ip/prefix"] or ["ip", "netmask"] (IPv4 only).
pub fn parse_network(args: &[String]) -> PResult<Network> {
    if args.is_empty() || args.len() > 2 {
        return Err("Beri satu argumen ip/prefix atau dua argumen ip netmask.".into());
    }
    if args.len() == 2 {
        let (ip, mask) = (&args[0], &args[1]);
        if ip.contains('/') || mask.contains('/') {
            return Err("Gunakan ip/prefix atau ip netmask, bukan campuran.".into());
        }
        let address = parse_address(ip)?;
        if address.version != Version::V4 {
            return Err("Bentuk \"ip netmask\" hanya untuk IPv4; gunakan ip/prefix.".into());
        }
        return Ok(make_network(Version::V4, address.value, prefix_from_netmask(mask)?));
    }
    let text = &args[0];
    let Some(slash) = text.find('/') else {
        return Err(format!("Prefix wajib: gunakan {text}/<prefix> atau {text} <netmask>"));
    };
    let address = parse_address(&text[..slash])?;
    Ok(make_network(
        address.version,
        address.value,
        parse_prefix(&text[slash + 1..], address.version)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv6_roundtrip() {
        assert_eq!(
            format_ipv6(parse_ipv6("2001:0:0:1:0:0:0:1").unwrap_or(0)),
            "2001:0:0:1::1"
        );
        assert_eq!(format_ipv6(parse_ipv6("::1.2.3.4").unwrap_or(0)), "::102:304");
        assert_eq!(format_ipv6(parse_ipv6("::ffff:1.2.3.4").unwrap_or(0)), "::ffff:1.2.3.4");
        assert!(parse_ipv6("1:2:3:4:5:6:7:8::").is_err());
    }

    #[test]
    fn masks() {
        assert_eq!(mask_of(Version::V6, 0), 0);
        assert_eq!(mask_of(Version::V4, 24), 0xffff_ff00);
        assert_eq!(prefix_from_netmask("0.0.0.0").unwrap_or(99), 0);
        assert_eq!(prefix_from_netmask("255.255.255.255").unwrap_or(99), 32);
        assert!(prefix_from_netmask("255.0.255.0").is_err());
    }
}
