use std::fmt::Write;

use crate::tui::helpers::render_styled_paragraph_with_scrollbar;
use crate::tui::model::PackObject;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text, ToText};

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
        let title = "Pack object Details";
        render_styled_paragraph_with_scrollbar(
            f,
            area,
            self.text(),
            self.scroll_position(),
            title,
            is_focused,
        );
    }
}

fn generate_pack_object_detail_content(pack_obj: &PackObject) -> Text<'static> {
    let mut detail = String::new();
    let mut lines: Vec<Line> = Vec::new();

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
            // Show hex dump of raw header data
            // "Byte {}\n: 0x{:02x} ({:08b}) = {}",
            // Show detailed byte-by-byte breakdown
            for (i, &byte) in raw_data.iter().enumerate() {
                lines.push(Line::from(format!("Byte {i}")));
                let mut continuation_line: Vec<Span> = Vec::new();
                continuation_line.push(Span::styled("   ╭─ ", Style::default().fg(Color::Green)));
                continuation_line.push(Span::from(format!(
                    "Continuation bit: {}",
                    if byte & 0x80 != 0 {
                        "1 (there is a following header byte)"
                    } else {
                        "0 (last byte)"
                    }
                )));
                lines.push(Line::from(continuation_line));
                if i == 0 {
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
                    byte_line.push(Span::from(format!("{:04b}", byte & 0x0F)));
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
                    let byte_line = [
                        Span::from("  ┊"),
                        Span::styled(
                            format!("{:b}", (byte >> 7) & 0x1),
                            Style::default().fg(Color::Green),
                        ),
                        Span::from(format!("{:07b}", byte & 0x7F)),
                        Span::styled("|", Style::default().add_modifier(Modifier::BOLD)),
                    ];
                    lines.push(Line::from(byte_line.to_vec()));

                    // For RefDelta, the last 20 bytes are the hash, so don't analyze them as size bytes
                    let is_ref_delta_hash = header.obj_type()
                        == crate::git::pack::ObjectType::RefDelta
                        && raw_data.len() >= 20
                        && i >= raw_data.len() - 20;

                    if !is_ref_delta_hash {
                        lines.push(Line::from(format!(
                            "    ╰─────┴─ Uncompressed size bits: {} (0x{:x})",
                            byte & 0x7F,
                            byte & 0x7F
                        )));
                    } else {
                        lines.push(Line::from("    └─ Part of base object hash".to_string()));
                    }
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Summary:"));
            lines.push(Line::from(format!(
                "  - Object type: {}",
                header.obj_type()
            )));
            lines.push(Line::from(format!(
                "  - Uncompressed data size: {} bytes",
                header.uncompressed_data_size()
            )));
            match header {
                crate::git::pack::ObjectHeader::OfsDelta { base_offset, .. } => {
                    lines.push(Line::from(format!(
                        "  - Base Offset: {} (0x{:x})",
                        base_offset, base_offset
                    )));
                }
                crate::git::pack::ObjectHeader::RefDelta { base_ref, .. } => {
                    lines.push(Line::from(format!(
                        "  - Base Reference: {}",
                        hex::encode(base_ref)
                    )));
                }
                _ => {}
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
            lines.push(Line::from("DELTA OBJECT ANALYSIS\n"));
            lines.push(Line::from("─".repeat(40)));
            lines.push(Line::from("\n\n"));

            if obj_type == crate::git::pack::ObjectType::OfsDelta {
                if let crate::git::pack::ObjectHeader::OfsDelta { base_offset, .. } =
                    &object_data.header
                {
                    lines.push(Line::from(format!(
                        "Base Offset: {base_offset} (0x{base_offset:x})"
                    )));
                    lines.push(Line::from(format!(
                        "This object is a delta relative to an object {base_offset} bytes back\n"
                    )));
                    lines.push(Line::from("\n".to_string()));
                }
            } else if let crate::git::pack::ObjectHeader::RefDelta { base_ref, .. } =
                &object_data.header
            {
                lines.push(Line::from(format!(
                    "Base Reference: {}",
                    hex::encode(base_ref)
                )));
                lines.push(Line::from(
                    "This object is a delta relative to the referenced object\n\n".to_string(),
                ));
            }

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
