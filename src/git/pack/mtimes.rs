use nom::{
    IResult, Parser,
    bytes::complete::take,
    error::{Error, ErrorKind},
    multi::count,
    number::complete::be_u32,
};
use std::fmt;

/// Represents a Git pack mtimes file (.mtimes)
///
/// Mtimes files accompany cruft packs, which store unreachable-but-not-yet-expired
/// objects. Packing such objects together would normally reset their filesystem
/// mtimes, so the .mtimes file records a per-object modification time instead,
/// letting `git gc` expire each object individually.
#[derive(Debug, Clone)]
pub struct PackMtimes {
    /// Mtimes file format version (should be 1)
    pub version: u32,
    /// Hash function identifier (1 for SHA-1, 2 for SHA-256)
    pub hash_function_id: u32,
    /// Table of modification times in epoch seconds, one per packed object
    /// The i-th entry corresponds to the i-th object in .idx (lexicographic OID) order
    pub mtimes: Vec<u32>,
    /// SHA-1/SHA-256 checksum of the corresponding pack file
    pub pack_checksum: Vec<u8>,
    /// SHA-1/SHA-256 checksum of all the above content
    pub file_checksum: Vec<u8>,
    /// Raw data for debugging/display purposes
    pub raw_data: Vec<u8>,
}

impl PackMtimes {
    /// Magic signature for mtimes files: "MTME"
    pub const SIGNATURE: u32 = 0x4d54_4d45;
    /// Current supported version
    pub const VERSION: u32 = 1;

    /// Parse a pack mtimes file from raw bytes
    ///
    /// # Errors
    ///
    /// Returns a nom parse error if the input is not a valid mtimes file:
    /// wrong "MTME" signature, unsupported version or hash function ID, or a
    /// file size inconsistent with the header and trailing checksums.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let original_input = input;

        // Parse header (signature + version + hash function ID)
        let (input, (version, hash_function_id)) = Self::parse_header(input)?;

        // Calculate number of objects from remaining data
        // Formula: (total_size - header_size - 2*checksum_size) / 4
        let header_size = 12; // 4 bytes signature + 4 bytes version + 4 bytes hash function ID
        let checksum_size = match hash_function_id {
            1 => 20, // SHA-1
            2 => 32, // SHA-256
            _ => return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))),
        };

        let remaining_after_header = original_input.len() - header_size;
        // A truncated file can be shorter than the two trailing checksums
        let Some(data_size) = remaining_after_header.checked_sub(2 * checksum_size) else {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        };

        if data_size % 4 != 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        }

        let num_objects = data_size / 4;

        // Parse mtimes table (4 bytes per object)
        let (input, mtimes) = count(be_u32, num_objects).parse(input)?;

        // Parse pack checksum
        let (input, pack_checksum_bytes) = take(checksum_size)(input)?;
        let pack_checksum = pack_checksum_bytes.to_vec();

        // Parse file checksum
        let (input, file_checksum_bytes) = take(checksum_size)(input)?;
        let file_checksum = file_checksum_bytes.to_vec();

        // Calculate raw data size (everything we've consumed)
        let consumed = original_input.len() - input.len();
        let raw_data = original_input[..consumed].to_vec();

        Ok((
            input,
            Self {
                version,
                hash_function_id,
                mtimes,
                pack_checksum,
                file_checksum,
                raw_data,
            },
        ))
    }

    /// Parse the mtimes header (signature + version + hash function ID)
    fn parse_header(input: &[u8]) -> IResult<&[u8], (u32, u32)> {
        // Magic signature: "MTME" (0x4d544d45)
        let (input, signature) = be_u32(input)?;
        if signature != Self::SIGNATURE {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Version number (should be 1)
        let (input, version) = be_u32(input)?;
        if version != Self::VERSION {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Hash function identifier
        let (input, hash_function_id) = be_u32(input)?;
        if hash_function_id != 1 && hash_function_id != 2 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        Ok((input, (version, hash_function_id)))
    }

    /// Get the total number of objects in this mtimes file
    #[must_use]
    pub const fn object_count(&self) -> usize {
        self.mtimes.len()
    }

    /// Get the modification time (epoch seconds) of the object at the given
    /// index position (position in the sorted object ID list from the .idx file)
    #[must_use]
    pub fn mtime_at(&self, index_pos: usize) -> Option<u32> {
        self.mtimes.get(index_pos).copied()
    }

    /// Get the hash function name as a string
    #[must_use]
    pub const fn hash_function_name(&self) -> &'static str {
        match self.hash_function_id {
            1 => "SHA-1",
            2 => "SHA-256",
            _ => "Unknown",
        }
    }

    /// Get the checksum size based on hash function
    #[must_use]
    pub const fn checksum_size(&self) -> usize {
        match self.hash_function_id {
            1 => 20, // SHA-1
            2 => 32, // SHA-256
            _ => 0,
        }
    }
}

impl fmt::Display for PackMtimes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Pack Mtimes (version {})", self.version)?;
        writeln!(
            f,
            "Hash function: {} (ID: {})",
            self.hash_function_name(),
            self.hash_function_id
        )?;
        writeln!(f, "Total objects: {}", self.object_count())?;
        writeln!(f, "Pack checksum: {}", hex::encode(&self.pack_checksum))?;
        writeln!(f, "File checksum: {}", hex::encode(&self.file_checksum))?;

        // Show some sample mtimes
        writeln!(f, "\nSample index position → mtime (epoch seconds):")?;
        let sample_count = std::cmp::min(10, self.object_count());
        for index_pos in 0..sample_count {
            if let Some(mtime) = self.mtime_at(index_pos) {
                writeln!(f, "  Index pos {index_pos:4} → {mtime}")?;
            }
        }

        if self.object_count() > sample_count {
            writeln!(
                f,
                "  ... ({} more objects)",
                self.object_count() - sample_count
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mtimes_data(num_objects: usize, hash_function_id: u32) -> Vec<u8> {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&PackMtimes::SIGNATURE.to_be_bytes()); // Signature
        data.extend_from_slice(&PackMtimes::VERSION.to_be_bytes()); // Version
        data.extend_from_slice(&hash_function_id.to_be_bytes()); // Hash function ID

        // Mtimes table (example: one-minute increments from a fixed base)
        for i in 0..num_objects {
            let mtime = 1_700_000_000u32 + (i as u32) * 60;
            data.extend_from_slice(&mtime.to_be_bytes());
        }

        // Pack checksum (dummy)
        let checksum_size = match hash_function_id {
            1 => 20,
            2 => 32,
            _ => 20,
        };
        data.extend_from_slice(&vec![0xaa; checksum_size]);

        // File checksum (dummy)
        data.extend_from_slice(&vec![0xbb; checksum_size]);

        data
    }

    #[test]
    fn test_parse_header() {
        let header_data = [
            0x4d, 0x54, 0x4d, 0x45, // "MTME" signature
            0x00, 0x00, 0x00, 0x01, // Version 1
            0x00, 0x00, 0x00, 0x01, // SHA-1 (hash function ID 1)
        ];

        let (remaining, (version, hash_function_id)) =
            PackMtimes::parse_header(&header_data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(version, 1);
        assert_eq!(hash_function_id, 1);
    }

    #[test]
    fn test_parse_complete_mtimes() {
        let data = create_test_mtimes_data(5, 1); // 5 objects, SHA-1
        let (remaining, mtimes) = PackMtimes::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(mtimes.version, 1);
        assert_eq!(mtimes.hash_function_id, 1);
        assert_eq!(mtimes.object_count(), 5);
        assert_eq!(mtimes.hash_function_name(), "SHA-1");
        assert_eq!(mtimes.checksum_size(), 20);

        // One-minute increments from the test data builder
        assert_eq!(mtimes.mtime_at(0), Some(1_700_000_000));
        assert_eq!(mtimes.mtime_at(1), Some(1_700_000_060));
        assert_eq!(mtimes.mtime_at(4), Some(1_700_000_240));
        assert_eq!(mtimes.mtime_at(5), None); // Out of bounds
    }

    #[test]
    fn test_parse_sha256_mtimes() {
        let data = create_test_mtimes_data(3, 2); // 3 objects, SHA-256
        let (remaining, mtimes) = PackMtimes::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(mtimes.version, 1);
        assert_eq!(mtimes.hash_function_id, 2);
        assert_eq!(mtimes.object_count(), 3);
        assert_eq!(mtimes.hash_function_name(), "SHA-256");
        assert_eq!(mtimes.checksum_size(), 32);

        // Verify checksums are the right size
        assert_eq!(mtimes.pack_checksum.len(), 32);
        assert_eq!(mtimes.file_checksum.len(), 32);
    }

    #[test]
    fn test_empty_pack() {
        // A cruft pack can legitimately have zero objects
        let data = create_test_mtimes_data(0, 1);
        let (remaining, mtimes) = PackMtimes::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(mtimes.object_count(), 0);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = create_test_mtimes_data(1, 1);
        // Corrupt the signature
        data[0] = 0x00;

        let result = PackMtimes::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version() {
        let header_data = [
            0x4d, 0x54, 0x4d, 0x45, // "MTME" signature
            0x00, 0x00, 0x00, 0x02, // Version 2 (invalid)
            0x00, 0x00, 0x00, 0x01, // SHA-1
        ];

        let result = PackMtimes::parse_header(&header_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hash_function() {
        let header_data = [
            0x4d, 0x54, 0x4d, 0x45, // "MTME" signature
            0x00, 0x00, 0x00, 0x01, // Version 1
            0x00, 0x00, 0x00, 0x03, // Invalid hash function ID
        ];

        let result = PackMtimes::parse_header(&header_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_file_shorter_than_checksums() {
        // Valid 12-byte header but not enough bytes left for the two
        // trailing checksums — must error, not underflow
        let mut data = Vec::new();
        data.extend_from_slice(&PackMtimes::SIGNATURE.to_be_bytes());
        data.extend_from_slice(&PackMtimes::VERSION.to_be_bytes());
        data.extend_from_slice(&1u32.to_be_bytes()); // SHA-1
        data.extend_from_slice(&[0xaa; 10]); // truncated tail

        assert!(PackMtimes::parse(&data).is_err());
    }

    #[test]
    fn test_misaligned_mtimes_table() {
        // Mtimes table size not a multiple of 4 — must error
        let mut data = create_test_mtimes_data(2, 1);
        data.insert(12, 0x00); // extra byte between header and table

        assert!(PackMtimes::parse(&data).is_err());
    }

    #[test]
    fn test_display_format() {
        let data = create_test_mtimes_data(3, 1);
        let (_, mtimes) = PackMtimes::parse(&data).unwrap();

        let display_str = format!("{mtimes}");
        assert!(display_str.contains("Pack Mtimes (version 1)"));
        assert!(display_str.contains("Hash function: SHA-1"));
        assert!(display_str.contains("Total objects: 3"));
        assert!(display_str.contains("Index pos"));
        assert!(display_str.contains("1700000000"));
    }
}
