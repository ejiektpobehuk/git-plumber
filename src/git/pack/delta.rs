use std::fmt;

#[derive(Debug, Clone)]
pub enum DeltaInstruction {
    Copy { offset: usize, size: usize },
    Insert { data: Vec<u8> },
}

impl fmt::Display for DeltaInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Copy { offset, size } => {
                write!(f, "Copy: offset={offset}, size={size}")
            }
            Self::Insert { data } => {
                write!(f, "Insert: {} bytes", data.len())?;
                if data.len() <= 100 {
                    write!(f, " (Data: {:?})", String::from_utf8_lossy(data))
                } else {
                    write!(f, " (Data: {:?}...)", String::from_utf8_lossy(&data[..100]))
                }
            }
        }
    }
}

pub fn parse_delta_instructions(input: &[u8]) -> nom::IResult<&[u8], Vec<DeltaInstruction>> {
    let mut instructions = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let first_byte = input[i];
        i += 1;

        if first_byte == 0 {
            // Reserved instruction
            continue;
        }

        if first_byte & 0x80 != 0 {
            // Copy instruction
            let mut offset = 0;
            let mut size = 0;
            let mut shift = 0;

            // Parse offset bytes
            for bit in 0..4 {
                if first_byte & (1 << bit) != 0 {
                    if i >= input.len() {
                        return Err(nom::Err::Incomplete(nom::Needed::new(1)));
                    }
                    offset |= (input[i] as usize) << shift;
                    i += 1;
                }
                shift += 8;
            }

            // Parse size bytes
            shift = 0;
            for bit in 4..7 {
                if first_byte & (1 << bit) != 0 {
                    if i >= input.len() {
                        return Err(nom::Err::Incomplete(nom::Needed::new(1)));
                    }
                    size |= (input[i] as usize) << shift;
                    i += 1;
                }
                shift += 8;
            }

            // Handle special case: size 0 means 0x10000
            if size == 0 {
                size = 0x10000;
            }

            instructions.push(DeltaInstruction::Copy { offset, size });
        } else {
            // Insert instruction
            let size = first_byte as usize;
            if size == 0 {
                return Err(nom::Err::Error(nom::error::Error::new(
                    &input[i..],
                    nom::error::ErrorKind::Tag,
                )));
            }
            if i + size > input.len() {
                return Err(nom::Err::Incomplete(nom::Needed::new(size)));
            }
            let data = input[i..i + size].to_vec();
            i += size;
            instructions.push(DeltaInstruction::Insert { data });
        }
    }

    Ok((&input[i..], instructions))
}

#[must_use]
pub fn parse_delta_object(data: &[u8]) -> Vec<u8> {
    let mut i = 0;
    let mut _shift = 0;

    // Parse base object size
    loop {
        if i >= data.len() {
            return data.to_vec(); // fallback: return the data as is
        }
        let byte = data[i];
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        _shift += 7;
    }

    // Reset shift for target size
    _shift = 0;

    // Parse target object size
    loop {
        if i >= data.len() {
            return data.to_vec(); // fallback: return the data as is
        }
        let byte = data[i];
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        _shift += 7;
    }

    // Return just the delta instructions portion
    data[i..].to_vec()
}
