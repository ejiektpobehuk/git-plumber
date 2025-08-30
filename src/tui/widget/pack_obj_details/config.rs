use ratatui::style::Color;
use std::fmt;

// Constants for formatting
pub const COLORS: [Color; 4] = [Color::Blue, Color::Magenta, Color::Cyan, Color::Red];
pub const PREVIEW_SIZE_LIMIT: usize = 1000;
pub const HEX_PREVIEW_LIMIT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderSection {
    Size,
    Hash,
    Offset,
}

impl fmt::Display for HeaderSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size => write!(f, "size"),
            Self::Hash => write!(f, "hash"),
            Self::Offset => write!(f, "offset"),
        }
    }
}

impl HeaderSection {
    #[must_use]
    pub fn from_byte_position(
        byte_index: usize,
        obj_type: crate::git::pack::ObjectType,
        raw_data_len: usize,
        size_byte_count: usize,
    ) -> Self {
        const HASH_BYTES: usize = 20;
        let is_ref_delta_hash = obj_type == crate::git::pack::ObjectType::RefDelta
            && raw_data_len >= HASH_BYTES
            && byte_index >= raw_data_len - HASH_BYTES;

        let is_ofs_delta_offset =
            obj_type == crate::git::pack::ObjectType::OfsDelta && byte_index >= size_byte_count;

        if is_ref_delta_hash {
            Self::Hash
        } else if is_ofs_delta_offset {
            Self::Offset
        } else {
            Self::Size
        }
    }
}

// Helper function to calculate the number of bytes used for size encoding
#[must_use]
pub fn calculate_size_byte_count(obj_type: crate::git::pack::ObjectType, raw_data: &[u8]) -> usize {
    const HASH_BYTES: usize = 20;
    match obj_type {
        crate::git::pack::ObjectType::RefDelta => {
            // RefDelta: size bytes + 20-byte hash
            raw_data.len() - HASH_BYTES
        }
        crate::git::pack::ObjectType::OfsDelta => {
            // OfsDelta: find where size encoding ends
            let mut size_bytes = 0;
            for (i, &byte) in raw_data.iter().enumerate() {
                size_bytes = i + 1;
                if byte & 0x80 == 0 {
                    // No continuation bit, this is the last size byte
                    break;
                }
            }
            size_bytes
        }
        _ => raw_data.len(), // Regular objects use all bytes for size
    }
}

// Add Adler-32 checksum calculation function
#[must_use]
pub fn calculate_adler32(data: &[u8]) -> u32 {
    const ADLER32_MODULUS: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;

    for &byte in data {
        a = (a + u32::from(byte)) % ADLER32_MODULUS;
        b = (b + a) % ADLER32_MODULUS;
    }

    (b << 16) | a
}
