use nom::{
    IResult, Parser,
    bytes::complete::take,
    error::{Error, ErrorKind},
    multi::count,
    number::complete::{be_u8, be_u32, be_u64},
};
use std::fmt;

/// Size of the fixed multi-pack-index header in bytes
const HEADER_SIZE: usize = 12;
/// Size of a single chunk lookup table entry (4-byte ID + 8-byte offset)
const CHUNK_LOOKUP_ENTRY_SIZE: usize = 12;

/// A single entry of the multi-pack-index chunk lookup table
///
/// The chunk lookup table maps four-character chunk IDs to absolute file
/// offsets. Chunk sizes are not stored on disk; each chunk's size is derived
/// from the offset of the following entry.
#[derive(Debug, Clone)]
pub struct ChunkEntry {
    /// Four-character chunk identifier (e.g. 0x504e414d = "PNAM")
    pub id: u32,
    /// Absolute offset of the chunk from the start of the file
    pub offset: u64,
    /// Chunk size in bytes (next entry's offset minus this entry's offset)
    pub size: u64,
}

impl ChunkEntry {
    /// Get the chunk ID as a four-character string (e.g. "PNAM")
    ///
    /// Non-printable bytes are rendered as '?' so unknown IDs stay displayable.
    #[must_use]
    pub fn id_str(&self) -> String {
        self.id
            .to_be_bytes()
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() {
                    b as char
                } else {
                    '?'
                }
            })
            .collect()
    }

    /// Whether this chunk ID is defined by the multi-pack-index format
    #[must_use]
    pub const fn is_known(&self) -> bool {
        matches!(
            self.id,
            MultiPackIndex::CHUNK_PNAM
                | MultiPackIndex::CHUNK_OIDF
                | MultiPackIndex::CHUNK_OIDL
                | MultiPackIndex::CHUNK_OOFF
                | MultiPackIndex::CHUNK_LOFF
                | MultiPackIndex::CHUNK_RIDX
                | MultiPackIndex::CHUNK_BTMP
                | MultiPackIndex::CHUNK_BASE
        )
    }

    /// Short human-readable description of the chunk's purpose
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self.id {
            MultiPackIndex::CHUNK_PNAM => "Packfile names (required)",
            MultiPackIndex::CHUNK_OIDF => "OID fanout table (required)",
            MultiPackIndex::CHUNK_OIDL => "OID lookup table (required)",
            MultiPackIndex::CHUNK_OOFF => "Object offsets (required)",
            MultiPackIndex::CHUNK_LOFF => "Large offsets (optional)",
            MultiPackIndex::CHUNK_RIDX => "Reverse index (optional)",
            MultiPackIndex::CHUNK_BTMP => "Bitmapped packfiles (optional)",
            MultiPackIndex::CHUNK_BASE => "Base multi-pack-index (incremental)",
            _ => "Unknown chunk",
        }
    }
}

/// Location of a single object: which pack it lives in and where
#[derive(Debug, Clone, Copy)]
pub struct ObjectOffset {
    /// Index into the pack name list (PNAM chunk order)
    pub pack_id: u32,
    /// Raw 4-byte offset; if the MSB is set, the low 31 bits index the
    /// large offset (LOFF) table instead of being a pack offset
    pub raw_offset: u32,
}

impl ObjectOffset {
    /// Whether the offset redirects into the large offset table
    #[must_use]
    pub const fn is_large(&self) -> bool {
        self.raw_offset & 0x8000_0000 != 0
    }

    /// Resolve the actual pack file offset, consulting the large offset
    /// table when needed. Returns None for an out-of-range large offset index.
    #[must_use]
    pub fn resolve(&self, large_offsets: Option<&[u64]>) -> Option<u64> {
        if self.is_large() {
            let index = (self.raw_offset & 0x7fff_ffff) as usize;
            large_offsets?.get(index).copied()
        } else {
            Some(u64::from(self.raw_offset))
        }
    }
}

/// Represents a Git multi-pack-index file (multi-pack-index)
///
/// A multi-pack-index (MIDX) indexes the objects of many pack files at once,
/// mapping each object ID to the pack it lives in and its offset there. This
/// lets Git binary-search one index instead of every per-pack .idx file.
/// The file is chunk-based: a lookup table of four-character chunk IDs and
/// absolute offsets describes where each data table lives.
#[derive(Debug, Clone)]
pub struct MultiPackIndex {
    /// File format version (1 or 2; version 2 allows unsorted pack names)
    pub version: u8,
    /// Hash function identifier (1 for SHA-1, 2 for SHA-256)
    pub hash_function_id: u8,
    /// Number of chunks in the chunk lookup table (excluding the terminator)
    pub chunk_count: u8,
    /// Number of base multi-pack-index files (non-zero only for incremental
    /// chains, which this parser rejects)
    pub base_midx_count: u8,
    /// Number of pack files covered by this multi-pack-index
    pub num_packs: u32,
    /// Chunk lookup table entries (without the terminating entry), with sizes
    /// computed from consecutive offsets
    pub chunks: Vec<ChunkEntry>,
    /// Pack file names from the PNAM chunk, in pack-int-id order
    pub pack_names: Vec<String>,
    /// Fan-out table from the OIDF chunk: entry i counts objects whose first
    /// OID byte is <= i; entry 255 is the total object count
    pub fan_out: [u32; 256],
    /// Object IDs from the OIDL chunk in lexicographic order (hash-length bytes each)
    pub object_ids: Vec<Vec<u8>>,
    /// Per-object pack ID and offset from the OOFF chunk, in OIDL order
    pub object_offsets: Vec<ObjectOffset>,
    /// Large offsets from the LOFF chunk (present only when some pack offset
    /// exceeds 31 bits)
    pub large_offsets: Option<Vec<u64>>,
    /// Reverse index from the RIDX chunk: positions sorted by pseudo-pack order
    pub reverse_index: Option<Vec<u32>>,
    /// SHA-1/SHA-256 checksum of all the above content
    pub checksum: Vec<u8>,
    /// Raw data for debugging/display purposes
    pub raw_data: Vec<u8>,
}

impl MultiPackIndex {
    /// Magic signature for multi-pack-index files: "MIDX"
    pub const SIGNATURE: u32 = 0x4d49_4458;
    /// Supported versions: 1 (default) and 2 (compact form, unsorted pack names)
    pub const VERSIONS: [u8; 2] = [1, 2];

    /// Chunk ID "PNAM": NUL-terminated pack file names
    pub const CHUNK_PNAM: u32 = 0x504e_414d;
    /// Chunk ID "OIDF": 256-entry object ID fanout table
    pub const CHUNK_OIDF: u32 = 0x4f49_4446;
    /// Chunk ID "OIDL": object ID lookup table
    pub const CHUNK_OIDL: u32 = 0x4f49_444c;
    /// Chunk ID "OOFF": object offsets (pack ID + offset pairs)
    pub const CHUNK_OOFF: u32 = 0x4f4f_4646;
    /// Chunk ID "LOFF": 8-byte large offsets
    pub const CHUNK_LOFF: u32 = 0x4c4f_4646;
    /// Chunk ID "RIDX": reverse index (pseudo-pack order)
    pub const CHUNK_RIDX: u32 = 0x5249_4458;
    /// Chunk ID "BTMP": bitmapped packfiles
    pub const CHUNK_BTMP: u32 = 0x4254_4d50;
    /// Chunk ID "BASE": base multi-pack-index references (incremental chains)
    pub const CHUNK_BASE: u32 = 0x4241_5345;

    /// Parse a multi-pack-index file from raw bytes
    ///
    /// # Errors
    ///
    /// Returns a nom parse error if the input is not a valid multi-pack-index:
    /// wrong "MIDX" signature, unsupported version or hash function ID, an
    /// incremental file (non-zero base count), a malformed chunk lookup table,
    /// a missing required chunk, or chunk contents inconsistent with the header.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let original_input = input;

        let (input, (version, hash_function_id, chunk_count, base_midx_count, num_packs)) =
            Self::parse_header(input)?;

        // Chunk lookup table: (chunk_count + 1) entries, the last is a
        // terminator with ID 0 whose offset marks the end of chunk data
        let (_, toc) = count(Self::parse_toc_entry, chunk_count as usize + 1).parse(input)?;

        let Some(&(terminator_id, trailer_offset)) = toc.last() else {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        };
        if terminator_id != 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Offsets must start right after the lookup table, never decrease
        // (a zero-size chunk yields equal consecutive offsets), and stay
        // within the file
        let expected_first_offset =
            (HEADER_SIZE + (chunk_count as usize + 1) * CHUNK_LOOKUP_ENTRY_SIZE) as u64;
        if let Some(&(_, first_offset)) = toc.first()
            && first_offset != expected_first_offset
        {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
        }
        if toc.windows(2).any(|w| w[0].1 > w[1].1) {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Verify)));
        }
        if trailer_offset > original_input.len() as u64 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
        }

        let chunks: Vec<ChunkEntry> = toc
            .windows(2)
            .map(|w| ChunkEntry {
                id: w[0].0,
                offset: w[0].1,
                size: w[1].1 - w[0].1,
            })
            .collect();

        // Slice out each known chunk from the original buffer (offsets are
        // absolute, so sequential parsing doesn't apply past this point)
        let chunk_slice = |id: u32| -> Option<&[u8]> {
            chunks.iter().find(|c| c.id == id).map(|c| {
                let start = c.offset as usize;
                let end = (c.offset + c.size) as usize;
                &original_input[start..end]
            })
        };
        let required_chunk = |id: u32| -> Result<&[u8], nom::Err<Error<&[u8]>>> {
            chunk_slice(id).ok_or(nom::Err::Error(Error::new(original_input, ErrorKind::Tag)))
        };

        let pack_names = Self::parse_pnam(required_chunk(Self::CHUNK_PNAM)?, num_packs)?;
        let fan_out = Self::parse_oidf(required_chunk(Self::CHUNK_OIDF)?)?;
        let total_objects = fan_out[255] as usize;

        let hash_len = match hash_function_id {
            1 => 20, // SHA-1
            2 => 32, // SHA-256
            _ => unreachable!("validated in parse_header"),
        };

        let object_ids =
            Self::parse_oidl(required_chunk(Self::CHUNK_OIDL)?, total_objects, hash_len)?;
        let object_offsets = Self::parse_ooff(required_chunk(Self::CHUNK_OOFF)?, total_objects)?;
        let large_offsets = chunk_slice(Self::CHUNK_LOFF)
            .map(Self::parse_loff)
            .transpose()?;
        let reverse_index = chunk_slice(Self::CHUNK_RIDX)
            .map(|data| Self::parse_ridx(data, total_objects))
            .transpose()?;

        // Every large-offset redirect must land inside the LOFF table
        let large_count = large_offsets.as_ref().map_or(0, Vec::len);
        for object_offset in &object_offsets {
            if object_offset.is_large()
                && (object_offset.raw_offset & 0x7fff_ffff) as usize >= large_count
            {
                return Err(nom::Err::Error(Error::new(
                    original_input,
                    ErrorKind::Verify,
                )));
            }
        }

        // Trailer checksum sits at the terminator offset
        let trailer_start = trailer_offset as usize;
        let (remaining, checksum_bytes) = take(hash_len)(&original_input[trailer_start..])?;
        let checksum = checksum_bytes.to_vec();

        let consumed = original_input.len() - remaining.len();
        let raw_data = original_input[..consumed].to_vec();

        Ok((
            remaining,
            Self {
                version,
                hash_function_id,
                chunk_count,
                base_midx_count,
                num_packs,
                chunks,
                pack_names,
                fan_out,
                object_ids,
                object_offsets,
                large_offsets,
                reverse_index,
                checksum,
                raw_data,
            },
        ))
    }

    /// Parse the 12-byte header (signature, version, hash version, chunk
    /// count, base count, pack count)
    fn parse_header(input: &[u8]) -> IResult<&[u8], (u8, u8, u8, u8, u32)> {
        // Magic signature: "MIDX" (0x4d494458)
        let (input, signature) = be_u32(input)?;
        if signature != Self::SIGNATURE {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Version (1 by default; 2 only relaxes pack name ordering)
        let (input, version) = be_u8(input)?;
        if !Self::VERSIONS.contains(&version) {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Hash function identifier
        let (input, hash_function_id) = be_u8(input)?;
        if hash_function_id != 1 && hash_function_id != 2 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        let (input, chunk_count) = be_u8(input)?;

        // Incremental multi-pack-index chains (multi-pack-index.d/) are not
        // supported; a standalone file always has zero bases
        let (input, base_midx_count) = be_u8(input)?;
        if base_midx_count != 0 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        let (input, num_packs) = be_u32(input)?;

        Ok((
            input,
            (version, hash_function_id, chunk_count, base_midx_count, num_packs),
        ))
    }

    /// Parse one chunk lookup table entry (4-byte ID + 8-byte offset)
    fn parse_toc_entry(input: &[u8]) -> IResult<&[u8], (u32, u64)> {
        let (input, id) = be_u32(input)?;
        let (input, offset) = be_u64(input)?;
        Ok((input, (id, offset)))
    }

    /// Parse the PNAM chunk: NUL-terminated pack names, NUL-padded at the end
    /// to 4-byte alignment
    fn parse_pnam(data: &[u8], num_packs: u32) -> Result<Vec<String>, nom::Err<Error<&[u8]>>> {
        let names: Vec<String> = data
            .split(|&b| b == 0)
            .filter(|name| !name.is_empty())
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect();

        if names.len() != num_packs as usize {
            return Err(nom::Err::Error(Error::new(data, ErrorKind::Verify)));
        }

        Ok(names)
    }

    /// Parse the OIDF chunk: 256-entry monotonic fan-out table
    fn parse_oidf(data: &[u8]) -> Result<[u32; 256], nom::Err<Error<&[u8]>>> {
        let (_, fan_out_vec) = count(be_u32, 256).parse(data)?;

        let mut fan_out = [0u32; 256];
        for (i, &value) in fan_out_vec.iter().enumerate() {
            fan_out[i] = value;
        }

        for i in 1..256 {
            if fan_out[i] < fan_out[i - 1] {
                return Err(nom::Err::Error(Error::new(data, ErrorKind::Verify)));
            }
        }

        Ok(fan_out)
    }

    /// Parse the OIDL chunk: object IDs in lexicographic order
    fn parse_oidl(
        data: &[u8],
        total_objects: usize,
        hash_len: usize,
    ) -> Result<Vec<Vec<u8>>, nom::Err<Error<&[u8]>>> {
        if data.len() != total_objects * hash_len {
            return Err(nom::Err::Error(Error::new(data, ErrorKind::LengthValue)));
        }

        Ok(data.chunks_exact(hash_len).map(<[u8]>::to_vec).collect())
    }

    /// Parse the OOFF chunk: per-object pack ID + offset pairs
    fn parse_ooff(
        data: &[u8],
        total_objects: usize,
    ) -> Result<Vec<ObjectOffset>, nom::Err<Error<&[u8]>>> {
        if data.len() != total_objects * 8 {
            return Err(nom::Err::Error(Error::new(data, ErrorKind::LengthValue)));
        }

        let (_, offsets) = count(
            |input| {
                let (input, pack_id) = be_u32(input)?;
                let (input, raw_offset) = be_u32(input)?;
                Ok((input, ObjectOffset { pack_id, raw_offset }))
            },
            total_objects,
        )
        .parse(data)?;

        Ok(offsets)
    }

    /// Parse the LOFF chunk: 8-byte large offsets
    fn parse_loff(data: &[u8]) -> Result<Vec<u64>, nom::Err<Error<&[u8]>>> {
        if !data.len().is_multiple_of(8) {
            return Err(nom::Err::Error(Error::new(data, ErrorKind::LengthValue)));
        }

        let (_, offsets) = count(be_u64, data.len() / 8).parse(data)?;
        Ok(offsets)
    }

    /// Parse the RIDX chunk: one 4-byte position per object
    fn parse_ridx(
        data: &[u8],
        total_objects: usize,
    ) -> Result<Vec<u32>, nom::Err<Error<&[u8]>>> {
        if data.len() != total_objects * 4 {
            return Err(nom::Err::Error(Error::new(data, ErrorKind::LengthValue)));
        }

        let (_, positions) = count(be_u32, total_objects).parse(data)?;
        Ok(positions)
    }

    /// Get the total number of objects across all indexed packs
    #[must_use]
    pub const fn object_count(&self) -> usize {
        self.object_ids.len()
    }

    /// Get the number of pack files covered by this multi-pack-index
    #[must_use]
    pub const fn pack_count(&self) -> usize {
        self.pack_names.len()
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

    /// Get the hex representation of the object ID at the given lexicographic index
    #[must_use]
    pub fn oid_hex_at(&self, index: usize) -> Option<String> {
        self.object_ids.get(index).map(hex::encode)
    }

    /// Get the pack name for a pack-int-id
    #[must_use]
    pub fn pack_name_for(&self, pack_id: u32) -> Option<&str> {
        self.pack_names.get(pack_id as usize).map(String::as_str)
    }

    /// Get the (pack-int-id, resolved offset) pair for the object at the given index
    #[must_use]
    pub fn offset_at(&self, index: usize) -> Option<(u32, u64)> {
        let object_offset = self.object_offsets.get(index)?;
        let resolved = object_offset.resolve(self.large_offsets.as_deref())?;
        Some((object_offset.pack_id, resolved))
    }

    /// Find a chunk lookup table entry by chunk ID
    #[must_use]
    pub fn chunk_by_id(&self, id: u32) -> Option<&ChunkEntry> {
        self.chunks.iter().find(|c| c.id == id)
    }
}

impl fmt::Display for MultiPackIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Multi-Pack Index (version {})", self.version)?;
        writeln!(
            f,
            "Hash function: {} (ID: {})",
            self.hash_function_name(),
            self.hash_function_id
        )?;
        writeln!(f, "Packs: {}", self.pack_count())?;
        writeln!(f, "Total objects: {}", self.object_count())?;
        writeln!(f, "Checksum: {}", hex::encode(&self.checksum))?;

        writeln!(f, "\nChunks:")?;
        for chunk in &self.chunks {
            writeln!(
                f,
                "  {} at offset {} ({} bytes) — {}",
                chunk.id_str(),
                chunk.offset,
                chunk.size,
                chunk.description()
            )?;
        }

        writeln!(f, "\nPack files:")?;
        let pack_sample = std::cmp::min(10, self.pack_count());
        for (pack_id, name) in self.pack_names.iter().take(pack_sample).enumerate() {
            writeln!(f, "  [{pack_id}] {name}")?;
        }
        if self.pack_count() > pack_sample {
            writeln!(f, "  ... ({} more packs)", self.pack_count() - pack_sample)?;
        }

        writeln!(f, "\nSample objects (OID → pack, offset):")?;
        let object_sample = std::cmp::min(10, self.object_count());
        for index in 0..object_sample {
            if let (Some(oid), Some((pack_id, offset))) =
                (self.oid_hex_at(index), self.offset_at(index))
            {
                let pack_name = self.pack_name_for(pack_id).unwrap_or("<unknown pack>");
                writeln!(f, "  {oid} → {pack_name} @ {offset}")?;
            }
        }
        if self.object_count() > object_sample {
            writeln!(
                f,
                "  ... ({} more objects)",
                self.object_count() - object_sample
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble a multi-pack-index from raw chunk payloads, computing the
    /// chunk lookup table offsets automatically
    fn assemble_midx(
        version: u8,
        hash_function_id: u8,
        base_midx_count: u8,
        num_packs: u32,
        chunks: &[(u32, Vec<u8>)],
        checksum_len: usize,
    ) -> Vec<u8> {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&MultiPackIndex::SIGNATURE.to_be_bytes());
        data.push(version);
        data.push(hash_function_id);
        data.push(chunks.len() as u8);
        data.push(base_midx_count);
        data.extend_from_slice(&num_packs.to_be_bytes());

        // Chunk lookup table ((chunks + 1 terminator) * 12 bytes)
        let mut offset = (HEADER_SIZE + (chunks.len() + 1) * CHUNK_LOOKUP_ENTRY_SIZE) as u64;
        for (id, payload) in chunks {
            data.extend_from_slice(&id.to_be_bytes());
            data.extend_from_slice(&offset.to_be_bytes());
            offset += payload.len() as u64;
        }
        data.extend_from_slice(&0u32.to_be_bytes()); // terminator ID
        data.extend_from_slice(&offset.to_be_bytes()); // trailer offset

        for (_, payload) in chunks {
            data.extend_from_slice(payload);
        }

        // Trailer checksum (dummy)
        data.extend_from_slice(&vec![0xcc; checksum_len]);

        data
    }

    /// Build a NUL-terminated, 4-byte-aligned PNAM payload
    fn build_pnam(pack_names: &[&str]) -> Vec<u8> {
        let mut payload = Vec::new();
        for name in pack_names {
            payload.extend_from_slice(name.as_bytes());
            payload.push(0);
        }
        while payload.len() % 4 != 0 {
            payload.push(0);
        }
        payload
    }

    /// Synthesize OIDs with increasing first bytes and the matching fanout
    fn build_oids(num_objects: usize, hash_len: usize) -> (Vec<u8>, Vec<u8>) {
        assert!(num_objects <= 256, "test builder supports up to 256 objects");

        let mut oidl = Vec::new();
        for i in 0..num_objects {
            let mut oid = vec![0x11u8; hash_len];
            oid[0] = i as u8;
            oidl.extend_from_slice(&oid);
        }

        let mut oidf = Vec::new();
        for first_byte in 0..256 {
            let cumulative = std::cmp::min(first_byte + 1, num_objects) as u32;
            oidf.extend_from_slice(&cumulative.to_be_bytes());
        }

        (oidf, oidl)
    }

    fn create_test_midx_data(
        pack_names: &[&str],
        num_objects: usize,
        hash_function_id: u8,
        with_large_offsets: bool,
    ) -> Vec<u8> {
        let hash_len = if hash_function_id == 2 { 32 } else { 20 };
        let (oidf, oidl) = build_oids(num_objects, hash_len);

        // OOFF: round-robin pack IDs, offsets in 100-byte steps; optionally
        // redirect the last object through the large offset table
        let mut ooff = Vec::new();
        for i in 0..num_objects {
            let pack_id = (i % pack_names.len()) as u32;
            let raw_offset = if with_large_offsets && i == num_objects - 1 {
                0x8000_0000u32 // index 0 into LOFF
            } else {
                (i as u32) * 100
            };
            ooff.extend_from_slice(&pack_id.to_be_bytes());
            ooff.extend_from_slice(&raw_offset.to_be_bytes());
        }

        let mut chunks = vec![
            (MultiPackIndex::CHUNK_PNAM, build_pnam(pack_names)),
            (MultiPackIndex::CHUNK_OIDF, oidf),
            (MultiPackIndex::CHUNK_OIDL, oidl),
            (MultiPackIndex::CHUNK_OOFF, ooff),
        ];
        if with_large_offsets {
            chunks.push((
                MultiPackIndex::CHUNK_LOFF,
                0x1_0000_0000u64.to_be_bytes().to_vec(),
            ));
        }

        assemble_midx(
            1,
            hash_function_id,
            0,
            pack_names.len() as u32,
            &chunks,
            hash_len,
        )
    }

    #[test]
    fn test_parse_header() {
        let data = create_test_midx_data(&["pack-a.pack"], 3, 1, false);
        let (_, (version, hash_function_id, chunk_count, base_midx_count, num_packs)) =
            MultiPackIndex::parse_header(&data).unwrap();

        assert_eq!(version, 1);
        assert_eq!(hash_function_id, 1);
        assert_eq!(chunk_count, 4);
        assert_eq!(base_midx_count, 0);
        assert_eq!(num_packs, 1);
    }

    #[test]
    fn test_parse_complete_midx() {
        let data = create_test_midx_data(&["pack-a.pack", "pack-b.pack"], 5, 1, false);
        let (remaining, midx) = MultiPackIndex::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(midx.version, 1);
        assert_eq!(midx.hash_function_id, 1);
        assert_eq!(midx.hash_function_name(), "SHA-1");
        assert_eq!(midx.checksum_size(), 20);
        assert_eq!(midx.pack_count(), 2);
        assert_eq!(midx.pack_names, vec!["pack-a.pack", "pack-b.pack"]);
        assert_eq!(midx.object_count(), 5);
        assert_eq!(midx.fan_out[255], 5);
        assert_eq!(midx.chunks.len(), 4);
        assert_eq!(midx.checksum, vec![0xcc; 20]);
        assert_eq!(midx.raw_data.len(), data.len());

        // Round-robin pack IDs and 100-byte offset steps from the builder
        assert_eq!(midx.offset_at(0), Some((0, 0)));
        assert_eq!(midx.offset_at(1), Some((1, 100)));
        assert_eq!(midx.offset_at(4), Some((0, 400)));
        assert_eq!(midx.offset_at(5), None); // out of bounds

        // OIDs have increasing first bytes
        assert_eq!(midx.oid_hex_at(0).unwrap()[..2], *"00");
        assert_eq!(midx.oid_hex_at(4).unwrap()[..2], *"04");

        assert_eq!(midx.pack_name_for(1), Some("pack-b.pack"));
        assert_eq!(midx.pack_name_for(2), None);
    }

    #[test]
    fn test_parse_sha256_midx() {
        let data = create_test_midx_data(&["pack-a.pack"], 3, 2, false);
        let (remaining, midx) = MultiPackIndex::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(midx.hash_function_id, 2);
        assert_eq!(midx.hash_function_name(), "SHA-256");
        assert_eq!(midx.checksum_size(), 32);
        assert_eq!(midx.object_ids[0].len(), 32);
        assert_eq!(midx.checksum.len(), 32);
    }

    #[test]
    fn test_version_2_accepted() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 2, 1, false);
        data[4] = 2; // version byte

        let (_, midx) = MultiPackIndex::parse(&data).unwrap();
        assert_eq!(midx.version, 2);
    }

    #[test]
    fn test_large_offsets() {
        let data = create_test_midx_data(&["pack-a.pack"], 3, 1, true);
        let (_, midx) = MultiPackIndex::parse(&data).unwrap();

        assert_eq!(midx.large_offsets, Some(vec![0x1_0000_0000]));
        assert!(midx.object_offsets[2].is_large());
        assert_eq!(midx.offset_at(2), Some((0, 0x1_0000_0000)));
    }

    #[test]
    fn test_large_offset_index_out_of_range() {
        // MSB-flagged offset with no LOFF chunk to redirect into
        let hash_len = 20;
        let (oidf, oidl) = build_oids(1, hash_len);
        let ooff = [0u32.to_be_bytes(), 0x8000_0000u32.to_be_bytes()].concat();

        let chunks = vec![
            (MultiPackIndex::CHUNK_PNAM, build_pnam(&["pack-a.pack"])),
            (MultiPackIndex::CHUNK_OIDF, oidf),
            (MultiPackIndex::CHUNK_OIDL, oidl),
            (MultiPackIndex::CHUNK_OOFF, ooff),
        ];
        let data = assemble_midx(1, 1, 0, 1, &chunks, hash_len);

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_pnam_padding() {
        // "pack-abc.pack" is 13 bytes + NUL = 14, so the chunk carries two
        // padding NULs to reach 4-byte alignment
        let data = create_test_midx_data(&["pack-abc.pack"], 1, 1, false);
        let (_, midx) = MultiPackIndex::parse(&data).unwrap();

        assert_eq!(midx.pack_names, vec!["pack-abc.pack"]);
        let pnam = midx.chunk_by_id(MultiPackIndex::CHUNK_PNAM).unwrap();
        assert_eq!(pnam.size % 4, 0);
    }

    #[test]
    fn test_pnam_count_mismatch() {
        // Header claims two packs, PNAM contains one
        let hash_len = 20;
        let (oidf, oidl) = build_oids(1, hash_len);
        let ooff = [0u32.to_be_bytes(), 0u32.to_be_bytes()].concat();

        let chunks = vec![
            (MultiPackIndex::CHUNK_PNAM, build_pnam(&["pack-a.pack"])),
            (MultiPackIndex::CHUNK_OIDF, oidf),
            (MultiPackIndex::CHUNK_OIDL, oidl),
            (MultiPackIndex::CHUNK_OOFF, ooff),
        ];
        let data = assemble_midx(1, 1, 0, 2, &chunks, hash_len);

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_ridx_parsed() {
        let hash_len = 20;
        let (oidf, oidl) = build_oids(2, hash_len);
        let mut ooff = Vec::new();
        for i in 0..2u32 {
            ooff.extend_from_slice(&0u32.to_be_bytes());
            ooff.extend_from_slice(&(i * 100).to_be_bytes());
        }
        let ridx = [1u32.to_be_bytes(), 0u32.to_be_bytes()].concat();

        let chunks = vec![
            (MultiPackIndex::CHUNK_PNAM, build_pnam(&["pack-a.pack"])),
            (MultiPackIndex::CHUNK_OIDF, oidf),
            (MultiPackIndex::CHUNK_OIDL, oidl),
            (MultiPackIndex::CHUNK_OOFF, ooff),
            (MultiPackIndex::CHUNK_RIDX, ridx),
        ];
        let data = assemble_midx(1, 1, 0, 1, &chunks, hash_len);

        let (_, midx) = MultiPackIndex::parse(&data).unwrap();
        assert_eq!(midx.reverse_index, Some(vec![1, 0]));
    }

    #[test]
    fn test_unknown_chunk_listed_but_not_fatal() {
        let hash_len = 20;
        let (oidf, oidl) = build_oids(1, hash_len);
        let ooff = [0u32.to_be_bytes(), 0u32.to_be_bytes()].concat();

        let unknown_id = 0x5445_5354; // "TEST"
        let chunks = vec![
            (MultiPackIndex::CHUNK_PNAM, build_pnam(&["pack-a.pack"])),
            (MultiPackIndex::CHUNK_OIDF, oidf),
            (MultiPackIndex::CHUNK_OIDL, oidl),
            (MultiPackIndex::CHUNK_OOFF, ooff),
            (unknown_id, vec![0xde, 0xad, 0xbe, 0xef]),
        ];
        let data = assemble_midx(1, 1, 0, 1, &chunks, hash_len);

        let (_, midx) = MultiPackIndex::parse(&data).unwrap();
        let unknown = midx.chunk_by_id(unknown_id).unwrap();
        assert_eq!(unknown.id_str(), "TEST");
        assert!(!unknown.is_known());
        assert_eq!(unknown.size, 4);
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        data[0] = 0x00;

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_invalid_version() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        data[4] = 3;

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_invalid_hash_function() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        data[5] = 3;

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_incremental_rejected() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        data[7] = 1; // base MIDX count

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_missing_required_chunk() {
        // No OOFF chunk
        let hash_len = 20;
        let (oidf, oidl) = build_oids(1, hash_len);

        let chunks = vec![
            (MultiPackIndex::CHUNK_PNAM, build_pnam(&["pack-a.pack"])),
            (MultiPackIndex::CHUNK_OIDF, oidf),
            (MultiPackIndex::CHUNK_OIDL, oidl),
        ];
        let data = assemble_midx(1, 1, 0, 1, &chunks, hash_len);

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_non_monotonic_toc_offsets() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        // Second TOC entry starts at HEADER_SIZE + 12; corrupt its offset
        // (bytes 4..12 of the entry) to go backwards
        let second_entry_offset_pos = HEADER_SIZE + CHUNK_LOOKUP_ENTRY_SIZE + 4;
        data[second_entry_offset_pos..second_entry_offset_pos + 8]
            .copy_from_slice(&1u64.to_be_bytes());

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_toc_offset_past_end_of_file() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        // Corrupt the terminator offset (last TOC entry) to point past EOF
        let terminator_offset_pos = HEADER_SIZE + 4 * CHUNK_LOOKUP_ENTRY_SIZE + 4;
        data[terminator_offset_pos..terminator_offset_pos + 8]
            .copy_from_slice(&(u32::MAX as u64).to_be_bytes());

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_truncated_trailer() {
        let mut data = create_test_midx_data(&["pack-a.pack"], 1, 1, false);
        data.truncate(data.len() - 10); // cut into the checksum

        assert!(MultiPackIndex::parse(&data).is_err());
    }

    #[test]
    fn test_empty_midx() {
        // Zero objects is degenerate but structurally valid
        let data = create_test_midx_data(&["pack-a.pack"], 0, 1, false);
        let (remaining, midx) = MultiPackIndex::parse(&data).unwrap();

        assert!(remaining.is_empty());
        assert_eq!(midx.object_count(), 0);
    }

    #[test]
    fn test_display_format() {
        let data = create_test_midx_data(&["pack-a.pack", "pack-b.pack"], 3, 1, false);
        let (_, midx) = MultiPackIndex::parse(&data).unwrap();

        let display_str = format!("{midx}");
        assert!(display_str.contains("Multi-Pack Index (version 1)"));
        assert!(display_str.contains("Hash function: SHA-1"));
        assert!(display_str.contains("Packs: 2"));
        assert!(display_str.contains("Total objects: 3"));
        assert!(display_str.contains("PNAM"));
        assert!(display_str.contains("pack-b.pack"));
    }
}
