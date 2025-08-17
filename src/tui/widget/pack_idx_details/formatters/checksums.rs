use crate::git::pack::PackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

pub struct ChecksumsFormatter<'a> {
    pack_index: &'a PackIndex,
}

impl<'a> ChecksumsFormatter<'a> {
    pub fn new(pack_index: &'a PackIndex) -> Self {
        Self { pack_index }
    }

    pub fn format_checksums(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "CHECKSUMS & INTEGRITY",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        self.add_sha1_checksums_section(lines);
        self.add_integrity_info(lines);
    }

    fn add_sha1_checksums_section(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "SHA-1 Checksums:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        // Pack file checksum
        lines.push(Line::styled(
            "Pack File Checksum:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        ));
        lines.push(Line::styled(
            format!("  {}", hex::encode(self.pack_index.pack_checksum)),
            Style::default().fg(Color::Green),
        ));
        lines.push(Line::from(
            "• Purpose: Identifies the corresponding .pack file",
        ));
        lines.push(Line::from(
            "• Must match the checksum at the end of .pack file",
        ));
        lines.push(Line::from(
            "• Ensures index corresponds to correct pack data",
        ));
        lines.push(Line::from(""));

        // Index file checksum
        lines.push(Line::styled(
            "Index File Checksum:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        ));
        lines.push(Line::styled(
            format!("  {}", hex::encode(self.pack_index.index_checksum)),
            Style::default().fg(Color::Green),
        ));
        lines.push(Line::from(
            "• Purpose: Verifies integrity of entire index file",
        ));
        lines.push(Line::from("• Calculated over all preceding index data"));
        lines.push(Line::from("• Detects corruption or tampering"));
        lines.push(Line::from(""));
    }

    fn add_integrity_info(&self, lines: &mut Vec<Line<'static>>) {
        // Verification status (placeholder - could be enhanced with actual verification)
        lines.push(Line::styled(
            "Verification Status:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            "  ℹ Checksum verification not implemented yet",
            Style::default().fg(Color::Blue),
        ));
        lines.push(Line::from(
            "  Use 'git fsck' to verify repository integrity",
        ));
        lines.push(Line::from(""));
    }
}
