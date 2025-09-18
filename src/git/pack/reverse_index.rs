use nom::{
    IResult, Parser,
    bytes::complete::take,
    error::{Error, ErrorKind},
    multi::count,
    number::complete::be_u32,
};
use std::fmt;

use crate::git::pack::PackError;

/// Represents a Git pack reverse index file (.rev)
///
/// Reverse index files enable efficient conversion between three key properties:
/// - Index position: Position in the sorted object ID list from the .idx file
/// - Pack position: Position in pack file order (0 = first object, 1 = second, etc.)
/// - Offset: Byte offset within the pack file where object contents are found
#[derive(Debug, Clone)]
pub struct PackReverseIndex {
    /// Reverse index file format version (should be 1)
    pub version: u32,
    /// Hash function identifier (1 for SHA-1, 2 for SHA-256)
    pub hash_function_id: u32,
    /// Table of index positions, one per packed object
    /// Objects are sorted by their corresponding offsets in the packfile
    pub index_positions: Vec<u32>,
    /// SHA-1/SHA-256 checksum of the corresponding pack file
    pub pack_checksum: Vec<u8>,
    /// SHA-1/SHA-256 checksum of all the above content
    pub file_checksum: Vec<u8>,
    /// Raw data for debugging/display purposes
    pub raw_data: Vec<u8>,
}

impl PackReverseIndex {
    /// Magic signature for reverse index files: "RIDX"
    pub const SIGNATURE: u32 = 0x52494458;
    /// Current supported version
    pub const VERSION: u32 = 1;

    /// Parse a pack reverse index file from raw bytes
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
        let data_size = remaining_after_header - 2 * checksum_size;

        if data_size % 4 != 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        }

        let num_objects = data_size / 4;

        // Parse index positions table (4 bytes per object)
        let (input, index_positions) = count(be_u32, num_objects).parse(input)?;

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
                index_positions,
                pack_checksum,
                file_checksum,
                raw_data,
            },
        ))
    }

    /// Parse the reverse index header (signature + version + hash function ID)
    fn parse_header(input: &[u8]) -> IResult<&[u8], (u32, u32)> {
        // Magic signature: "RIDX" (0x52494458)
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

    /// Get the total number of objects in this reverse index
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.index_positions.len()
    }

    /// Convert pack position to index position
    /// This is an O(1) operation - the main benefit of the reverse index
    #[must_use]
    pub fn pack_pos_to_index(&self, pack_pos: usize) -> Option<u32> {
        self.index_positions.get(pack_pos).copied()
    }

    /// Get the hash function name as a string
    #[must_use]
    pub fn hash_function_name(&self) -> &'static str {
        match self.hash_function_id {
            1 => "SHA-1",
            2 => "SHA-256",
            _ => "Unknown",
        }
    }

    /// Get the checksum size based on hash function
    #[must_use]
    pub fn checksum_size(&self) -> usize {
        match self.hash_function_id {
            1 => 20, // SHA-1
            2 => 32, // SHA-256
            _ => 0,
        }
    }

    /// Verify the integrity of the reverse index file
    pub const fn verify_checksum(&self) -> Result<(), PackError> {
        // TODO: Implement checksum verification
        // This would involve calculating the hash of all data except the final checksum bytes
        // and comparing with self.file_checksum
        Ok(())
    }
}

impl fmt::Display for PackReverseIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Pack Reverse Index (version {})", self.version)?;
        writeln!(
            f,
            "Hash function: {} (ID: {})",
            self.hash_function_name(),
            self.hash_function_id
        )?;
        writeln!(f, "Total objects: {}", self.object_count())?;
        writeln!(f, "Pack checksum: {}", hex::encode(&self.pack_checksum))?;
        writeln!(f, "File checksum: {}", hex::encode(&self.file_checksum))?;

        // Show some sample mappings
        writeln!(f, "\nSample pack position → index position mappings:")?;
        let sample_count = std::cmp::min(10, self.object_count());
        for pack_pos in 0..sample_count {
            if let Some(index_pos) = self.pack_pos_to_index(pack_pos) {
                writeln!(f, "  Pack pos {pack_pos:4} → Index pos {index_pos:4}")?;
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

    fn create_test_reverse_index_data(num_objects: usize, hash_function_id: u32) -> Vec<u8> {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&PackReverseIndex::SIGNATURE.to_be_bytes()); // Signature
        data.extend_from_slice(&PackReverseIndex::VERSION.to_be_bytes()); // Version
        data.extend_from_slice(&hash_function_id.to_be_bytes()); // Hash function ID

        // Index positions table (example: reverse order)
        for i in 0..num_objects {
            let index_pos = (num_objects - 1 - i) as u32;
            data.extend_from_slice(&index_pos.to_be_bytes());
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
            0x52, 0x49, 0x44, 0x58, // "RIDX" signature
            0x00, 0x00, 0x00, 0x01, // Version 1
            0x00, 0x00, 0x00, 0x01, // SHA-1 (hash function ID 1)
        ];

        let (remaining, (version, hash_function_id)) =
            PackReverseIndex::parse_header(&header_data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(version, 1);
        assert_eq!(hash_function_id, 1);
    }

    #[test]
    fn test_parse_complete_reverse_index() {
        let data = create_test_reverse_index_data(5, 1); // 5 objects, SHA-1
        let (remaining, reverse_index) = PackReverseIndex::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(reverse_index.version, 1);
        assert_eq!(reverse_index.hash_function_id, 1);
        assert_eq!(reverse_index.object_count(), 5);
        assert_eq!(reverse_index.hash_function_name(), "SHA-1");
        assert_eq!(reverse_index.checksum_size(), 20);

        // Test the reverse mapping (our test data uses reverse order)
        assert_eq!(reverse_index.pack_pos_to_index(0), Some(4)); // First pack pos maps to last index pos
        assert_eq!(reverse_index.pack_pos_to_index(1), Some(3));
        assert_eq!(reverse_index.pack_pos_to_index(2), Some(2));
        assert_eq!(reverse_index.pack_pos_to_index(3), Some(1));
        assert_eq!(reverse_index.pack_pos_to_index(4), Some(0)); // Last pack pos maps to first index pos
        assert_eq!(reverse_index.pack_pos_to_index(5), None); // Out of bounds
    }

    #[test]
    fn test_parse_sha256_reverse_index() {
        let data = create_test_reverse_index_data(3, 2); // 3 objects, SHA-256
        let (remaining, reverse_index) = PackReverseIndex::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(reverse_index.version, 1);
        assert_eq!(reverse_index.hash_function_id, 2);
        assert_eq!(reverse_index.object_count(), 3);
        assert_eq!(reverse_index.hash_function_name(), "SHA-256");
        assert_eq!(reverse_index.checksum_size(), 32);

        // Verify checksums are the right size
        assert_eq!(reverse_index.pack_checksum.len(), 32);
        assert_eq!(reverse_index.file_checksum.len(), 32);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = create_test_reverse_index_data(1, 1);
        // Corrupt the signature
        data[0] = 0x00;

        let result = PackReverseIndex::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version() {
        let header_data = [
            0x52, 0x49, 0x44, 0x58, // "RIDX" signature
            0x00, 0x00, 0x00, 0x02, // Version 2 (invalid)
            0x00, 0x00, 0x00, 0x01, // SHA-1
        ];

        let result = PackReverseIndex::parse_header(&header_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hash_function() {
        let header_data = [
            0x52, 0x49, 0x44, 0x58, // "RIDX" signature
            0x00, 0x00, 0x00, 0x01, // Version 1
            0x00, 0x00, 0x00, 0x03, // Invalid hash function ID
        ];

        let result = PackReverseIndex::parse_header(&header_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_display_format() {
        let data = create_test_reverse_index_data(3, 1);
        let (_, reverse_index) = PackReverseIndex::parse(&data).unwrap();

        let display_str = format!("{reverse_index}");
        assert!(display_str.contains("Pack Reverse Index (version 1)"));
        assert!(display_str.contains("Hash function: SHA-1"));
        assert!(display_str.contains("Total objects: 3"));
        assert!(display_str.contains("Pack pos"));
        assert!(display_str.contains("Index pos"));
    }
}
