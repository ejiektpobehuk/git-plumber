pub mod entries;
pub mod header;
pub mod type_bitmaps;

use crate::git::pack::PackBitmap;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

pub use header::HeaderFormatter;

pub struct PackBitmapFormatter<'a> {
    bitmap: &'a PackBitmap,
}

impl<'a> PackBitmapFormatter<'a> {
    #[must_use]
    pub const fn new(bitmap: &'a PackBitmap) -> Self {
        Self { bitmap }
    }

    #[must_use]
    pub fn generate_content(&self) -> Text<'static> {
        // Purpose explanation
        let mut lines = vec![
            Line::from(vec![Span::from(
                "Stores reachability bitmaps for selected commits: one bit per",
            )]),
            Line::from(vec![Span::from(
                "object in the pack/MIDX saying whether it is reachable from that",
            )]),
            Line::from(vec![Span::from(
                "commit. Lets git answer \"which objects does this fetch need?\"",
            )]),
            Line::from(vec![Span::from(
                "with bit operations instead of walking the object graph.",
            )]),
            Line::from(""),
        ];

        // Header section
        HeaderFormatter::new(self.bitmap).format_header(&mut lines);

        // Type index bitmaps section
        lines.extend(type_bitmaps::format_type_bitmaps(self.bitmap));

        // Commit entries section
        lines.extend(entries::format_entries(self.bitmap));

        // Optional trailing sections
        self.add_optional_sections(&mut lines);

        // Trailing checksum
        self.add_checksum_section(&mut lines);

        Text::from(lines)
    }

    fn entries_end_position(&self) -> usize {
        let type_bitmaps_size = [
            &self.bitmap.commits_bitmap,
            &self.bitmap.trees_bitmap,
            &self.bitmap.blobs_bitmap,
            &self.bitmap.tags_bitmap,
        ]
        .iter()
        .map(|b| b.compressed_byte_size())
        .sum::<usize>();
        let entries_size = self
            .bitmap
            .entries
            .iter()
            .map(|e| 6 + e.bitmap.compressed_byte_size())
            .sum::<usize>();

        12 + self.bitmap.checksum_size + type_bitmaps_size + entries_size
    }

    fn add_optional_sections(&self, lines: &mut Vec<Line<'static>>) {
        if !self.bitmap.has_pseudo_merges()
            && !self.bitmap.has_lookup_table()
            && !self.bitmap.has_hash_cache()
        {
            return;
        }

        lines.push(Line::styled(
            "OPTIONAL SECTIONS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let border_style = Style::default().fg(Color::Gray);
        let value_style = Style::default().fg(Color::LightGreen);
        let mut byte_position = self.entries_end_position();

        if self.bitmap.has_pseudo_merges() {
            lines.push(Line::from(vec![
                Span::styled(format!(" {byte_position:6}"), border_style),
                Span::styled(" │ ", border_style),
                Span::from("Pseudo-merge bitmaps: "),
                Span::styled(
                    format!("{} bytes", self.bitmap.pseudo_merge_size),
                    value_style,
                ),
                Span::styled(" (not decoded)", border_style),
            ]));
            byte_position += self.bitmap.pseudo_merge_size;
        }

        if let Some(table) = &self.bitmap.lookup_table {
            lines.push(Line::from(vec![
                Span::styled(format!(" {byte_position:6}"), border_style),
                Span::styled(" │ ", border_style),
                Span::from("Commit lookup table: "),
                Span::styled(format!("{} × 16 bytes", table.len()), value_style),
            ]));
            byte_position += table.len() * 16;

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("   Pos", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("      │ ", border_style),
                Span::styled("Offset", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("     │ ", border_style),
                Span::styled("XOR row", Style::default().add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::styled(
                "  ─────────┼────────────┼─────────",
                border_style,
            ));
            for entry in table {
                let xor_row =
                    if entry.xor_row == crate::git::pack::bitmap::LookupTableEntry::NO_XOR_ROW {
                        "none".to_string()
                    } else {
                        format!("{}", entry.xor_row)
                    };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:8}", entry.commit_pos),
                        Style::default().fg(Color::LightBlue),
                    ),
                    Span::styled(" │ ", border_style),
                    Span::styled(format!("{:10}", entry.offset), value_style),
                    Span::styled(" │ ", border_style),
                    Span::styled(xor_row, value_style),
                ]));
            }
            lines.push(Line::from(""));
        }

        if self.bitmap.has_hash_cache() {
            lines.push(Line::from(vec![
                Span::styled(format!(" {byte_position:6}"), border_style),
                Span::styled(" │ ", border_style),
                Span::from("Name-hash cache: "),
                Span::styled(
                    format!(
                        "{} × 4 bytes (one path-name hash per object)",
                        self.bitmap.hash_cache_size / 4
                    ),
                    value_style,
                ),
            ]));
        }

        lines.push(Line::from(""));
    }

    fn add_checksum_section(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "CHECKSUM",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let border_style = Style::default().fg(Color::Gray);
        let checksum_start = self
            .bitmap
            .raw_data
            .len()
            .saturating_sub(self.bitmap.checksum_size);

        lines.push(Line::from(vec![
            Span::styled(" Byte", border_style),
            Span::styled("   │ ", border_style),
            Span::styled(
                "Bitmap file checksum Hex",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "────────┼─────────────────────────────────────────",
            border_style,
        )]));
        lines.push(Line::from(vec![
            Span::styled(format!(" {checksum_start:6}"), border_style),
            Span::styled(" │ ", border_style),
            Span::from(hex::encode(&self.bitmap.file_checksum)),
        ]));
    }
}
