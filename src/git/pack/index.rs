use nom::{
    IResult, Parser,
    bytes::complete::{tag, take},
    error::{Error, ErrorKind},
    multi::count,
    number::complete::{be_u32, be_u64},
};
use std::fmt;

use crate::git::pack::PackError;

/// Represents a Git pack index file (.idx)
///
/// Pack index files provide efficient lookup into pack files by mapping
/// object SHA-1 hashes to their byte offsets within the corresponding pack file.
#[derive(Debug, Clone)]
pub struct PackIndex {
    /// Index file format version (should be 2)
    pub version: u32,
    /// Fan-out table: 256 entries indicating object count for each first byte
    pub fan_out: [u32; 256],
    /// Sorted array of 20-byte SHA-1 object names
    pub object_names: Vec<[u8; 20]>,
    /// CRC32 checksums for packed object data (for integrity verification)
    pub crc32_checksums: Vec<u32>,
    /// 4-byte offsets into the pack file for each object
    pub offsets: Vec<u32>,
    /// Optional 8-byte offsets for large pack files (when 4-byte offset has MSB set)
    pub large_offsets: Option<Vec<u64>>,
    /// SHA-1 checksum of the corresponding pack file
    pub pack_checksum: [u8; 20],
    /// SHA-1 checksum of all the index data above
    pub index_checksum: [u8; 20],
    /// Raw data for debugging/display purposes
    pub raw_data: Vec<u8>,
}

impl PackIndex {
    /// Parse a pack index file from raw bytes
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let original_input = input;

        // Parse header (magic + version)
        let (input, _) = Self::parse_header(input)?;

        // Parse fan-out table (256 entries)
        let (input, fan_out) = Self::parse_fan_out_table(input)?;

        // Get total object count from last fan-out entry
        let total_objects = fan_out[255] as usize;

        // Parse object names (20 bytes each)
        let (input, object_names) = count(Self::parse_object_name, total_objects).parse(input)?;

        // Parse CRC32 table (4 bytes per object)
        let (input, crc32_checksums) = count(be_u32, total_objects).parse(input)?;

        // Parse offset table (4 bytes per object)
        let (input, offsets) = count(be_u32, total_objects).parse(input)?;

        // Check if we need large offsets (any offset has MSB set)
        let needs_large_offsets = offsets.iter().any(|&offset| offset & 0x80000000 != 0);
        let large_offset_count = offsets
            .iter()
            .filter(|&&offset| offset & 0x80000000 != 0)
            .count();

        // Parse large offset table if needed
        let (input, large_offsets) = if needs_large_offsets {
            let (input, large_offsets) = count(be_u64, large_offset_count).parse(input)?;
            (input, Some(large_offsets))
        } else {
            (input, None)
        };

        // Parse pack checksum (20 bytes)
        let (input, pack_checksum_bytes) = take(20usize)(input)?;
        let mut pack_checksum = [0u8; 20];
        pack_checksum.copy_from_slice(pack_checksum_bytes);

        // Parse index checksum (20 bytes)
        let (input, index_checksum_bytes) = take(20usize)(input)?;
        let mut index_checksum = [0u8; 20];
        index_checksum.copy_from_slice(index_checksum_bytes);

        // Calculate raw data size (everything we've consumed)
        let consumed = original_input.len() - input.len();
        let raw_data = original_input[..consumed].to_vec();

        Ok((
            input,
            PackIndex {
                version: 2, // We only support version 2
                fan_out,
                object_names,
                crc32_checksums,
                offsets,
                large_offsets,
                pack_checksum,
                index_checksum,
                raw_data,
            },
        ))
    }

    /// Parse the index header (magic number + version)
    fn parse_header(input: &[u8]) -> IResult<&[u8], ()> {
        // Magic number for version 2: \377tOc (0xff744f63)
        let magic_bytes: &[u8] = &[0xff, 0x74, 0x4f, 0x63];
        let (input, _) = tag(magic_bytes)(input)?;

        // Version number (should be 2)
        let (input, version) = be_u32(input)?;

        if version != 2 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        Ok((input, ()))
    }

    /// Parse the fan-out table (256 entries)
    fn parse_fan_out_table(input: &[u8]) -> IResult<&[u8], [u32; 256]> {
        let (input, fan_out_vec) = count(be_u32, 256).parse(input)?;

        // Convert Vec to array
        let mut fan_out = [0u32; 256];
        for (i, &value) in fan_out_vec.iter().enumerate() {
            fan_out[i] = value;
        }

        // Validate fan-out table is monotonic
        for i in 1..256 {
            if fan_out[i] < fan_out[i - 1] {
                return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
            }
        }

        Ok((input, fan_out))
    }

    /// Parse a single 20-byte object name (SHA-1)
    fn parse_object_name(input: &[u8]) -> IResult<&[u8], [u8; 20]> {
        let (input, name_bytes) = take(20usize)(input)?;
        let mut name = [0u8; 20];
        name.copy_from_slice(name_bytes);
        Ok((input, name))
    }

    /// Get the total number of objects in this index
    pub fn object_count(&self) -> usize {
        self.object_names.len()
    }

    /// Look up an object by its SHA-1 hash
    /// Returns the offset in the pack file if found
    pub fn lookup_object(&self, sha1: &[u8; 20]) -> Option<u64> {
        // Use fan-out table for efficient binary search
        let first_byte = sha1[0] as usize;

        // Determine search range using fan-out table
        let start_idx = if first_byte == 0 {
            0
        } else {
            self.fan_out[first_byte - 1] as usize
        };
        let end_idx = self.fan_out[first_byte] as usize;

        // Binary search within the range
        let search_slice = &self.object_names[start_idx..end_idx];

        match search_slice.binary_search(sha1) {
            Ok(relative_idx) => {
                let absolute_idx = start_idx + relative_idx;
                Some(self.get_object_offset(absolute_idx))
            }
            Err(_) => None,
        }
    }

    /// Get the pack file offset for an object at the given index
    pub fn get_object_offset(&self, index: usize) -> u64 {
        if index >= self.offsets.len() {
            return 0;
        }

        let offset = self.offsets[index];

        // Check if this is a large offset (MSB set)
        if offset & 0x80000000 != 0 {
            // Use large offset table
            let large_offset_index = (offset & 0x7fffffff) as usize;
            if let Some(ref large_offsets) = self.large_offsets
                && large_offset_index < large_offsets.len()
            {
                return large_offsets[large_offset_index];
            }
        }

        offset as u64
    }

    /// Get the CRC32 checksum for an object at the given index
    pub fn get_object_crc32(&self, index: usize) -> Option<u32> {
        self.crc32_checksums.get(index).copied()
    }

    /// Verify the integrity of the index file
    pub fn verify_checksum(&self) -> Result<(), PackError> {
        // TODO: Implement SHA-1 checksum verification
        // This would involve calculating SHA-1 of all data except the final 20 bytes
        // and comparing with self.index_checksum
        Ok(())
    }
}

impl fmt::Display for PackIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Pack Index (version {})", self.version)?;
        writeln!(f, "Total objects: {}", self.object_count())?;
        writeln!(f, "Pack checksum: {}", hex::encode(self.pack_checksum))?;
        writeln!(f, "Index checksum: {}", hex::encode(self.index_checksum))?;

        if let Some(ref large_offsets) = self.large_offsets {
            writeln!(f, "Large offsets: {} entries", large_offsets.len())?;
        }

        // Show distribution from fan-out table
        writeln!(f, "\nObject distribution by first byte:")?;
        let mut prev_count = 0;
        for (byte, &count) in self.fan_out.iter().enumerate() {
            let objects_for_byte = count - prev_count;
            if objects_for_byte > 0 {
                writeln!(f, "  0x{:02x}: {} objects", byte, objects_for_byte)?;
            }
            prev_count = count;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        let header_data = [
            0xff, 0x74, 0x4f, 0x63, // Magic number
            0x00, 0x00, 0x00, 0x02, // Version 2
        ];

        let (remaining, _) = PackIndex::parse_header(&header_data).unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_parse_object_name() {
        let name_data = [0x01; 20]; // 20 bytes of 0x01

        let (remaining, name) = PackIndex::parse_object_name(&name_data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(name, [0x01; 20]);
    }

    #[test]
    fn test_fan_out_validation() {
        // Valid monotonic fan-out table
        let mut fan_out_data = Vec::new();
        for i in 0..256 {
            fan_out_data.extend_from_slice(&(i as u32).to_be_bytes());
        }

        let (_, fan_out) = PackIndex::parse_fan_out_table(&fan_out_data).unwrap();
        assert_eq!(fan_out[0], 0);
        assert_eq!(fan_out[255], 255);
    }

    #[test]
    fn test_lookup_object() {
        // Create a minimal index for testing
        let mut fan_out = [0u32; 256];
        fan_out[0] = 1; // One object starting with 0x00
        for i in 1..256 {
            fan_out[i] = 1; // All subsequent entries have the same count
        }

        let object_names = vec![[0x00; 20]]; // One object with all zeros
        let crc32_checksums = vec![0x12345678];
        let offsets = vec![100]; // Offset 100 in pack file

        let index = PackIndex {
            version: 2,
            fan_out,
            object_names,
            crc32_checksums,
            offsets,
            large_offsets: None,
            pack_checksum: [0; 20],
            index_checksum: [0; 20],
            raw_data: vec![],
        };

        // Should find the object
        assert_eq!(index.lookup_object(&[0x00; 20]), Some(100));

        // Should not find a different object
        assert_eq!(index.lookup_object(&[0xff; 20]), None);
    }

    #[test]
    fn test_multiple_objects_lookup() {
        // Create an index with multiple objects for more realistic testing
        let mut fan_out = [0u32; 256];

        // Set up fan-out table for objects starting with 0x00, 0x01, 0xaa, 0xff
        fan_out[0] = 2; // 2 objects starting with 0x00 or less
        fan_out[1] = 3; // 3 objects starting with 0x01 or less  
        for i in 2..0xaa {
            fan_out[i] = 3; // Same count up to 0xaa
        }
        fan_out[0xaa] = 4; // 4 objects starting with 0xaa or less
        for i in 0xab..0xff {
            fan_out[i] = 4; // Same count up to 0xff
        }
        fan_out[0xff] = 5; // 5 objects total

        // Create 5 test objects with specific first bytes
        let mut obj1 = [0u8; 20];
        obj1[0] = 0x00;
        let mut obj2 = [0u8; 20];
        obj2[0] = 0x00;
        obj2[1] = 0x01; // Different from obj1
        let mut obj3 = [0u8; 20];
        obj3[0] = 0x01;
        let mut obj4 = [0u8; 20];
        obj4[0] = 0xaa;
        let mut obj5 = [0u8; 20];
        obj5[0] = 0xff;

        let object_names = vec![obj1, obj2, obj3, obj4, obj5];
        let crc32_checksums = vec![0x11111111, 0x22222222, 0x33333333, 0x44444444, 0x55555555];
        let offsets = vec![100, 200, 300, 400, 500];

        let index = PackIndex {
            version: 2,
            fan_out,
            object_names,
            crc32_checksums,
            offsets,
            large_offsets: None,
            pack_checksum: [0; 20],
            index_checksum: [0; 20],
            raw_data: vec![],
        };

        // Test lookups
        assert_eq!(index.lookup_object(&obj1), Some(100));
        assert_eq!(index.lookup_object(&obj2), Some(200));
        assert_eq!(index.lookup_object(&obj3), Some(300));
        assert_eq!(index.lookup_object(&obj4), Some(400));
        assert_eq!(index.lookup_object(&obj5), Some(500));

        // Test non-existent object
        let mut nonexistent = [0u8; 20];
        nonexistent[0] = 0x80; // First byte that doesn't match any object
        assert_eq!(index.lookup_object(&nonexistent), None);

        // Test CRC32 access
        assert_eq!(index.get_object_crc32(0), Some(0x11111111));
        assert_eq!(index.get_object_crc32(4), Some(0x55555555));
        assert_eq!(index.get_object_crc32(10), None); // Out of bounds
    }

    #[test]
    fn test_large_offsets() {
        // Test large offset handling
        let mut fan_out = [0u32; 256];
        fan_out[0] = 1;
        for i in 1..256 {
            fan_out[i] = 1;
        }

        let object_names = vec![[0x00; 20]];
        let crc32_checksums = vec![0x12345678];
        let offsets = vec![0x80000000]; // MSB set indicates large offset
        let large_offsets = Some(vec![0x123456789abcdef0]); // Large 8-byte offset

        let index = PackIndex {
            version: 2,
            fan_out,
            object_names,
            crc32_checksums,
            offsets,
            large_offsets,
            pack_checksum: [0; 20],
            index_checksum: [0; 20],
            raw_data: vec![],
        };

        // Should return the large offset
        assert_eq!(index.get_object_offset(0), 0x123456789abcdef0);
        assert_eq!(index.lookup_object(&[0x00; 20]), Some(0x123456789abcdef0));
    }
}
