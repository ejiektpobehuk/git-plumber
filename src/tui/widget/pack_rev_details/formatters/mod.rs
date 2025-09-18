pub mod header;
pub mod mappings;

use crate::git::pack::PackReverseIndex;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub use header::HeaderFormatter;

pub struct PackReverseIndexFormatter<'a> {
    reverse_index: &'a PackReverseIndex,
}

impl<'a> PackReverseIndexFormatter<'a> {
    #[must_use]
    pub const fn new(reverse_index: &'a PackReverseIndex) -> Self {
        Self { reverse_index }
    }

    #[must_use]
    pub fn generate_content(&self) -> Text<'static> {
        let mut lines = Vec::new();

        // Purpose explanation
        lines.push(Line::from(vec![Span::from(
            "Enables O(1) conversion between pack positions and index positions.",
        )]));
        lines.push(Line::from(vec![Span::from(
            "Used for efficient disk size calculations and bitmap operations.",
        )]));
        lines.push(Line::from(""));

        // Header section
        let header_formatter = HeaderFormatter::new(self.reverse_index);
        header_formatter.format_header(&mut lines);

        lines.push(Line::from(""));

        // Mappings section
        lines.extend(mappings::format_mappings(self.reverse_index));

        lines.push(Line::from(""));

        // Calculate byte positions for checksums
        // Header: 12 bytes (4 signature + 4 version + 4 hash function ID)
        // Index positions: 4 bytes × number of objects
        let header_size = 12;
        let index_table_size = self.reverse_index.index_positions.len() * 4;
        let checksum_size = self.reverse_index.checksum_size();

        let pack_checksum_start = header_size + index_table_size;
        let file_checksum_start = pack_checksum_start + checksum_size;

        // Checksums
        lines.push(Line::styled(
            "CHECKSUMS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        // Packfile checksum
        lines.push(Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Packfile checksum Hex",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "──────┼─────────────────────────────────────────",
            Style::default().fg(Color::Gray),
        )]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("{pack_checksum_start:5}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::from(hex::encode(&self.reverse_index.pack_checksum)),
        ]));

        lines.push(Line::from(""));

        // Reverse checksum checksum
        lines.push(Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Reverse index checksum Hex",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("               │", Style::default().fg(Color::Gray)),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "──────┼──────────────────────────────────────────┤",
            Style::default().fg(Color::Gray),
        )]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("{file_checksum_start:5}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::from(hex::encode(&self.reverse_index.file_checksum)),
            Span::styled(" │", Style::default().fg(Color::Gray)),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "──────┴──────────────────────────────────────────╯",
            Style::default().fg(Color::Gray),
        )]));

        Text::from(lines)
    }
}
