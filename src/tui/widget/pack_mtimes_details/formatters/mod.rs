pub mod entries;
pub mod header;

use crate::git::pack::PackMtimes;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub use header::HeaderFormatter;

pub struct PackMtimesFormatter<'a> {
    mtimes: &'a PackMtimes,
}

impl<'a> PackMtimesFormatter<'a> {
    #[must_use]
    pub const fn new(mtimes: &'a PackMtimes) -> Self {
        Self { mtimes }
    }

    #[must_use]
    pub fn generate_content(&self) -> Text<'static> {
        // Purpose explanation
        let mut lines = vec![
            Line::from(vec![Span::from(
                "Stores per-object modification times for a cruft pack, which holds",
            )]),
            Line::from(vec![Span::from(
                "unreachable objects. Lets `git gc` expire each object individually",
            )]),
            Line::from(vec![Span::from(
                "instead of tracking mtimes of loose object files.",
            )]),
            Line::from(""),
        ];

        // Header section
        let header_formatter = HeaderFormatter::new(self.mtimes);
        header_formatter.format_header(&mut lines);

        lines.push(Line::from(""));

        // Mtimes table section
        lines.extend(entries::format_entries(self.mtimes));

        lines.push(Line::from(""));

        // Calculate byte positions for checksums
        // Header: 12 bytes (4 signature + 4 version + 4 hash function ID)
        // Mtimes table: 4 bytes × number of objects
        let header_size = 12;
        let mtimes_table_size = self.mtimes.mtimes.len() * 4;
        let checksum_size = self.mtimes.checksum_size();

        let pack_checksum_start = header_size + mtimes_table_size;
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
            Span::from(hex::encode(&self.mtimes.pack_checksum)),
        ]));

        lines.push(Line::from(""));

        // Mtimes file checksum
        lines.push(Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Mtimes file checksum Hex",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("                 │", Style::default().fg(Color::Gray)),
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
            Span::from(hex::encode(&self.mtimes.file_checksum)),
            Span::styled(" │", Style::default().fg(Color::Gray)),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "──────┴──────────────────────────────────────────╯",
            Style::default().fg(Color::Gray),
        )]));

        Text::from(lines)
    }
}
