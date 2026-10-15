/// CRC-32 (IEEE 802.3) of data as the 4 big-endian bytes.
pub fn crc32_bytes(data: &[u8]) -> [u8; 4] {
    let mut crc = flate2::Crc::new();
    crc.update(data);
    crc.sum().to_be_bytes()
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_vector() {
        assert_eq!(super::crc32_bytes(b"123456789"), [0xcb, 0xf4, 0x39, 0x26]);
        assert_eq!(super::crc32_bytes(b""), [0, 0, 0, 0]);
    }
}
