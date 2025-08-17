use nom::{
    IResult, Parser,
    bytes::complete::tag,
    combinator::map,
    error::{Error, ErrorKind},
    number::complete::be_u32,
};

pub mod delta;
pub mod index;
pub mod object;

pub use delta::{DeltaInstruction, parse_delta_instructions};
pub use index::PackIndex;
pub use object::{Object, ObjectHeader, ObjectType};

use thiserror::Error;

#[derive(Debug)]
pub struct Header {
    pub version: u32,
    pub object_count: u32,
    pub raw_data: Vec<u8>,
}

impl Header {
    /// 4-byte signature:
    ///    The signature is: {'P', 'A', 'C', 'K'}
    fn parse_signature(input: &[u8]) -> IResult<&[u8], ()> {
        map(tag("PACK"), |_| ()).parse(input)
    }

    ///    4-byte version number (network byte order):
    /// Git currently accepts version number 2 or 3 but
    ///       generates version 2 only.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let original_input = input;
        let (input, ()) = Self::parse_signature(input)?;
        let (input, version) = be_u32(input)?;

        if version != 2 && version != 3 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        let (input, object_count) = be_u32(input)?;

        // Calculate the header size (12 bytes: 4 for signature + 4 for version + 4 for object count)
        let header_size = original_input.len() - input.len();
        let raw_data = original_input[..header_size].to_vec();

        Ok((
            input,
            Header {
                version,
                object_count,
                raw_data,
            },
        ))
    }
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error("Invalid pack file signature")]
    InvalidSignature,

    #[error("Unsupported pack version: {0}")]
    UnsupportedVersion(u32),

    #[error("Invalid object type: {0}")]
    InvalidObjectType(u8),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Decompression error: {0}")]
    DecompressionError(#[from] std::io::Error),

    // Index-specific errors
    #[error("Invalid index file signature")]
    InvalidIndexSignature,

    #[error("Unsupported index version: {0}")]
    UnsupportedIndexVersion(u32),

    #[error("Corrupt fan-out table")]
    CorruptFanOutTable,

    #[error("Index checksum mismatch")]
    IndexChecksumMismatch,

    #[error("Pack checksum mismatch")]
    PackChecksumMismatch,

    #[error("Object not found in index: {0}")]
    ObjectNotFound(String),

    #[error("Invalid object index: {0}")]
    InvalidObjectIndex(usize),
}

// Helper function to create a simple pack file header with the specified number of objects
#[cfg(test)]
mod tests {
    use super::*;
    fn create_pack_header(object_count: u32) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(b"PACK"); // Signature
        header.extend_from_slice(&[0, 0, 0, 2]); // Version 2
        header.extend_from_slice(&object_count.to_be_bytes()); // Object count
        header
    }

    #[test]
    fn parse_pack_header() {
        let data = create_pack_header(3); // TODO: use a real pack file
        let (_, header) = Header::parse(&data).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.object_count, 3);

        // Verify the raw data contains the expected header bytes
        assert_eq!(header.raw_data.len(), 12); // 4 + 4 + 4 bytes
        assert_eq!(&header.raw_data[0..4], b"PACK"); // Signature
        assert_eq!(&header.raw_data[4..8], &[0, 0, 0, 2]); // Version 2
        assert_eq!(&header.raw_data[8..12], &3u32.to_be_bytes()); // Object count 3
    }
}
