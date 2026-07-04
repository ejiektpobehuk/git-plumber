use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::tui::widget::pack_obj_details::config::{
    COLORS, HeaderSection, calculate_size_byte_count,
};

// Separate header formatting logic
pub struct HeaderFormatter<'a> {
    header: &'a crate::git::pack::ObjectHeader,
}

impl<'a> HeaderFormatter<'a> {
    #[must_use]
    pub const fn new(header: &'a crate::git::pack::ObjectHeader) -> Self {
        Self { header }
    }

    pub fn format_header(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "OBJECT HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let raw_data = self.header.raw_data();
        if raw_data.is_empty() {
            return;
        }

        let (length_parts, colored_hash) = self.format_byte_breakdown(lines, raw_data);
        self.format_header_summary(lines, raw_data, &length_parts, &colored_hash);
    }

    fn format_byte_breakdown(
        &self,
        lines: &mut Vec<Line<'static>>,
        raw_data: &[u8],
    ) -> (Vec<Span<'static>>, Vec<Span<'static>>) {
        let mut length_parts = Vec::new();
        let mut colored_hash = Vec::new();

        for (i, &byte) in raw_data.iter().enumerate() {
            if i == 0 {
                self.format_first_byte(lines, byte, &mut length_parts);
            } else {
                self.format_subsequent_byte(
                    lines,
                    byte,
                    i,
                    raw_data,
                    &mut length_parts,
                    &mut colored_hash,
                );
            }
        }

        (length_parts, colored_hash)
    }

    fn format_first_byte(
        &self,
        lines: &mut Vec<Line<'static>>,
        byte: u8,
        length_parts: &mut Vec<Span<'static>>,
    ) {
        lines.push(Line::from("Byte 1"));

        let continuation_line = vec![
            Span::styled("   ╭─ ", Style::default().fg(Color::Green)),
            Span::from(format!(
                "Continuation bit: {}",
                if byte & 0x80 != 0 {
                    "1 (there is a following size byte)"
                } else {
                    "0 (the last size byte)"
                }
            )),
        ];
        lines.push(Line::from(continuation_line));

        let byte_line = vec![
            Span::styled("  |", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:b}", (byte >> 7) & 0x1),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("{:03b}", (byte >> 4) & 0x7),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("{:04b}", byte & 0x0F),
                Style::default().fg(COLORS[0]),
            ),
            Span::from("┊"),
        ];
        lines.push(Line::from(byte_line));

        length_parts.push(Span::styled(
            format!("{:04b}", byte & 0x0F),
            Style::default().fg(COLORS[0]),
        ));

        let obj_type_bits = (byte >> 4) & 0x7;
        let size_bits = byte & 0x0F;

        let size_line = vec![
            Span::styled("    ├─╯", Style::default().fg(Color::Yellow)),
            Span::from(format!(
                "╰──┴─ Uncompressed size bits: {size_bits} (0x{size_bits:x})"
            )),
        ];
        lines.push(Line::from(size_line));

        let obj_type_line = vec![
            Span::styled("    ╰─", Style::default().fg(Color::Yellow)),
            Span::from(format!(" Object type: {obj_type_bits} ─ ")),
            Span::styled(
                format!("{:?}", self.header.obj_type()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        lines.push(Line::from(obj_type_line));
    }

    fn format_subsequent_byte(
        &self,
        lines: &mut Vec<Line<'static>>,
        byte: u8,
        i: usize,
        raw_data: &[u8],
        length_parts: &mut Vec<Span<'static>>,
        colored_hash: &mut Vec<Span<'static>>,
    ) {
        let obj_type = self.header.obj_type();
        let size_byte_count = calculate_size_byte_count(obj_type, raw_data);

        let current_section =
            HeaderSection::from_byte_position(i, obj_type, raw_data.len(), size_byte_count);
        let prev_section =
            HeaderSection::from_byte_position(i - 1, obj_type, raw_data.len(), size_byte_count);
        let is_section_transition = current_section != prev_section;

        if current_section == HeaderSection::Hash {
            self.format_hash_byte(lines, byte, i, is_section_transition, colored_hash);
        } else {
            self.format_size_or_offset_byte(
                lines,
                byte,
                i,
                is_section_transition,
                current_section,
                length_parts,
            );
        }
    }

    fn format_hash_byte(
        &self,
        lines: &mut Vec<Line<'static>>,
        byte: u8,
        i: usize,
        is_section_transition: bool,
        colored_hash: &mut Vec<Span<'static>>,
    ) {
        if is_section_transition {
            lines.push(Line::from("          ╭──────┬─ Reference hash bytes"));
        }

        let byte_line = vec![
            Span::from(format!("Byte {:2}", i + 1)),
            if is_section_transition {
                Span::styled("  |", Style::default().add_modifier(Modifier::BOLD))
            } else {
                Span::from("  ┊")
            },
            Span::styled(
                format!("{byte:08b}"),
                Style::default().fg(COLORS[i % COLORS.len()]),
            ),
            Span::from("┊"),
            Span::from(" - 0x"),
            Span::styled(
                format!("{byte:02X}"),
                Style::default().fg(COLORS[i % COLORS.len()]),
            ),
        ];
        lines.push(Line::from(byte_line));

        colored_hash.push(Span::styled(
            format!("{byte:02X}"),
            Style::default().fg(COLORS[i % COLORS.len()]),
        ));
    }

    fn format_size_or_offset_byte(
        &self,
        lines: &mut Vec<Line<'static>>,
        byte: u8,
        i: usize,
        is_section_transition: bool,
        current_section: HeaderSection,
        length_parts: &mut Vec<Span<'static>>,
    ) {
        lines.push(Line::from(format!("Byte {}", i + 1)));

        let continuation_line = vec![
            Span::styled("   ╭─ ", Style::default().fg(Color::Green)),
            Span::from(format!(
                "Continuation bit: {}",
                if byte & 0x80 != 0 {
                    format!("1 (there is a following {current_section} byte)")
                } else {
                    format!("0 (the last {current_section} byte)")
                }
            )),
        ];
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

        let byte_line = vec![
            front_separator,
            Span::styled(
                format!("{:b}", (byte >> 7) & 0x1),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("{:07b}", byte & 0x7F),
                Style::default().fg(COLORS[i % COLORS.len()]),
            ),
            back_separator,
        ];
        lines.push(Line::from(byte_line));

        length_parts.push(Span::styled(
            format!("{:07b}", byte & 0x7F),
            Style::default().fg(COLORS[i % COLORS.len()]),
        ));

        let description = match current_section {
            HeaderSection::Offset => {
                format!("Base offset bits: {} (0x{:X})", byte & 0x7F, byte & 0x7F)
            }
            HeaderSection::Size => format!(
                "Uncompressed size bits: {} (0x{:X})",
                byte & 0x7F,
                byte & 0x7F
            ),
            HeaderSection::Hash => unreachable!(),
        };
        lines.push(Line::from(format!("    ╰─────┴─ {description}")));
    }

    fn format_header_summary(
        &self,
        lines: &mut Vec<Line<'static>>,
        raw_data: &[u8],
        length_parts: &[Span<'static>],
        colored_hash: &[Span<'static>],
    ) {
        lines.push(Line::from(""));
        lines.push(Line::from("Summary:"));
        lines.push(Line::from(format!(
            "  - Object type: {}",
            self.header.obj_type()
        )));
        lines.push(Line::from("  - Uncompressed data size:"));

        // Add size reconstruction logic here
        self.format_size_reconstruction(lines, raw_data, length_parts);

        // Add base reference/offset information
        self.format_base_info(lines, raw_data, length_parts, colored_hash);

        lines.push(Line::from("Calculated values:"));
        lines.push(Line::from(format!(
            "  - Header Length: {} bytes",
            raw_data.len()
        )));
    }

    fn format_size_reconstruction(
        &self,
        lines: &mut Vec<Line<'static>>,
        raw_data: &[u8],
        length_parts: &[Span<'static>],
    ) {
        // Separate size reconstruction from base reference/offset reconstruction
        let obj_type = self.header.obj_type();
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
            let color = COLORS[byte_idx % COLORS.len()];
            let bits = part.content.as_ref();
            all_bits.push_str(bits);
            // Track color for each bit
            for _ in 0..bits.len() {
                bit_colors.push(color);
            }
        }

        push_grouped_bits(&mut reconstruction_line, &all_bits, &bit_colors);
        reconstruction_line.push(Span::from(format!(
            " — 0x{:X}",
            self.header.uncompressed_data_size()
        )));
        lines.push(Line::from(reconstruction_line));
        lines.push(Line::from(format!(
            "      Result: {} bytes",
            self.header.uncompressed_data_size()
        )));
    }

    fn format_base_info(
        &self,
        lines: &mut Vec<Line<'static>>,
        raw_data: &[u8],
        length_parts: &[Span<'static>],
        colored_hash: &[Span<'static>],
    ) {
        match self.header {
            crate::git::pack::ObjectHeader::RefDelta { .. } => {
                lines.push(Line::from("  - Base object hash (20 bytes):"));
                let mut hash_line = vec![Span::from("      ")];
                hash_line.extend(colored_hash.iter().cloned());
                lines.push(Line::from(hash_line));
            }
            crate::git::pack::ObjectHeader::OfsDelta { base_offset, .. } => {
                lines.push(Line::from("  - Base offset:"));

                // Offset bytes follow the size bytes in length_parts
                let size_byte_count = calculate_size_byte_count(self.header.obj_type(), raw_data);
                let offset_parts = &length_parts[size_byte_count.min(length_parts.len())..];

                // Unlike the size, the offset is big-endian: bytes concatenate
                // in stream order, no reversal
                let mut all_bits = String::new();
                let mut bit_colors = Vec::new();
                for part in offset_parts {
                    let color = part.style.fg.unwrap_or(Color::White);
                    let bits = part.content.as_ref();
                    all_bits.push_str(bits);
                    for _ in 0..bits.len() {
                        bit_colors.push(color);
                    }
                }

                let mut reconstruction_line = vec![Span::from("      ")];
                push_grouped_bits(&mut reconstruction_line, &all_bits, &bit_colors);

                let concatenated = u128::from_str_radix(&all_bits, 2).unwrap_or(0);
                reconstruction_line.push(Span::from(format!(" — 0x{concatenated:X}")));
                lines.push(Line::from(reconstruction_line));

                // The concatenated bits alone don't give the offset: the
                // encoding adds 1 before each continuation shift, which
                // sums to 2^7 + 2^14 + ... per continuation byte
                let continuation_bytes = offset_parts.len().saturating_sub(1);
                if continuation_bytes > 0 {
                    let bias: u128 = (1..=continuation_bytes as u32)
                        .map(|j| 1u128 << (7 * j))
                        .sum();
                    lines.push(Line::from(format!(
                        "      + 0x{bias:X} (+1 bias per continuation byte)"
                    )));
                }
                lines.push(Line::from(format!(
                    "      Result: {base_offset} bytes back from this object's start"
                )));
            }
            crate::git::pack::ObjectHeader::Regular { .. } => {}
        }
    }
}

// Split concatenated bits into 8-bit groups with separators, keeping each
// bit's source-byte color; a short leading group absorbs the remainder
fn push_grouped_bits(
    reconstruction_line: &mut Vec<Span<'static>>,
    all_bits: &str,
    bit_colors: &[Color],
) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(header_bytes: &[u8]) -> Vec<String> {
        let (_, header) = crate::git::pack::ObjectHeader::parse(header_bytes).unwrap();
        let mut lines = Vec::new();
        HeaderFormatter::new(&header).format_header(&mut lines);
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn ofs_delta_summary_shows_bits_bias_and_result() {
        // Type 6 (ofs-delta), size 5, offset bytes [0x81, 0x00] -> offset 256
        let lines = render(&[0x65, 0x81, 0x00]);

        let bits_line = lines
            .iter()
            .find(|l| l.contains("000000|10000000"))
            .expect("offset bit reconstruction line");
        assert!(bits_line.contains("0x80"), "raw concatenated bits value");
        assert!(
            lines.iter().any(|l| l.contains("+ 0x80")),
            "continuation bias line"
        );
        assert!(
            lines.iter().any(|l| l.contains("Result: 256 bytes back")),
            "decoded offset matches parser"
        );
    }

    #[test]
    fn ofs_delta_single_byte_offset_has_no_bias_line() {
        // Offset fits one byte (0x7F -> 127), so no +1 bias applies
        let lines = render(&[0x65, 0x7F]);

        assert!(!lines.iter().any(|l| l.contains("bias")));
        assert!(lines.iter().any(|l| l.contains("Result: 127 bytes back")));
    }

    #[test]
    fn ref_delta_summary_shows_base_hash() {
        // Type 7 (ref-delta), size 5, followed by a 20-byte base hash
        let mut data = vec![0x75];
        data.extend(1..=20u8);
        let lines = render(&data);

        let idx = lines
            .iter()
            .position(|l| l.contains("Base object hash"))
            .expect("base hash label");
        let expected: String = (1..=20u8).map(|b| format!("{b:02X}")).collect();
        assert_eq!(lines[idx + 1].trim(), expected);
    }
}
