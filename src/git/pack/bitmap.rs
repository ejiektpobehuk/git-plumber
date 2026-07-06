use nom::{
    IResult, Parser,
    bytes::complete::take,
    error::{Error, ErrorKind},
    multi::count,
    number::complete::{be_u16, be_u32, be_u64, u8 as be_u8},
};
use std::fmt;

/// An EWAH-compressed bitmap as serialized inside a .bitmap file
///
/// EWAH stores the bitstream as chunks of 64-bit words: a run-length word
/// (RLW) describing K repeated words of a single bit, followed by M literal
/// words copied verbatim.
#[derive(Debug, Clone)]
pub struct EwahBitmap {
    /// Number of bits in the uncompressed bitmap
    pub bit_count: u32,
    /// Number of compressed 64-bit words stored
    pub word_count: u32,
    /// The compressed words themselves
    pub words: Vec<u64>,
    /// Byte position of the current (last) RLW within the compressed stream
    pub rlw_position: u32,
}

impl EwahBitmap {
    /// Parse a serialized EWAH bitmap (JavaEWAH / JGit compatible layout)
    ///
    /// # Errors
    ///
    /// Returns a nom parse error if the input is shorter than the word count
    /// declared in the EWAH header.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, bit_count) = be_u32(input)?;
        let (input, word_count) = be_u32(input)?;
        let (input, words) = count(be_u64, word_count as usize).parse(input)?;
        let (input, rlw_position) = be_u32(input)?;

        Ok((
            input,
            Self {
                bit_count,
                word_count,
                words,
                rlw_position,
            },
        ))
    }

    /// Size of this bitmap on disk: 12 bytes of header/trailer plus the words
    #[must_use]
    pub const fn compressed_byte_size(&self) -> usize {
        12 + self.words.len() * 8
    }

    /// Count the set bits by decoding the run-length encoded stream
    ///
    /// Malformed streams (literal count pointing past the stored words) are
    /// decoded up to the last complete chunk rather than causing a panic.
    #[must_use]
    pub fn count_set_bits(&self) -> u64 {
        let mut bits: u64 = 0;
        let mut i = 0;

        while i < self.words.len() {
            let rlw = self.words[i];
            // RLW layout (low to high): 1 repeated bit, 32-bit run length
            // in words, 31-bit literal word count
            let run_bit = rlw & 1;
            let run_len = (rlw >> 1) & 0xFFFF_FFFF;
            let literal_count = (rlw >> 33) as usize;

            if run_bit == 1 {
                bits += run_len * 64;
            }

            let literals_end = i + 1 + literal_count;
            if literals_end > self.words.len() {
                break;
            }
            for word in &self.words[i + 1..literals_end] {
                bits += u64::from(word.count_ones());
            }
            i = literals_end;
        }

        bits
    }
}

/// A single commit reachability bitmap entry in a .bitmap file
#[derive(Debug, Clone)]
pub struct BitmapEntry {
    /// Position of the commit in the pack index / multi-pack-index
    pub object_pos: u32,
    /// XOR compression offset: 0 means stored verbatim, y means the real
    /// bitmap is this one XORed with the entry y positions earlier
    pub xor_offset: u8,
    /// Entry flags (0x1 hints the bitmap can be reused when rebuilding)
    pub flags: u8,
    /// The EWAH-compressed (possibly XOR-encoded) reachability bitmap
    pub bitmap: EwahBitmap,
}

/// One triplet of the optional commit lookup table
#[derive(Debug, Clone)]
pub struct LookupTableEntry {
    /// Object position of the commit in the pack index / MIDX
    pub commit_pos: u32,
    /// Byte offset in the .bitmap file where the commit's bitmap starts
    pub offset: u64,
    /// Absolute lookup-table row used for XOR compression, or 0xffffffff
    pub xor_row: u32,
}

impl LookupTableEntry {
    /// Sentinel meaning "no XOR base row"
    pub const NO_XOR_ROW: u32 = 0xffff_ffff;
}

/// Represents a Git pack bitmap file (.bitmap)
///
/// Bitmap files store reachability bitmaps for a set of selected commits:
/// for each such commit, one bit per object in the pack (or multi-pack-index)
/// saying whether that object is reachable from the commit. Git uses them to
/// answer "which objects does this fetch need?" without walking the graph.
#[derive(Debug, Clone)]
pub struct PackBitmap {
    /// Bitmap file format version (should be 1)
    pub version: u16,
    /// Header flags (`FULL_DAG`, `HASH_CACHE`, `LOOKUP_TABLE`, `PSEUDO_MERGES`)
    pub flags: u16,
    /// Number of commit bitmap entries
    pub entry_count: u32,
    /// Checksum of the pack / multi-pack-index this bitmap belongs to
    pub pack_checksum: Vec<u8>,
    /// Type index bitmap: which objects are commits
    pub commits_bitmap: EwahBitmap,
    /// Type index bitmap: which objects are trees
    pub trees_bitmap: EwahBitmap,
    /// Type index bitmap: which objects are blobs
    pub blobs_bitmap: EwahBitmap,
    /// Type index bitmap: which objects are tags
    pub tags_bitmap: EwahBitmap,
    /// Commit reachability bitmap entries, in storage order
    pub entries: Vec<BitmapEntry>,
    /// Optional commit lookup table (`BITMAP_OPT_LOOKUP_TABLE`)
    pub lookup_table: Option<Vec<LookupTableEntry>>,
    /// Size in bytes of the optional pseudo-merge section (`BITMAP_OPT_PSEUDO_MERGES`)
    pub pseudo_merge_size: usize,
    /// Size in bytes of the optional name-hash cache (`BITMAP_OPT_HASH_CACHE`)
    pub hash_cache_size: usize,
    /// Trailing checksum of the preceding file contents
    pub file_checksum: Vec<u8>,
    /// Checksum length in bytes: 20 (SHA-1) or 32 (SHA-256)
    pub checksum_size: usize,
    /// Raw data for debugging/display purposes
    pub raw_data: Vec<u8>,
}

impl PackBitmap {
    /// Magic signature for bitmap files: "BITM"
    pub const SIGNATURE: u32 = 0x4249_544d;
    /// Current supported version
    pub const VERSION: u16 = 1;

    /// `BITMAP_OPT_FULL_DAG`: pack/MIDX has full closure (always required)
    pub const FLAG_FULL_DAG: u16 = 0x1;
    /// `BITMAP_OPT_HASH_CACHE`: file ends with per-object name-hash values
    pub const FLAG_HASH_CACHE: u16 = 0x4;
    /// `BITMAP_OPT_LOOKUP_TABLE`: file contains a commit lookup table
    pub const FLAG_LOOKUP_TABLE: u16 = 0x10;
    /// `BITMAP_OPT_PSEUDO_MERGES`: file contains pseudo-merge bitmaps
    pub const FLAG_PSEUDO_MERGES: u16 = 0x20;

    /// Parse a pack bitmap file from raw bytes
    ///
    /// The header does not record the hash function, so SHA-1 checksum sizes
    /// are tried first and SHA-256 as a fallback.
    ///
    /// # Errors
    ///
    /// Returns a nom parse error if the input is not a valid bitmap file:
    /// wrong "BITM" signature, unsupported version, missing required
    /// `FULL_DAG` flag, or a file size inconsistent with the header,
    /// bitmaps and optional sections.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        Self::parse_with_checksum_size(input, 20)
            .or_else(|_| Self::parse_with_checksum_size(input, 32))
    }

    fn parse_with_checksum_size(input: &[u8], checksum_size: usize) -> IResult<&[u8], Self> {
        let original_input = input;

        let (input, (version, flags, entry_count)) = Self::parse_header(input)?;

        let (input, pack_checksum_bytes) = take(checksum_size)(input)?;
        let pack_checksum = pack_checksum_bytes.to_vec();

        // Four EWAH type indexes, in fixed order
        let (input, commits_bitmap) = EwahBitmap::parse(input)?;
        let (input, trees_bitmap) = EwahBitmap::parse(input)?;
        let (input, blobs_bitmap) = EwahBitmap::parse(input)?;
        let (input, tags_bitmap) = EwahBitmap::parse(input)?;

        // Commit reachability bitmap entries
        let mut entries = Vec::with_capacity(entry_count as usize);
        let mut input = input;
        for _ in 0..entry_count {
            let (rest, object_pos) = be_u32(input)?;
            let (rest, xor_offset) = be_u8(rest)?;
            let (rest, entry_flags) = be_u8(rest)?;
            let (rest, bitmap) = EwahBitmap::parse(rest)?;
            entries.push(BitmapEntry {
                object_pos,
                xor_offset,
                flags: entry_flags,
                bitmap,
            });
            input = rest;
        }

        // Tail sections, in file order: pseudo-merges, lookup table, hash
        // cache, trailing checksum. The hash cache holds one u32 per object
        // in the pack/MIDX; that count is not stored in the file, but every
        // object is of exactly one type, so the largest uncompressed type
        // bitmap is exactly num_objects bits long.
        let num_objects = [&commits_bitmap, &trees_bitmap, &blobs_bitmap, &tags_bitmap]
            .iter()
            .map(|b| b.bit_count as usize)
            .max()
            .unwrap_or(0);

        let Some(middle_size) = input.len().checked_sub(checksum_size) else {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        };

        let lookup_table_size = if flags & Self::FLAG_LOOKUP_TABLE == 0 {
            0
        } else {
            entry_count as usize * 16
        };
        let hash_cache_size = if flags & Self::FLAG_HASH_CACHE == 0 {
            0
        } else {
            num_objects * 4
        };

        let Some(pseudo_merge_size) = middle_size.checked_sub(lookup_table_size + hash_cache_size)
        else {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        };

        // Without the pseudo-merge flag nothing else may sit between the
        // entries and the lookup table / hash cache
        if flags & Self::FLAG_PSEUDO_MERGES == 0 && pseudo_merge_size != 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        }

        let (input, _pseudo_merges) = take(pseudo_merge_size)(input)?;

        let (input, lookup_table) = if flags & Self::FLAG_LOOKUP_TABLE == 0 {
            (input, None)
        } else {
            let (input, triplets) = count(
                (be_u32, be_u64, be_u32).map(|(commit_pos, offset, xor_row)| LookupTableEntry {
                    commit_pos,
                    offset,
                    xor_row,
                }),
                entry_count as usize,
            )
            .parse(input)?;
            (input, Some(triplets))
        };

        let (input, _hash_cache) = take(hash_cache_size)(input)?;

        let (input, file_checksum_bytes) = take(checksum_size)(input)?;
        let file_checksum = file_checksum_bytes.to_vec();

        let consumed = original_input.len() - input.len();
        let raw_data = original_input[..consumed].to_vec();

        Ok((
            input,
            Self {
                version,
                flags,
                entry_count,
                pack_checksum,
                commits_bitmap,
                trees_bitmap,
                blobs_bitmap,
                tags_bitmap,
                entries,
                lookup_table,
                pseudo_merge_size,
                hash_cache_size,
                file_checksum,
                checksum_size,
                raw_data,
            },
        ))
    }

    /// Parse the bitmap header (signature + version + flags + entry count)
    fn parse_header(input: &[u8]) -> IResult<&[u8], (u16, u16, u32)> {
        // Magic signature: "BITM" (0x4249544d)
        let (input, signature) = be_u32(input)?;
        if signature != Self::SIGNATURE {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Version number (should be 1)
        let (input, version) = be_u16(input)?;
        if version != Self::VERSION {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        let (input, flags) = be_u16(input)?;
        // Git requires FULL_DAG in every bitmap file
        if flags & Self::FLAG_FULL_DAG == 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        let (input, entry_count) = be_u32(input)?;

        Ok((input, (version, flags, entry_count)))
    }

    /// Number of objects in the pack/MIDX this bitmap covers, derived from
    /// the uncompressed size of the type index bitmaps
    #[must_use]
    pub fn object_count(&self) -> usize {
        [
            &self.commits_bitmap,
            &self.trees_bitmap,
            &self.blobs_bitmap,
            &self.tags_bitmap,
        ]
        .iter()
        .map(|b| b.bit_count as usize)
        .max()
        .unwrap_or(0)
    }

    /// Get the total number of commit bitmap entries
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// True when the file carries a per-object name-hash cache
    #[must_use]
    pub const fn has_hash_cache(&self) -> bool {
        self.flags & Self::FLAG_HASH_CACHE != 0
    }

    /// True when the file carries a commit lookup table
    #[must_use]
    pub const fn has_lookup_table(&self) -> bool {
        self.flags & Self::FLAG_LOOKUP_TABLE != 0
    }

    /// True when the file carries pseudo-merge bitmaps
    #[must_use]
    pub const fn has_pseudo_merges(&self) -> bool {
        self.flags & Self::FLAG_PSEUDO_MERGES != 0
    }

    /// Names of the flags set in the header, in bit order
    #[must_use]
    pub fn flag_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.flags & Self::FLAG_FULL_DAG != 0 {
            names.push("FULL_DAG");
        }
        if self.flags & Self::FLAG_HASH_CACHE != 0 {
            names.push("HASH_CACHE");
        }
        if self.flags & Self::FLAG_LOOKUP_TABLE != 0 {
            names.push("LOOKUP_TABLE");
        }
        if self.flags & Self::FLAG_PSEUDO_MERGES != 0 {
            names.push("PSEUDO_MERGES");
        }
        names
    }

    /// Get the hash function name matching the detected checksum size
    #[must_use]
    pub const fn hash_function_name(&self) -> &'static str {
        match self.checksum_size {
            20 => "SHA-1",
            32 => "SHA-256",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for PackBitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Pack Bitmap (version {})", self.version)?;
        writeln!(
            f,
            "Flags: 0x{:04x} ({})",
            self.flags,
            self.flag_names().join(" | ")
        )?;
        writeln!(f, "Bitmapped commits: {}", self.entry_count())?;
        writeln!(f, "Objects covered: {}", self.object_count())?;
        writeln!(f, "Pack checksum: {}", hex::encode(&self.pack_checksum))?;
        writeln!(f, "File checksum: {}", hex::encode(&self.file_checksum))?;

        writeln!(f, "\nType index bitmaps (set bits):")?;
        writeln!(f, "  Commits: {}", self.commits_bitmap.count_set_bits())?;
        writeln!(f, "  Trees:   {}", self.trees_bitmap.count_set_bits())?;
        writeln!(f, "  Blobs:   {}", self.blobs_bitmap.count_set_bits())?;
        writeln!(f, "  Tags:    {}", self.tags_bitmap.count_set_bits())?;

        writeln!(f, "\nSample commit entries (object pos, xor offset):")?;
        let sample_count = std::cmp::min(10, self.entries.len());
        for entry in &self.entries[..sample_count] {
            writeln!(
                f,
                "  Pos {:6} → xor {:3}, {} set bits",
                entry.object_pos,
                entry.xor_offset,
                entry.bitmap.count_set_bits()
            )?;
        }
        if self.entries.len() > sample_count {
            writeln!(
                f,
                "  ... ({} more entries)",
                self.entries.len() - sample_count
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialize an EWAH bitmap holding the given literal words
    fn ewah_bytes(bit_count: u32, literal_words: &[u64]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&bit_count.to_be_bytes());
        // One RLW (no run, all literals) + the literal words
        let word_count = 1 + literal_words.len() as u32;
        data.extend_from_slice(&word_count.to_be_bytes());
        let rlw = (literal_words.len() as u64) << 33;
        data.extend_from_slice(&rlw.to_be_bytes());
        for word in literal_words {
            data.extend_from_slice(&word.to_be_bytes());
        }
        data.extend_from_slice(&0u32.to_be_bytes()); // RLW position
        data
    }

    fn create_test_bitmap_data(
        num_objects: u32,
        entry_count: u32,
        flags: u16,
        checksum_size: usize,
    ) -> Vec<u8> {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&PackBitmap::SIGNATURE.to_be_bytes());
        data.extend_from_slice(&PackBitmap::VERSION.to_be_bytes());
        data.extend_from_slice(&flags.to_be_bytes());
        data.extend_from_slice(&entry_count.to_be_bytes());
        data.extend_from_slice(&vec![0xaa; checksum_size]); // pack checksum

        // Type index bitmaps: first object is a commit, the rest are blobs
        data.extend_from_slice(&ewah_bytes(num_objects, &[0b1]));
        data.extend_from_slice(&ewah_bytes(num_objects, &[0b0]));
        data.extend_from_slice(&ewah_bytes(num_objects, &[!0b1]));
        data.extend_from_slice(&ewah_bytes(num_objects, &[0b0]));

        // Commit entries
        for i in 0..entry_count {
            data.extend_from_slice(&i.to_be_bytes()); // object position
            data.push(0); // xor offset
            data.push(1); // flags (reusable)
            data.extend_from_slice(&ewah_bytes(num_objects, &[0b11]));
        }

        // Optional lookup table
        if flags & PackBitmap::FLAG_LOOKUP_TABLE != 0 {
            for i in 0..entry_count {
                data.extend_from_slice(&i.to_be_bytes()); // commit_pos
                data.extend_from_slice(&u64::from(100 + i).to_be_bytes()); // offset
                data.extend_from_slice(&LookupTableEntry::NO_XOR_ROW.to_be_bytes());
            }
        }

        // Optional name-hash cache
        if flags & PackBitmap::FLAG_HASH_CACHE != 0 {
            for i in 0..num_objects {
                data.extend_from_slice(&i.to_be_bytes());
            }
        }

        // File checksum
        data.extend_from_slice(&vec![0xbb; checksum_size]);

        data
    }

    #[test]
    fn test_parse_header() {
        let header_data = [
            0x42, 0x49, 0x54, 0x4d, // "BITM" signature
            0x00, 0x01, // Version 1
            0x00, 0x01, // Flags: FULL_DAG
            0x00, 0x00, 0x00, 0x2a, // 42 entries
        ];

        let (remaining, (version, flags, entry_count)) =
            PackBitmap::parse_header(&header_data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(version, 1);
        assert_eq!(flags, PackBitmap::FLAG_FULL_DAG);
        assert_eq!(entry_count, 42);
    }

    #[test]
    fn test_parse_complete_bitmap() {
        let data = create_test_bitmap_data(64, 3, PackBitmap::FLAG_FULL_DAG, 20);
        let (remaining, bitmap) = PackBitmap::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(bitmap.version, 1);
        assert_eq!(bitmap.entry_count(), 3);
        assert_eq!(bitmap.object_count(), 64);
        assert_eq!(bitmap.checksum_size, 20);
        assert_eq!(bitmap.hash_function_name(), "SHA-1");
        assert!(!bitmap.has_hash_cache());
        assert!(!bitmap.has_lookup_table());
        assert!(!bitmap.has_pseudo_merges());
        assert!(bitmap.lookup_table.is_none());
        assert_eq!(bitmap.pack_checksum, vec![0xaa; 20]);
        assert_eq!(bitmap.file_checksum, vec![0xbb; 20]);
        assert_eq!(bitmap.raw_data.len(), data.len());

        // Type indexes from the test builder: 1 commit, 63 blobs
        assert_eq!(bitmap.commits_bitmap.count_set_bits(), 1);
        assert_eq!(bitmap.trees_bitmap.count_set_bits(), 0);
        assert_eq!(bitmap.blobs_bitmap.count_set_bits(), 63);
        assert_eq!(bitmap.tags_bitmap.count_set_bits(), 0);

        // Each entry bitmap has two set bits
        assert_eq!(bitmap.entries[0].object_pos, 0);
        assert_eq!(bitmap.entries[2].object_pos, 2);
        assert_eq!(bitmap.entries[0].bitmap.count_set_bits(), 2);
        assert_eq!(bitmap.entries[0].flags, 1);
    }

    #[test]
    fn test_parse_sha256_bitmap() {
        let data = create_test_bitmap_data(8, 1, PackBitmap::FLAG_FULL_DAG, 32);
        let (remaining, bitmap) = PackBitmap::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(bitmap.checksum_size, 32);
        assert_eq!(bitmap.hash_function_name(), "SHA-256");
        assert_eq!(bitmap.pack_checksum.len(), 32);
        assert_eq!(bitmap.file_checksum.len(), 32);
    }

    #[test]
    fn test_parse_with_hash_cache() {
        let flags = PackBitmap::FLAG_FULL_DAG | PackBitmap::FLAG_HASH_CACHE;
        let data = create_test_bitmap_data(16, 2, flags, 20);
        let (remaining, bitmap) = PackBitmap::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert!(bitmap.has_hash_cache());
        assert_eq!(bitmap.hash_cache_size, 16 * 4);
        assert_eq!(bitmap.pseudo_merge_size, 0);
    }

    #[test]
    fn test_parse_with_lookup_table() {
        let flags =
            PackBitmap::FLAG_FULL_DAG | PackBitmap::FLAG_HASH_CACHE | PackBitmap::FLAG_LOOKUP_TABLE;
        let data = create_test_bitmap_data(16, 2, flags, 20);
        let (remaining, bitmap) = PackBitmap::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert!(bitmap.has_lookup_table());
        let table = bitmap.lookup_table.as_ref().unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(table[0].commit_pos, 0);
        assert_eq!(table[0].offset, 100);
        assert_eq!(table[0].xor_row, LookupTableEntry::NO_XOR_ROW);
        assert_eq!(table[1].offset, 101);
    }

    #[test]
    fn test_empty_bitmap() {
        // A bitmap can cover a pack while selecting zero commits
        let data = create_test_bitmap_data(4, 0, PackBitmap::FLAG_FULL_DAG, 20);
        let (remaining, bitmap) = PackBitmap::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(bitmap.entry_count(), 0);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = create_test_bitmap_data(4, 1, PackBitmap::FLAG_FULL_DAG, 20);
        data[0] = 0x00;

        assert!(PackBitmap::parse(&data).is_err());
    }

    #[test]
    fn test_invalid_version() {
        let mut data = create_test_bitmap_data(4, 1, PackBitmap::FLAG_FULL_DAG, 20);
        data[5] = 0x02; // version 2

        assert!(PackBitmap::parse(&data).is_err());
    }

    #[test]
    fn test_missing_full_dag_flag() {
        // FULL_DAG is required by git; a file without it must be rejected
        let data = create_test_bitmap_data(4, 1, PackBitmap::FLAG_HASH_CACHE, 20);

        assert!(PackBitmap::parse(&data).is_err());
    }

    #[test]
    fn test_truncated_file() {
        let data = create_test_bitmap_data(64, 3, PackBitmap::FLAG_FULL_DAG, 20);

        assert!(PackBitmap::parse(&data[..data.len() - 25]).is_err());
    }

    #[test]
    fn test_trailing_garbage_rejected() {
        // Extra bytes between the entries and the checksum only pass when the
        // pseudo-merge flag says a variable-length section lives there
        let mut data = create_test_bitmap_data(4, 1, PackBitmap::FLAG_FULL_DAG, 20);
        let insert_at = data.len() - 20;
        data.splice(insert_at..insert_at, [0u8; 8]);

        assert!(PackBitmap::parse(&data).is_err());
    }

    #[test]
    fn test_ewah_run_length_counting() {
        // RLW with a run of 2 all-ones words (128 bits) plus 1 literal word
        let rlw: u64 = (1 << 33) | (2 << 1) | 1;
        let mut data = Vec::new();
        data.extend_from_slice(&192u32.to_be_bytes()); // bit count
        data.extend_from_slice(&2u32.to_be_bytes()); // word count
        data.extend_from_slice(&rlw.to_be_bytes());
        data.extend_from_slice(&0xff00_0000_0000_0000u64.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes()); // RLW position

        let (remaining, ewah) = EwahBitmap::parse(&data).unwrap();
        assert!(remaining.is_empty());
        assert_eq!(ewah.count_set_bits(), 128 + 8);
        assert_eq!(ewah.compressed_byte_size(), 12 + 16);
    }

    #[test]
    fn test_display_format() {
        let data = create_test_bitmap_data(64, 3, PackBitmap::FLAG_FULL_DAG, 20);
        let (_, bitmap) = PackBitmap::parse(&data).unwrap();

        let display_str = format!("{bitmap}");
        assert!(display_str.contains("Pack Bitmap (version 1)"));
        assert!(display_str.contains("FULL_DAG"));
        assert!(display_str.contains("Bitmapped commits: 3"));
        assert!(display_str.contains("Objects covered: 64"));
    }
}
