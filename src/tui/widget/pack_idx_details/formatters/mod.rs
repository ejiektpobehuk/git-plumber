pub mod checksums;
pub mod fanout;
pub mod header;
pub mod objects;

use crate::git::pack::PackIndex;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};

pub use checksums::ChecksumsFormatter;
pub use fanout::FanoutFormatter;
pub use header::HeaderFormatter;
pub use objects::ObjectsFormatter;

pub struct PackIndexFormatter<'a> {
    pack_index: &'a PackIndex,
}

impl<'a> PackIndexFormatter<'a> {
    pub fn new(pack_index: &'a PackIndex) -> Self {
        Self { pack_index }
    }

    pub fn generate_content(&self) -> Text<'static> {
        let mut lines = Vec::new();
        // Add educational information at the top
        self.add_educational_info(&mut lines);
        // Add detailed sections
        self.add_header_section(&mut lines);
        self.add_fanout_section(&mut lines);
        self.add_objects_section(&mut lines);
        self.add_checksums_section(&mut lines);
        self.add_structure_diagram(&mut lines);

        Text::from(lines)
    }

    fn add_educational_info(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(
            "Index file helps Git to quickly locate objects in the pack file.",
        ));
        lines.push(Line::from(
            "It has a quick lookup table to find the offset of an object in the pack file.",
        ));
        lines.push(Line::from(""));
    }

    fn add_header_section(&self, lines: &mut Vec<Line<'static>>) {
        let formatter = HeaderFormatter::new(self.pack_index);
        formatter.format_header(lines);
    }

    fn add_fanout_section(&self, lines: &mut Vec<Line<'static>>) {
        let formatter = FanoutFormatter::new(self.pack_index);
        formatter.format_fanout_table(lines);
    }

    fn add_objects_section(&self, lines: &mut Vec<Line<'static>>) {
        let formatter = ObjectsFormatter::new(self.pack_index);
        formatter.format_objects_overview(lines);
    }

    fn add_checksums_section(&self, lines: &mut Vec<Line<'static>>) {
        let formatter = ChecksumsFormatter::new(self.pack_index);
        formatter.format_checksums(lines);
    }

    fn add_structure_diagram(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "FILE STRUCTURE DIAGRAM",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        lines.push(Line::from("┌─────────────────────────────────┐"));
        lines.push(Line::from("│ Magic Number (\\377tOc)          │ 4 bytes"));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from("│ Version (2)                     │ 4 bytes"));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from(
            "│ Fan-out Table                   │ 256 × 4 bytes",
        ));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from(format!(
            "│ Object Names (SHA-1)            │ {} × 20 bytes",
            self.pack_index.object_count()
        )));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from(format!(
            "│ CRC32 Checksums                 │ {} × 4 bytes",
            self.pack_index.object_count()
        )));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from(format!(
            "│ Pack File Offsets               │ {} × 4 bytes",
            self.pack_index.object_count()
        )));

        if let Some(ref large_offsets) = self.pack_index.large_offsets {
            lines.push(Line::from("├─────────────────────────────────┤"));
            lines.push(Line::from(format!(
                "│ Large Offsets (optional)        │ {} × 8 bytes",
                large_offsets.len()
            )));
        }

        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from("│ Pack File Checksum              │ 20 bytes"));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from("│ Index File Checksum             │ 20 bytes"));
        lines.push(Line::from("└─────────────────────────────────┘"));
        lines.push(Line::from(""));
    }
}
