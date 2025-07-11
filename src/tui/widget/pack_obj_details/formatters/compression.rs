use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::tui::widget::pack_obj_details::config::{ADLER32_SIZE, COLORS, calculate_adler32};

// Separate formatters for specific sections
pub struct ZlibHeaderFormatter<'a> {
    compressed_data: &'a [u8],
}

impl<'a> ZlibHeaderFormatter<'a> {
    pub const fn new(compressed_data: &'a [u8]) -> Self {
        Self { compressed_data }
    }

    pub fn format_header(&self, lines: &mut Vec<Line<'static>>) {
        self.format_cmf_byte(lines);
        self.format_flg_byte(lines);
    }

    fn format_cmf_byte(&self, lines: &mut Vec<Line<'static>>) {
        let byte1 = self.compressed_data[0];
        lines.push(Line::from(vec![
            Span::from("Byte 1"),
            Span::styled("   CMF", Style::default().add_modifier(Modifier::BOLD)),
            Span::from(" - "),
            Span::styled("C", Style::default().add_modifier(Modifier::BOLD)),
            Span::from("ompression "),
            Span::styled("m", Style::default().add_modifier(Modifier::BOLD)),
            Span::from("ethod and "),
            Span::styled("f", Style::default().add_modifier(Modifier::BOLD)),
            Span::from("lags"),
        ]));

        let cmf_line = vec![
            Span::styled("  |", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:04b}", byte1 >> 4),
                Style::default().fg(COLORS[0]),
            ),
            Span::styled(
                format!("{:04b}", byte1 & 0x0F),
                Style::default().fg(COLORS[1]),
            ),
            Span::from("┊"),
            Span::from(" - 0x"),
            Span::styled(
                format!("{:01X}", byte1 >> 4),
                Style::default().fg(COLORS[0]),
            ),
            Span::styled(
                format!("{:01X}", byte1 & 0x0F),
                Style::default().fg(COLORS[1]),
            ),
        ];
        lines.push(Line::from(cmf_line));

        let cinfo = (byte1 >> 4) & 0x0F;
        let cm = byte1 & 0x0F;

        lines.push(Line::from(vec![
            Span::from("   ├──╯"),
            Span::from("╰──┴─ "),
            Span::from("CINFO: "),
            Span::from(format!("{} - {} window size", cinfo, 1 << (cinfo + 8))),
        ]));
        lines.push(Line::from(format!(
            "   ╰─ CM: {} - {}",
            cm,
            if cm == 8 {
                "deflate compression method"
            } else {
                "unknown compression method"
            }
        )));
        lines.push(Line::from(""));
    }

    fn format_flg_byte(&self, lines: &mut Vec<Line<'static>>) {
        let byte1 = self.compressed_data[0];
        let byte2 = self.compressed_data[1];

        lines.push(Line::from(vec![
            Span::from("Byte 2"),
            Span::styled("   FLG", Style::default().add_modifier(Modifier::BOLD)),
            Span::from(" - "),
            Span::styled("Fl", Style::default().add_modifier(Modifier::BOLD)),
            Span::from("a"),
            Span::styled("g", Style::default().add_modifier(Modifier::BOLD)),
            Span::from("s"),
        ]));

        let flg_line = vec![
            Span::styled("  ┊", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:02b}", byte2 >> 6),
                Style::default().fg(COLORS[0]),
            ),
            Span::styled(
                format!("{:01b}", (byte2 >> 5) & 0x01),
                Style::default().fg(COLORS[1]),
            ),
            Span::styled(
                format!("{:05b}", byte2 & 0x1F),
                Style::default().fg(COLORS[2]),
            ),
            Span::styled("|", Style::default().add_modifier(Modifier::BOLD)),
            Span::from(" - 0x"),
            Span::from(format!("{byte2:02X}")),
        ];
        lines.push(Line::from(flg_line));

        let flevel = byte2 >> 6;
        let fdict = (byte2 >> 5) & 0x01;
        let fcheck = byte2 & 0x1F;

        // Format flag explanations
        Self::format_flag_explanations(lines, byte1, byte2, flevel, fdict, fcheck);
    }

    fn format_flag_explanations(
        lines: &mut Vec<Line<'static>>,
        byte1: u8,
        byte2: u8,
        flevel: u8,
        fdict: u8,
        fcheck: u8,
    ) {
        let checksum = (u16::from(byte1) * 256 + u16::from(byte2)) % 31;

        lines.push(Line::from(vec![
            Span::styled("   ├╯", Style::default().fg(COLORS[0])),
            Span::styled("│", Style::default().fg(COLORS[1])),
            Span::styled("╰───┴─ ", Style::default().fg(COLORS[2])),
            Span::from("FCHECK: "),
            Span::styled(format!("{fcheck:05b}"), Style::default().fg(COLORS[2])),
            Span::from(" - checksum bits "),
            if checksum == 0 {
                Span::styled("✓", Style::default().fg(Color::Green))
            } else {
                Span::styled("⚠", Style::default().fg(Color::Red))
            },
        ]));

        lines.push(Line::from(vec![
            Span::styled("   │", Style::default().fg(COLORS[0])),
            Span::styled(" ╰─ ", Style::default().fg(COLORS[1])),
            Span::from("FDICT: "),
            Span::styled(format!("{fdict}"), Style::default().fg(COLORS[1])),
            Span::from(format!(
                " - {}",
                if fdict == 0 {
                    "no dictionary is used"
                } else {
                    "a dictionary is used"
                }
            )),
        ]));

        lines.push(Line::from(vec![
            Span::styled("   ╰─ ", Style::default().fg(COLORS[2])),
            Span::from("FLEVEL: "),
            Span::styled(format!("{flevel}"), Style::default().fg(COLORS[2])),
            Span::from(format!(
                " - {}",
                match flevel {
                    0 => "fastest compression",
                    1 => "fast compression",
                    2 => "default compression",
                    3 => "maximum compression",
                    _ => "unknown",
                }
            )),
        ]));

        lines.push(Line::from(""));
    }
}

pub struct DeflateBlockFormatter<'a> {
    compressed_data: &'a [u8],
}

impl<'a> DeflateBlockFormatter<'a> {
    pub const fn new(compressed_data: &'a [u8]) -> Self {
        Self { compressed_data }
    }

    pub fn format_block_header(&self, lines: &mut Vec<Line<'static>>) {
        let byte3 = self.compressed_data[2];
        let bfinal = byte3 & 0x01;
        let btype = (byte3 >> 1) & 0x03;
        let remaining_bits = byte3 >> 3;

        lines.push(Line::from("Byte 3   Start of deflate compressed data"));

        let deflate_line = vec![
            Span::styled("  |", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{remaining_bits:05b}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(format!("{btype:02b}"), Style::default().fg(COLORS[0])),
            Span::styled(format!("{bfinal:01b}"), Style::default().fg(COLORS[1])),
            Span::from("┊"),
            Span::from(" - 0x"),
            Span::from(format!("{byte3:02X}")),
        ];
        lines.push(Line::from(deflate_line));

        Self::format_block_flags(lines, bfinal, btype, remaining_bits);

        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Bytes [ {} - {} ] contain compressed data",
            4,
            self.compressed_data.len() - 3 - 1
        )));
    }

    fn format_block_flags(
        lines: &mut Vec<Line<'static>>,
        bfinal: u8,
        btype: u8,
        _remaining_bits: u8,
    ) {
        lines.push(Line::from(vec![
            Span::styled("   ├───╯", Style::default().fg(Color::Gray)),
            Span::styled("├╯", Style::default().fg(COLORS[0])),
            Span::styled("╰─ ", Style::default().fg(COLORS[1])),
            Span::from("BFINAL: "),
            Span::styled(format!("{bfinal}"), Style::default().fg(COLORS[2])),
        ]));

        lines.push(Line::from(vec![
            Span::styled("   │", Style::default().fg(Color::Gray)),
            Span::styled("    ╰─ ", Style::default().fg(COLORS[0])),
            Span::from("BTYPE: "),
            Span::styled(format!("{btype}"), Style::default().fg(COLORS[1])),
            Span::from(format!(
                " - {}",
                match btype {
                    0 => "no compression (stored)",
                    1 => "fixed Huffman codes",
                    2 => "dynamic Huffman codes",
                    3 => "reserved (error)",
                    _ => "unknown",
                }
            )),
        ]));

        lines.push(Line::from(vec![
            Span::styled("   ╰─ ", Style::default().fg(Color::Gray)),
            Span::from(format!(
                "start of {}",
                match btype {
                    0 => "literal data length field",
                    1 => "fixed Huffman compressed data",
                    2 => "dynamic Huffman table definition",
                    3 => "invalid data",
                    _ => "unknown data",
                }
            )),
        ]));
    }
}

pub struct Adler32Formatter<'a> {
    compressed_data: &'a [u8],
    uncompressed_data: &'a [u8],
}

impl<'a> Adler32Formatter<'a> {
    pub fn new(compressed_data: &'a [u8], uncompressed_data: &'a [u8]) -> Self {
        Self {
            compressed_data,
            uncompressed_data,
        }
    }

    pub fn format_checksum(&self, lines: &mut Vec<Line<'static>>) {
        let len = self.compressed_data.len();
        let checksum_bytes = &self.compressed_data[len - ADLER32_SIZE..];

        let calculated_checksum = calculate_adler32(self.uncompressed_data);
        let stored_checksum = u32::from_be_bytes([
            checksum_bytes[0],
            checksum_bytes[1],
            checksum_bytes[2],
            checksum_bytes[3],
        ]);

        let colored_spans = self.format_checksum_bytes(lines, checksum_bytes, len);
        self.format_checksum_verification(
            lines,
            stored_checksum,
            calculated_checksum,
            colored_spans,
        );
    }

    fn format_checksum_bytes(
        &self,
        lines: &mut Vec<Line<'static>>,
        checksum_bytes: &[u8],
        len: usize,
    ) -> Vec<Span<'static>> {
        let mut stored_checksum_colored = Vec::new();

        for (i, &byte) in checksum_bytes.iter().enumerate() {
            let byte_num = len - ADLER32_SIZE + i + 1;
            let byte_line = vec![
                Span::from(format!("Byte {byte_num} ")),
                if i == 0 {
                    Span::styled("  |", Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Span::from("  ┊")
                },
                Span::styled(
                    format!("{byte:08b}"),
                    Style::default().fg(COLORS[i % COLORS.len()]),
                ),
                if i == checksum_bytes.len() - 1 {
                    Span::styled("|", Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Span::from("┊")
                },
                Span::from(" - 0x"),
                Span::styled(
                    format!("{byte:02X}"),
                    Style::default().fg(COLORS[i % COLORS.len()]),
                ),
            ];
            lines.push(Line::from(byte_line));

            stored_checksum_colored.push(Span::styled(
                format!("{byte:02X}"),
                Style::default().fg(COLORS[i % COLORS.len()]),
            ));
        }

        lines.push(Line::from(""));
        stored_checksum_colored
    }

    fn format_checksum_verification(
        &self,
        lines: &mut Vec<Line<'static>>,
        stored_checksum: u32,
        calculated_checksum: u32,
        colored_spans: Vec<Span<'static>>,
    ) {
        lines.push(Line::from("Checksum verification:"));

        let mut stored_line = vec![Span::from("  - Stored checksum:   0x")];
        stored_line.extend(colored_spans);
        lines.push(Line::from(stored_line));

        lines.push(Line::from(format!(
            "  - Calculated checksum: 0x{calculated_checksum:08X}"
        )));

        let is_valid = stored_checksum == calculated_checksum;
        let mut verification_line = vec![Span::from("  - Verification: ")];
        if is_valid {
            verification_line.push(Span::from("checksum matches "));
            verification_line.push(Span::styled("✓", Style::default().fg(Color::Green)));
        } else {
            verification_line.push(Span::from("checksum mismatch "));
            verification_line.push(Span::styled("✗", Style::default().fg(Color::Red)));
        }
        lines.push(Line::from(verification_line));
    }
}
