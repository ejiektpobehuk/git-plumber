use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

use crate::tui::widget::pack_obj_details::config::HEX_PREVIEW_LIMIT;

// Separate delta formatting logic
pub struct DeltaFormatter<'a> {
    uncompressed_data: &'a [u8],
}

impl<'a> DeltaFormatter<'a> {
    pub const fn new(uncompressed_data: &'a [u8]) -> Self {
        Self { uncompressed_data }
    }

    pub fn format_delta_instructions(&self, lines: &mut Vec<Line<'static>>) {
        match crate::git::pack::parse_delta_instructions(self.uncompressed_data) {
            Ok((_, instructions)) => {
                self.format_instructions_header(lines, &instructions);
                self.format_individual_instructions(lines, &instructions);
                self.format_reconstruction_explanation(lines);
            }
            Err(e) => {
                lines.push(Line::from(format!(
                    "Error parsing delta instructions: {e:?}"
                )));
            }
        }
    }

    fn format_instructions_header(
        &self,
        lines: &mut Vec<Line<'static>>,
        instructions: &[crate::git::pack::DeltaInstruction],
    ) {
        lines.push(Line::styled(
            "DELTA INSTRUCTIONS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Total Instructions: {}",
            instructions.len()
        )));
        lines.push(Line::from(""));
    }

    fn format_individual_instructions(
        &self,
        lines: &mut Vec<Line<'static>>,
        instructions: &[crate::git::pack::DeltaInstruction],
    ) {
        for (i, instruction) in instructions.iter().enumerate() {
            match instruction {
                crate::git::pack::DeltaInstruction::Copy { offset, size } => {
                    self.format_copy_instruction(lines, i + 1, *offset, *size);
                }
                crate::git::pack::DeltaInstruction::Insert { data } => {
                    self.format_insert_instruction(lines, i + 1, data);
                }
            }
            lines.push(Line::from(""));
        }
    }

    fn format_copy_instruction(
        &self,
        lines: &mut Vec<Line<'static>>,
        index: usize,
        offset: usize,
        size: usize,
    ) {
        lines.push(Line::from(format!(
            "  {index}. COPY: offset={offset} (0x{offset:x}), size={size} bytes"
        )));
        lines.push(Line::from(format!(
            "      └─ Copy {size} bytes from base object starting at offset {offset}"
        )));
    }

    fn format_insert_instruction(&self, lines: &mut Vec<Line<'static>>, index: usize, data: &[u8]) {
        lines.push(Line::from(format!(
            "  {}. INSERT: {} bytes",
            index,
            data.len()
        )));

        if data.len() <= 64 {
            self.format_small_insert_data(lines, data);
        } else {
            self.format_large_insert_data(lines, data);
        }
    }

    fn format_small_insert_data(&self, lines: &mut Vec<Line<'static>>, data: &[u8]) {
        let data_str = String::from_utf8_lossy(data);
        if data_str
            .chars()
            .all(|c| c.is_ascii() && !c.is_control() || c == '\n' || c == '\t')
        {
            lines.push(Line::from(format!("      └─ Data: {data_str}")));
        } else {
            lines.push(Line::from(format!("      └─ Hex: {}", hex::encode(data))));
        }
    }

    fn format_large_insert_data(&self, lines: &mut Vec<Line<'static>>, data: &[u8]) {
        lines.push(Line::from(format!(
            "      └─ Hex (first 32 bytes): {}",
            hex::encode(&data[..HEX_PREVIEW_LIMIT])
        )));
        lines.push(Line::from(format!(
            "      └─ ... and {} more bytes",
            data.len() - HEX_PREVIEW_LIMIT
        )));
    }

    fn format_reconstruction_explanation(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from("DELTA RECONSTRUCTION PROCESS"));
        lines.push(Line::from("─".repeat(35)));
        lines.push(Line::from(""));
        lines.push(Line::from("To reconstruct the original object:"));
        lines.push(Line::from("1. Start with the base object"));
        lines.push(Line::from("2. Apply each instruction in sequence:"));
        lines.push(Line::from("   • COPY: Copy bytes from base object"));
        lines.push(Line::from("   • INSERT: Add new bytes to output"));
        lines.push(Line::from("3. The result is the reconstructed object"));
        lines.push(Line::from(""));
        lines.push(Line::from("This delta compression allows Git to store"));
        lines.push(Line::from("similar objects very efficiently by only"));
        lines.push(Line::from("storing the differences."));
    }
}
