/// Shared formatting utilities for widget formatters

/// Format a 32-bit value as hex bytes (4 bytes, big-endian)
///
/// # Examples
///
/// ```
/// let formatted = format_u32_as_hex_bytes(0x12345678);
/// assert_eq!(formatted, "12 34 56 78");
/// ```
pub fn format_u32_as_hex_bytes(value: u32) -> String {
    format!(
        "{:02x} {:02x} {:02x} {:02x}",
        (value >> 24) & 0xff,
        (value >> 16) & 0xff,
        (value >> 8) & 0xff,
        value & 0xff
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_u32_as_hex_bytes() {
        assert_eq!(format_u32_as_hex_bytes(0x00000000), "00 00 00 00");
        assert_eq!(format_u32_as_hex_bytes(0x12345678), "12 34 56 78");
        assert_eq!(format_u32_as_hex_bytes(0xffffffff), "ff ff ff ff");
        assert_eq!(format_u32_as_hex_bytes(0x00000001), "00 00 00 01");
        assert_eq!(format_u32_as_hex_bytes(0x01000000), "01 00 00 00");
    }
}
