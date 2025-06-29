use std::fmt::Write;

use crate::tui::helpers::render_styled_paragraph_with_scrollbar;
use crate::tui::model::PackObject;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text, ToText};
use std::fmt;

#[derive(Debug, Clone)]
pub enum PackObjectWidget {
    Uninitiolized,
    Initiolized {
        pack_obj: PackObject,
        scroll_position: usize,
        max_scroll: usize,
        text_cache: Option<ratatui::text::Text<'static>>,
    },
}

impl PackObjectWidget {
    pub fn new(pack_obj: PackObject) -> Self {
        Self::Initiolized {
            pack_obj,
            scroll_position: 0,
            max_scroll: 0,
            text_cache: None,
        }
    }

    pub fn empty() -> Self {
        Self::Uninitiolized
    }

    pub fn text(&mut self) -> ratatui::text::Text<'static> {
        match self {
            &mut Self::Initiolized {
                ref pack_obj,
                ref mut text_cache,
                ..
            } => {
                if let Some(cached_content) = text_cache {
                    return cached_content.clone();
                }
                let content = generate_pack_object_detail_content(pack_obj);
                *text_cache = Some(content.clone());
                content
            }
            Self::Uninitiolized => "Initializing Pack Object Preview ...".to_text(),
        }
    }

    pub fn scroll_up(&mut self) {
        match self {
            Self::Initiolized {
                scroll_position, ..
            } => {
                if *scroll_position > 0 {
                    *scroll_position -= 1;
                }
            }
            Self::Uninitiolized => {}
        }
    }

    pub fn scroll_down(&mut self) {
        match self {
            &mut Self::Initiolized {
                ref mut scroll_position,
                ref max_scroll,
                ..
            } => {
                if *scroll_position < *max_scroll {
                    *scroll_position += 1;
                }
            }
            Self::Uninitiolized => {}
        }
    }

    pub fn scroll_to_top(&mut self) {
        match self {
            Self::Initiolized {
                scroll_position, ..
            } => *scroll_position = 0,
            Self::Uninitiolized => {}
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        match self {
            Self::Initiolized {
                scroll_position,
                max_scroll,
                ..
            } => {
                *scroll_position = *max_scroll;
            }
            Self::Uninitiolized => {}
        }
    }

    fn scroll_position(&self) -> usize {
        match self {
            Self::Initiolized {
                scroll_position, ..
            } => *scroll_position,
            Self::Uninitiolized => 0,
        }
    }

    pub fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
    ) {
        let content = self.text();

        match self {
            Self::Initiolized { max_scroll, .. } => {
                let total_lines = content.lines.len();
                let visible_height = area.height as usize - 2; // Account for borders
                *max_scroll = total_lines.saturating_sub(visible_height);
            }
            Self::Uninitiolized => {}
        }

        let title = "Pack object Details";
        render_styled_paragraph_with_scrollbar(
            f,
            area,
            content,
            self.scroll_position(),
            title,
            is_focused,
        );
    }
}

// Helper function to calculate the number of bytes used for size encoding
fn calculate_size_byte_count(obj_type: crate::git::pack::ObjectType, raw_data: &[u8]) -> usize {
    match obj_type {
        crate::git::pack::ObjectType::RefDelta => {
            // RefDelta: size bytes + 20-byte hash
            raw_data.len() - 20
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum HeaderSection {
    Size,
    Hash,
    Offset,
}

impl fmt::Display for HeaderSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeaderSection::Size => write!(f, "size"),
            HeaderSection::Hash => write!(f, "hash"),
            HeaderSection::Offset => write!(f, "offset"),
        }
    }
}

impl HeaderSection {
    fn from_byte_position(
        byte_index: usize,
        obj_type: crate::git::pack::ObjectType,
        raw_data_len: usize,
        size_byte_count: usize,
    ) -> Self {
        let is_ref_delta_hash = obj_type == crate::git::pack::ObjectType::RefDelta
            && raw_data_len >= 20
            && byte_index >= raw_data_len - 20;

        let is_ofs_delta_offset =
            obj_type == crate::git::pack::ObjectType::OfsDelta && byte_index >= size_byte_count;

        if is_ref_delta_hash {
            HeaderSection::Hash
        } else if is_ofs_delta_offset {
            HeaderSection::Offset
        } else {
            HeaderSection::Size
        }
    }
}

fn generate_pack_object_detail_content(pack_obj: &PackObject) -> Text<'static> {
    let mut detail = String::new();
    let mut lines: Vec<Line> = Vec::new();
    let colors = [Color::Blue, Color::Magenta, Color::Cyan, Color::Red];
    let mut colored_hash: Vec<Span> = Vec::new();

    // If we have object data, show detailed header information
    if let Some(ref object_data) = pack_obj.object_data {
        let header = &object_data.header;
        let raw_data = header.raw_data();

        // Show basic header info first

        lines.push(Line::styled(
            "OBJECT HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        if !raw_data.is_empty() {
            let mut length_parts = Vec::new();
            // Show hex dump of raw header data
            // "Byte {}\n: 0x{:02x} ({:08b}) = {}",
            // Show detailed byte-by-byte breakdown
            for (i, &byte) in raw_data.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(format!("Byte 1")));
                    let mut continuation_line: Vec<Span> = Vec::new();
                    continuation_line
                        .push(Span::styled("   ╭─ ", Style::default().fg(Color::Green)));
                    continuation_line.push(Span::from(format!(
                        "Continuation bit: {}",
                        if byte & 0x80 != 0 {
                            "1 (there is a following size byte)"
                        } else {
                            "0 (the last size byte)"
                        }
                    )));
                    lines.push(Line::from(continuation_line));
                    let mut byte_line: Vec<Span> = Vec::new();
                    byte_line.push(Span::styled(
                        "  |",
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                    byte_line.push(Span::styled(
                        format!("{:b}", (byte >> 7) & 0x1),
                        Style::default().fg(Color::Green),
                    ));
                    byte_line.push(Span::styled(
                        format!("{:03b}", (byte >> 4) & 0x7),
                        Style::default().fg(Color::Yellow),
                    ));
                    byte_line.push(Span::styled(
                        format!("{:04b}", byte & 0x0F),
                        Style::default().fg(colors[0]),
                    ));
                    length_parts.push(Span::styled(
                        format!("{:04b}", byte & 0x0F),
                        Style::default().fg(colors[0]),
                    ));
                    byte_line.push(Span::from("┊"));
                    lines.push(Line::from(byte_line));

                    let obj_type_bits = (byte >> 4) & 0x7;
                    let size_bits = byte & 0x0F;
                    let size_line = [
                        Span::styled("    ├─╯", Style::default().fg(Color::Yellow)),
                        Span::from(format!(
                            "╰──┴─ Uncompressed size bits: {} (0x{:x})",
                            size_bits, size_bits
                        )),
                    ];
                    lines.push(Line::from(size_line.to_vec()));
                    let obj_type_line = [
                        Span::styled("    ╰─", Style::default().fg(Color::Yellow)),
                        Span::from(format!(" Object type: {} ─ ", obj_type_bits)),
                        Span::styled(
                            format!("{:?}", header.obj_type()),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ];
                    lines.push(Line::from(obj_type_line.to_vec()));
                } else {
                    // Determine what this byte represents based on object type and position
                    let obj_type = header.obj_type();
                    let size_byte_count = calculate_size_byte_count(obj_type, raw_data);

                    // Determine current and previous byte's logical section types
                    let current_section = HeaderSection::from_byte_position(
                        i,
                        obj_type,
                        raw_data.len(),
                        size_byte_count,
                    );
                    let prev_section = HeaderSection::from_byte_position(
                        i - 1,
                        obj_type,
                        raw_data.len(),
                        size_byte_count,
                    );

                    // Use solid separator when transitioning between different sections
                    let is_section_transition = current_section != prev_section;

                    if current_section == HeaderSection::Hash {
                        if is_section_transition {
                            lines.push(Line::from("          ╭──────┬─ Reference hash bytes"));
                        }
                        // Byte 2   ┊00000010|
                        //           ╰──────┴─ hash byte: 2 (0x02)
                        let mut byte_line: Vec<Span> = Vec::new();
                        byte_line.push(Span::from(format!("Byte {:2}", i + 1)));
                        if is_section_transition {
                            byte_line.push(Span::styled(
                                "  |",
                                Style::default().add_modifier(Modifier::BOLD),
                            ))
                        } else {
                            byte_line.push(Span::from("  ┊"))
                        };
                        byte_line.push(Span::styled(
                            format!("{:08b}", byte),
                            colors[i % colors.len()],
                        ));
                        byte_line.push(Span::from("┊"));
                        byte_line.push(Span::from(" - 0x"));
                        byte_line.push(Span::styled(
                            format!("{:02X}", byte),
                            colors[i % colors.len()],
                        ));
                        colored_hash.push(Span::styled(
                            format!("{:02X}", byte),
                            colors[i % colors.len()],
                        ));
                        lines.push(Line::from(byte_line));
                    } else {
                        lines.push(Line::from(format!("Byte {}", i + 1)));
                        let mut continuation_line: Vec<Span> = Vec::new();
                        continuation_line
                            .push(Span::styled("   ╭─ ", Style::default().fg(Color::Green)));
                        continuation_line.push(Span::from(format!(
                            "Continuation bit: {}",
                            if byte & 0x80 != 0 {
                                format!("1 (there is a following {} byte)", current_section)
                            } else {
                                format!("0 (the last {} byte)", current_section)
                            }
                        )));
                        lines.push(Line::from(continuation_line));
                        let front_separator = if is_section_transition {
                            Span::styled("  |", Style::default().add_modifier(Modifier::BOLD))
                        } else {
                            Span::from("  ┊")
                        };
                        let back_separator = if byte & 0x80 != 0 {
                            Span::from("┊")
                        } else {
                            Span::styled("|", Style::default().add_modifier(Modifier::BOLD))
                        };
                        let byte_line = [
                            front_separator,
                            Span::styled(
                                format!("{:b}", (byte >> 7) & 0x1),
                                Style::default().fg(Color::Green),
                            ),
                            Span::styled(format!("{:07b}", byte & 0x7F), colors[i % colors.len()]),
                            back_separator,
                        ];
                        length_parts.push(Span::styled(
                            format!("{:07b}", byte & 0x7F),
                            colors[i % colors.len()],
                        ));
                        lines.push(Line::from(byte_line.to_vec()));

                        match current_section {
                            HeaderSection::Offset => {
                                lines.push(Line::from(format!(
                                    "    ╰─────┴─ Base offset bits: {} (0x{:X})",
                                    byte & 0x7F,
                                    byte & 0x7F
                                )));
                            }
                            HeaderSection::Size => {
                                lines.push(Line::from(format!(
                                    "    ╰─────┴─ Uncompressed size bits: {} (0x{:X})",
                                    byte & 0x7F,
                                    byte & 0x7F
                                )));
                            }
                            _ => {}
                        }
                    }
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Summary:"));
            lines.push(Line::from(format!(
                "  - Object type: {}",
                header.obj_type()
            )));
            lines.push(Line::from("  - Uncompressed data size:"));

            // Separate size reconstruction from base reference/offset reconstruction
            let obj_type = header.obj_type();
            let size_byte_count = calculate_size_byte_count(obj_type, raw_data);

            // Create reconstruction line with byte separators and colors indicating source byte
            let mut reconstruction_line = vec![Span::from("      ")];

            // Concatenate only the size bits and track their source colors
            // Note: Git uses little-endian bit order for variable-length encoding,
            // so we need to reverse the order of bytes when reconstructing
            let mut all_bits = String::new();
            let mut bit_colors = Vec::new();

            // Reverse the order: later bytes contribute higher-order bits
            for (byte_idx, part) in length_parts
                .iter()
                .enumerate()
                .take(size_byte_count)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                let color = colors[byte_idx % colors.len()];
                let bits = part.content.as_ref();
                all_bits.push_str(bits);
                // Track color for each bit
                for _ in 0..bits.len() {
                    bit_colors.push(color);
                }
            }

            // Split into 8-bit groups (bytes) with separators
            let mut bit_pos = 0;
            let mut byte_count = 0;
            while bit_pos < all_bits.len() {
                if byte_count > 0 {
                    reconstruction_line.push(Span::styled("|", Style::default().fg(Color::Gray)));
                }

                // Take up to 8 bits for this byte, but if it's the first group and we have more than 8 bits total,
                // take only the remaining bits that don't fit evenly into 8-bit groups
                let remaining_bits = all_bits.len() - bit_pos;
                let bits_in_this_group = if byte_count == 0 && remaining_bits > 8 {
                    remaining_bits % 8
                } else {
                    8.min(remaining_bits)
                };

                // If first group would be 0 bits, take 8 instead
                let bits_in_this_group = if bits_in_this_group == 0 {
                    8
                } else {
                    bits_in_this_group
                };

                let end_pos = bit_pos + bits_in_this_group;
                let byte_bits = &all_bits[bit_pos..end_pos];

                // Create spans for each bit with its original color
                for (i, bit_char) in byte_bits.chars().enumerate() {
                    let color = bit_colors[bit_pos + i];
                    reconstruction_line.push(Span::styled(
                        bit_char.to_string(),
                        Style::default().fg(color),
                    ));
                }

                bit_pos = end_pos;
                byte_count += 1;
            }
            reconstruction_line.push(Span::from(format!(
                " — 0x{:X}",
                header.uncompressed_data_size()
            )));
            lines.push(Line::from(reconstruction_line));
            lines.push(Line::from(format!(
                "      Result: {} bytes",
                header.uncompressed_data_size()
            )));

            // Show base reference/offset reconstruction for delta objects
            if obj_type == crate::git::pack::ObjectType::RefDelta {
                lines.push(Line::from("  - Base object hash (20 bytes):"));
                colored_hash.insert(0, Span::from("      "));
                lines.push(Line::from(colored_hash));
            } else if obj_type == crate::git::pack::ObjectType::OfsDelta {
                lines.push(Line::from("  - Base offset:"));
                let offset_bytes = &raw_data[size_byte_count..];

                // Collect offset bits (excluding continuation bits) in correct order
                // Note: Git uses big-endian bit order for offset encoding
                // Earlier bytes contribute higher-order bits, so we use natural order
                let mut offset_parts = Vec::new();

                for (i, &byte) in offset_bytes.iter().enumerate() {
                    let color = colors[(size_byte_count + i) % colors.len()];
                    let payload_bits = format!("{:07b}", byte & 0x7F);
                    offset_parts.push((payload_bits, color));
                }

                let mut offset_bits = String::new();
                let mut offset_bit_colors = Vec::new();

                // Use natural order: earlier bytes contribute higher-order bits
                for (payload_bits, color) in offset_parts.into_iter() {
                    offset_bits.push_str(&payload_bits);
                    // Track color for each bit
                    for _ in 0..7 {
                        offset_bit_colors.push(color);
                    }
                }

                // Display offset reconstruction
                let mut offset_reconstruction_line = vec![Span::from("      ")];
                let mut bit_pos = 0;
                let mut byte_count = 0;

                while bit_pos < offset_bits.len() {
                    if byte_count > 0 {
                        offset_reconstruction_line
                            .push(Span::styled("|", Style::default().fg(Color::Gray)));
                    }

                    let remaining_bits = offset_bits.len() - bit_pos;
                    let bits_in_this_group = if byte_count == 0 && remaining_bits > 8 {
                        remaining_bits % 8
                    } else {
                        8.min(remaining_bits)
                    };

                    let bits_in_this_group = if bits_in_this_group == 0 {
                        8
                    } else {
                        bits_in_this_group
                    };

                    let end_pos = bit_pos + bits_in_this_group;
                    let byte_bits = &offset_bits[bit_pos..end_pos];

                    // Create spans for each bit with its original color
                    for (i, bit_char) in byte_bits.chars().enumerate() {
                        let color = offset_bit_colors[bit_pos + i];
                        offset_reconstruction_line.push(Span::styled(
                            bit_char.to_string(),
                            Style::default().fg(color),
                        ));
                    }

                    bit_pos = end_pos;
                    byte_count += 1;
                }

                if let crate::git::pack::ObjectHeader::OfsDelta { base_offset, .. } = header {
                    offset_reconstruction_line.push(Span::from(format!(" — 0x{:X}", base_offset)));
                    lines.push(Line::from(offset_reconstruction_line));
                    lines.push(Line::from(format!(
                        "      Result: {} bytes back",
                        base_offset
                    )));
                }
            }
            lines.push(Line::from("Calculated values:"));
            lines.push(Line::from(format!(
                "  - Header Length: {} bytes",
                raw_data.len()
            )));
        }
    } else {
        // Fallback for basic object info
        lines.push(Line::from(format!("Object Type: {}", pack_obj.obj_type)));
        lines.push(Line::from(format!(
            "Uncompressed Size: {} bytes",
            pack_obj.size
        )));
        if let Some(ref base_info) = pack_obj.base_info {
            lines.push(Line::from(format!("Base Info: {base_info}")));
        }
    }

    // If we have object data, show detailed information
    if let Some(ref object_data) = pack_obj.object_data {
        lines.push(Line::from("\n".to_string()));

        let obj_type = object_data.header.obj_type();

        // Enhanced visualization for delta objects
        if obj_type == crate::git::pack::ObjectType::OfsDelta
            || obj_type == crate::git::pack::ObjectType::RefDelta
        {
            // Parse and display delta instructions
            match crate::git::pack::parse_delta_instructions(&object_data.uncompressed_data) {
                Ok((_, instructions)) => {
                    lines.push(Line::from("DELTA INSTRUCTIONS\n"));
                    lines.push(Line::from("─".repeat(30)));
                    lines.push(Line::from("\n\n"));

                    lines.push(Line::from(format!(
                        "Total Instructions: {}\n",
                        instructions.len()
                    )));
                    lines.push(Line::from("\n".to_string()));

                    for (i, instruction) in instructions.iter().enumerate() {
                        let _ = write!(detail, "  {}. ", i + 1);
                        match instruction {
                            crate::git::pack::DeltaInstruction::Copy { offset, size } => {
                                lines.push(Line::from(format!(
                                    "COPY: offset={offset} (0x{offset:x}), size={size} bytes"
                                )));
                                lines.push(Line::from(format!("      └─ Copy {size} bytes from base object starting at offset {offset}")));
                            }
                            crate::git::pack::DeltaInstruction::Insert { data } => {
                                lines.push(Line::from(format!("INSERT: {} bytes", data.len())));

                                // Show first few bytes of data
                                if data.len() <= 64 {
                                    let data_str = String::from_utf8_lossy(data);
                                    if data_str.chars().all(|c| {
                                        c.is_ascii() && !c.is_control() || c == '\n' || c == '\t'
                                    }) {
                                        lines
                                            .push(Line::from(format!("      └─ Data: {data_str}")));
                                    } else {
                                        lines.push(Line::from(format!(
                                            "      └─ Hex: {}",
                                            hex::encode(&data[..data.len().min(32)])
                                        )));
                                        if data.len() > 32 {
                                            lines.push(Line::from(
                                                "      └─ ... (truncated)\n".to_string(),
                                            ));
                                        }
                                    }
                                } else {
                                    // For larger data, show hex dump of first 32 bytes
                                    lines.push(Line::from(format!(
                                        "      └─ Hex (first 32 bytes): {}",
                                        hex::encode(&data[..32])
                                    )));
                                    lines.push(Line::from(format!(
                                        "      └─ ... and {} more bytes",
                                        data.len() - 32
                                    )));
                                }
                            }
                        }
                        lines.push(Line::from("\n".to_string()));
                    }

                    // Delta reconstruction explanation
                    lines.push(Line::from("\nDELTA RECONSTRUCTION PROCESS\n"));
                    lines.push(Line::from("─".repeat(35)));
                    lines.push(Line::from("\n\n"));
                    lines.push(Line::from("To reconstruct the original object:\n"));
                    lines.push(Line::from("1. Start with the base object\n"));
                    lines.push(Line::from("2. Apply each instruction in sequence:\n"));
                    lines.push(Line::from("   • COPY: Copy bytes from base object\n"));
                    lines.push(Line::from("   • INSERT: Add new bytes to output\n"));
                    lines.push(Line::from("3. The result is the reconstructed object\n\n"));

                    lines.push(Line::from("This delta compression allows Git to store\n"));
                    lines.push(Line::from("similar objects very efficiently by only\n"));
                    lines.push(Line::from("storing the differences.\n"));
                }
                Err(e) => {
                    lines.push(Line::from(format!(
                        "Error parsing delta instructions: {e:?}"
                    )));
                }
            }
        } else {
            // Regular object - show content preview
            lines.push(Line::styled(
                "OBJECT DATA",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from("─".repeat(30)));
            lines.push(Line::from("gzip compressed data"));
            lines.push(Line::from(""));
            lines.push(Line::from("Calculated values:"));
            lines.push(Line::from(format!(
                "  - Compressed data size: {} bytes",
                object_data.compressed_size
            )));
            lines.push(Line::from(format!(
                "  - Compression ratio: {:.1}%",
                (object_data.compressed_size as f64 / object_data.uncompressed_data.len() as f64)
                    * 100.0
            )));
            if let Some(ref sha1) = pack_obj.sha1 {
                lines.push(Line::from(format!("  - SHA-1: {}", sha1)));
            }

            let content_str = String::from_utf8_lossy(&object_data.uncompressed_data);
            if content_str.len() <= 1000 {
                lines.push(Line::from("Content:"));
                lines.push(Line::from(""));
                lines.push(Line::from(content_str.to_string()));
            } else {
                lines.push(Line::from(format!(
                    "Content (first 1000 chars):\n{}",
                    &content_str[..1000]
                )));
                lines.push(Line::from("... (truncated)\n".to_string()));
            }
        }
    } else {
        lines.push(Line::from("BASIC OBJECT INFO\n"));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from("\n\n"));
        lines.push(Line::from(
            "This object is stored compressed within the pack file.\n",
        ));
        lines.push(Line::from(
            "To view the actual content, use git cat-file or similar tools.\n\n",
        ));
        lines.push(Line::from("Pack objects can be:\n"));
        lines.push(Line::from("• Blob: File contents\n"));
        lines.push(Line::from("• Tree: Directory structure\n"));
        lines.push(Line::from("• Commit: Commit information\n"));
        lines.push(Line::from("• Tag: Annotated tag\n"));
        lines.push(Line::from("• OFS Delta: Delta relative to offset\n"));
        lines.push(Line::from("• REF Delta: Delta relative to reference\n"));
    }

    Text::from(lines)
}
