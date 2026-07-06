//! Shared formatting utilities for widget formatters

/// Format a 32-bit value as hex bytes (4 bytes, big-endian)
///
/// # Examples
///
/// ```
/// use git_plumber::tui::widget::formatters_utils::format_u32_as_hex_bytes;
///
/// let formatted = format_u32_as_hex_bytes(0x12345678);
/// assert_eq!(formatted, "12 34 56 78");
/// ```
#[must_use]
pub fn format_u32_as_hex_bytes(value: u32) -> String {
    format!(
        "{:02x} {:02x} {:02x} {:02x}",
        (value >> 24) & 0xff,
        (value >> 16) & 0xff,
        (value >> 8) & 0xff,
        value & 0xff
    )
}

/// Format an epoch-seconds timestamp as a human-readable UTC datetime
///
/// Uses the days-to-civil-date algorithm (Howard Hinnant's `civil_from_days`)
/// to avoid pulling in a date/time crate for display-only formatting.
///
/// # Examples
///
/// ```
/// use git_plumber::tui::widget::formatters_utils::format_epoch_utc;
///
/// assert_eq!(format_epoch_utc(0), "1970-01-01 00:00:00 UTC");
/// assert_eq!(format_epoch_utc(1_700_000_000), "2023-11-14 22:13:20 UTC");
/// ```
#[must_use]
pub fn format_epoch_utc(epoch: u32) -> String {
    let secs = i64::from(epoch);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);

    // Shift epoch from 1970-01-01 to 0000-03-01 so leap days fall at era ends
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097; // day of 400-year era
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // year of era
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of March-based year
    let mp = (5 * doy + 2) / 153; // March-based month
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_epoch_utc() {
        assert_eq!(format_epoch_utc(0), "1970-01-01 00:00:00 UTC");
        // Leap-year day
        assert_eq!(format_epoch_utc(951_782_400), "2000-02-29 00:00:00 UTC");
        assert_eq!(format_epoch_utc(1_700_000_000), "2023-11-14 22:13:20 UTC");
        // u32 max: far future, must not panic or wrap
        assert_eq!(format_epoch_utc(u32::MAX), "2106-02-07 06:28:15 UTC");
    }

    #[test]
    fn test_format_u32_as_hex_bytes() {
        assert_eq!(format_u32_as_hex_bytes(0x00000000), "00 00 00 00");
        assert_eq!(format_u32_as_hex_bytes(0x12345678), "12 34 56 78");
        assert_eq!(format_u32_as_hex_bytes(0xffffffff), "ff ff ff ff");
        assert_eq!(format_u32_as_hex_bytes(0x00000001), "00 00 00 01");
        assert_eq!(format_u32_as_hex_bytes(0x01000000), "01 00 00 00");
    }
}
