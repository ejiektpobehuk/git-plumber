pub mod chunks;
pub mod header;
pub mod objects;
pub mod packs;

use crate::git::pack::MultiPackIndex;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub use chunks::ChunksFormatter;
pub use header::HeaderFormatter;
pub use objects::ObjectsFormatter;
pub use packs::PacksFormatter;

pub struct MultiPackIndexFormatter<'a> {
    multi_pack_index: &'a MultiPackIndex,
}

impl<'a> MultiPackIndexFormatter<'a> {
    #[must_use]
    pub const fn new(multi_pack_index: &'a MultiPackIndex) -> Self {
        Self { multi_pack_index }
    }

    #[must_use]
    pub fn generate_content(&self) -> Text<'static> {
        let mut lines = vec![
            Line::from(
                "One index over many pack files: maps each object ID to the pack",
            ),
            Line::from(
                "that stores it and its offset there. Git binary-searches this one",
            ),
            Line::from(
                "file instead of every per-pack .idx. Data lives in chunks located",
            ),
            Line::from("via the chunk lookup table."),
            Line::from(""),
        ];

        HeaderFormatter::new(self.multi_pack_index).format_header(&mut lines);
        ChunksFormatter::new(self.multi_pack_index).format_chunk_table(&mut lines);
        PacksFormatter::new(self.multi_pack_index).format_pack_names(&mut lines);
        ObjectsFormatter::new(self.multi_pack_index).format_objects_overview(&mut lines);
        self.add_checksum_section(&mut lines);
        self.add_structure_diagram(&mut lines);

        Text::from(lines)
    }

    fn add_checksum_section(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "CHECKSUM",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let checksum_start = self
            .multi_pack_index
            .raw_data
            .len()
            .saturating_sub(self.multi_pack_index.checksum_size());

        lines.push(Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Multi-pack-index checksum Hex",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "──────┼─────────────────────────────────────────",
            Style::default().fg(Color::Gray),
        )]));
        lines.push(Line::from(vec![
            Span::styled(
                format!("{checksum_start:5}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::from(hex::encode(&self.multi_pack_index.checksum)),
        ]));
        lines.push(Line::from(""));
    }

    fn add_structure_diagram(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "FILE STRUCTURE DIAGRAM",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let chunk_count = self.multi_pack_index.chunks.len();

        lines.push(Line::from("┌─────────────────────────────────┐"));
        lines.push(Line::from("│ Header (magic \"MIDX\", ...)      │ 12 bytes"));
        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from(format!(
            "│ Chunk Lookup Table              │ {} × 12 bytes",
            chunk_count + 1
        )));

        for chunk in &self.multi_pack_index.chunks {
            lines.push(Line::from("├─────────────────────────────────┤"));
            lines.push(Line::from(format!(
                "│ {:<4} Chunk                      │ {} bytes",
                chunk.id_str(),
                chunk.size
            )));
        }

        lines.push(Line::from("├─────────────────────────────────┤"));
        lines.push(Line::from(format!(
            "│ Checksum                        │ {} bytes",
            self.multi_pack_index.checksum_size()
        )));
        lines.push(Line::from("└─────────────────────────────────┘"));
        lines.push(Line::from(""));
    }
}
