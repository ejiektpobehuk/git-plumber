use flate2::read::ZlibDecoder;
use nom::{
    IResult,
    error::{Error, ErrorKind},
};
use std::fmt;
use std::io::Read;

use crate::git::pack::PackError;
use crate::git::pack::delta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Invalid = 0,
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
    Reserved = 5,
    OfsDelta = 6,
    RefDelta = 7,
}

impl TryFrom<u8> for ObjectType {
    type Error = PackError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ObjectType::Invalid),
            1 => Ok(ObjectType::Commit),
            2 => Ok(ObjectType::Tree),
            3 => Ok(ObjectType::Blob),
            4 => Ok(ObjectType::Tag),
            5 => Ok(ObjectType::Reserved),
            6 => Ok(ObjectType::OfsDelta),
            7 => Ok(ObjectType::RefDelta),
            _ => Err(PackError::InvalidObjectType(value)),
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Invalid => "invalid",
                Self::Commit => "commit",
                Self::Tree => "tree",
                Self::Blob => "blob",
                Self::Tag => "tag",
                Self::Reserved => "reserved",
                Self::OfsDelta => "ofs_delta",
                Self::RefDelta => "ref_delta",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectHeader {
    Regular {
        obj_type: ObjectType, // Commit, Tree, Blob, Tag
        uncompressed_data_size: usize,
        raw_data: Vec<u8>,
    },
    OfsDelta {
        uncompressed_data_size: usize,
        base_offset: i64,
        raw_data: Vec<u8>,
    },
    RefDelta {
        uncompressed_data_size: usize,
        base_ref: [u8; 20],
        raw_data: Vec<u8>,
    },
}

impl ObjectHeader {
    // Helper method to get the object type
    pub fn obj_type(&self) -> ObjectType {
        match self {
            Self::Regular { obj_type, .. } => *obj_type,
            Self::OfsDelta { .. } => ObjectType::OfsDelta,
            Self::RefDelta { .. } => ObjectType::RefDelta,
        }
    }

    // Helper method to get the uncompressed size
    pub fn uncompressed_data_size(&self) -> usize {
        match self {
            Self::Regular {
                uncompressed_data_size,
                ..
            }
            | Self::OfsDelta {
                uncompressed_data_size,
                ..
            }
            | Self::RefDelta {
                uncompressed_data_size,
                ..
            } => *uncompressed_data_size,
        }
    }

    // Helper method to get the raw header data
    pub fn raw_data(&self) -> &[u8] {
        match self {
            Self::Regular { raw_data, .. }
            | Self::OfsDelta { raw_data, .. }
            | Self::RefDelta { raw_data, .. } => raw_data,
        }
    }

    /// Every byte of the header has its Most Significant Bit used as
    /// a continuation bit:
    /// 0 -> this is the last byte
    /// 1 -> there is the next byte
    ///
    /// After the continuation bit in the first byte there are 3 bits for the type.
    /// Type 5 is reserved for future expansion. Type 0 is invalid.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let original_input = input;
        let mut i = 0;

        // Check if we have at least one byte
        if i >= input.len() {
            return Err(nom::Err::Incomplete(nom::Needed::new(1)));
        }

        // First byte special handling
        let first_byte = input[i];
        i += 1;

        let obj_type: ObjectType = ((first_byte >> 4) & 0x7).try_into().unwrap();
        let mut size = (first_byte & 0x0F) as usize;

        // If MSB is set, we have more bytes for the size
        if first_byte & 0x80 != 0 {
            let mut shift = 4; // We already have 4 bits from the first byte

            // Process additional bytes
            loop {
                if i >= input.len() {
                    return Err(nom::Err::Incomplete(nom::Needed::new(1)));
                }

                let byte = input[i];
                i += 1;

                // Add the 7 least significant bits to our size, shifted appropriately
                size |= ((byte & 0x7F) as usize) << shift;
                shift += 7;

                // If MSB is 0, we're done
                if byte & 0x80 == 0 {
                    break;
                }
            }
        }

        // Handle delta objects
        let header = match obj_type {
            ObjectType::OfsDelta => {
                // Parse variable-length offset encoding (see git packfile format)
                let mut offset: u64 = 0;
                let mut c: u8;
                loop {
                    if i >= input.len() {
                        return Err(nom::Err::Incomplete(nom::Needed::new(1)));
                    }
                    c = input[i];
                    i += 1;
                    offset = (offset << 7) | (u64::from(c) & 0x7F);
                    if c & 0x80 == 0 {
                        break;
                    }
                }
                // Calculate header size and store raw data
                let header_size = i;
                let raw_data = original_input[..header_size].to_vec();

                // The offset is stored as the distance backwards from the current object's header
                Self::OfsDelta {
                    uncompressed_data_size: size,
                    base_offset: offset as i64,
                    raw_data,
                }
            }
            ObjectType::RefDelta => {
                if i + 20 > input.len() {
                    return Err(nom::Err::Incomplete(nom::Needed::new(20)));
                }
                // Read the 20-byte base object SHA-1
                let mut ref_bytes = [0u8; 20];
                ref_bytes.copy_from_slice(&input[i..i + 20]);
                i += 20;

                // Calculate header size and store raw data
                let header_size = i;
                let raw_data = original_input[..header_size].to_vec();

                Self::RefDelta {
                    uncompressed_data_size: size,
                    base_ref: ref_bytes,
                    raw_data,
                }
            }
            _ => {
                // Calculate header size and store raw data for regular objects
                let header_size = i;
                let raw_data = original_input[..header_size].to_vec();

                Self::Regular {
                    obj_type,
                    uncompressed_data_size: size,
                    raw_data,
                }
            }
        };

        Ok((&input[i..], header))
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub header: ObjectHeader,
    pub uncompressed_data: Vec<u8>,
    pub compressed_data: Vec<u8>, // Raw compressed bytes
    pub compressed_size: usize,   // Size of the compressed data
    pub data_offset: usize,       // Where compressed data begins
}

impl Object {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, header) = ObjectHeader::parse(input)?;
        let pre_parse_input_size = input.len();
        let (remaining_input, data) = Self::parse_data(input, header.uncompressed_data_size())?;
        let compressed_size = pre_parse_input_size - remaining_input.len();

        // Store the compressed data bytes (before they were consumed by parse_data)
        let compressed_data = input[..compressed_size].to_vec();

        // If this is a delta object, parse and display the delta instructions
        let obj_type = header.obj_type();
        let uncompressed_data =
            if obj_type == ObjectType::OfsDelta || obj_type == ObjectType::RefDelta {
                delta::parse_delta_object(&data)
            } else {
                data
            };

        Ok((
            remaining_input,
            Self {
                header,
                uncompressed_data,
                compressed_data,
                compressed_size,
                data_offset: 0,
            },
        ))
    }

    /// Parses the compressed object data.
    /// Returns the decompressed data and the remaining input.
    /// The input should start with a zlib header (0x78).
    fn parse_data(input: &[u8], max_size: usize) -> IResult<&[u8], Vec<u8>> {
        // Check for zlib header
        if input.is_empty() || input[0] != 0x78 {
            return Err(nom::Err::Error(Error::new(input, ErrorKind::Tag)));
        }

        // Create a decoder
        let mut decoder = ZlibDecoder::new(input);
        let mut decompressed = Vec::with_capacity(max_size);

        // Read all decompressed data
        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => {
                // Get the number of bytes consumed by the decoder
                let consumed = usize::try_from(decoder.total_in()).unwrap();
                Ok((&input[consumed..], decompressed))
            }
            Err(_) => Err(nom::Err::Error(Error::new(input, ErrorKind::Tag))),
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let obj_type = self.header.obj_type();
        writeln!(f, "Object type: {}", obj_type)?;
        writeln!(f, "Object size: {}", self.header.uncompressed_data_size())?;
        writeln!(f, "Object compressed size: {}", self.compressed_size)?;

        if obj_type == ObjectType::OfsDelta || obj_type == ObjectType::RefDelta {
            if let Ok((_, instructions)) = delta::parse_delta_instructions(&self.uncompressed_data)
            {
                writeln!(f, "Delta instructions:")?;
                for (i, instruction) in instructions.iter().enumerate() {
                    writeln!(f, "  {}. {}", i + 1, instruction)?;
                }
            }
        } else {
            writeln!(
                f,
                "Object data: {:?}",
                String::from_utf8_lossy(&self.uncompressed_data)
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_object_header() {
        // 9    e    0    e    7    8
        // 1001 1110 0000 1110 0111 1000
        // 1 - continuation bit
        // 001 - object type
        // 1110 - less significant part of uncompressed size
        // 0 - continuation bit
        // 111 1000 - more significant part of uncompressed size
        let data = &[0x9e, 0x0e, 0x78];

        let (_, header) = ObjectHeader::parse(data).unwrap();
        match header {
            ObjectHeader::Regular {
                obj_type,
                uncompressed_data_size,
                raw_data,
            } => {
                assert_eq!(obj_type, ObjectType::Commit);
                assert_eq!(uncompressed_data_size, 238);
                // Verify raw data contains the first 2 bytes (header portion)
                assert_eq!(raw_data.len(), 2);
                assert_eq!(raw_data, vec![0x9e, 0x0e]);
            }
            _ => panic!("Expected Regular header variant"),
        }
    }

    #[test]
    fn parse_object_data() {
        // TODO: use a real object
        let test_data = b"Hello, World!";
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(test_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Parse the compressed data
        let (remaining, decompressed) = Object::parse_data(&compressed, test_data.len()).unwrap();

        // Verify the decompressed data matches the original
        assert_eq!(decompressed, test_data);
        // Verify we consumed all the compressed data
        assert!(remaining.is_empty());
    }
}
