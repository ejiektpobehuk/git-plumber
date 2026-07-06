use crate::git::pack::PackBitmap;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct HeaderFormatter<'a> {
    bitmap: &'a PackBitmap,
}

impl<'a> HeaderFormatter<'a> {
    #[must_use]
    pub const fn new(bitmap: &'a PackBitmap) -> Self {
        Self { bitmap }
    }

    pub fn format_header(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "BITMAP HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let border_style = Style::default().fg(Color::Gray);
        let value_style = Style::default().fg(Color::LightGreen);

        lines.push(Line::from(vec![
            Span::styled(" Byte  ", border_style),
            Span::styled("│ ", border_style),
            Span::styled(
                "Field           ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", border_style),
            Span::styled("Value", Style::default().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(
            "───────┼─────────────────┼─────────────────────",
            border_style,
        ));

        let raw = &self.bitmap.raw_data;
        let signature_utf8: String = raw[0..4].iter().map(|&b| b as char).collect();

        let mut push_row = |byte_range: &str, field: &str, value: String| {
            lines.push(Line::from(vec![
                Span::styled(format!(" {byte_range:<6}"), border_style),
                Span::styled("│ ", border_style),
                Span::from(format!("{field:<16}")),
                Span::styled("│ ", border_style),
                Span::styled(value, value_style),
            ]));
        };

        push_row(
            "0-3",
            "Signature",
            format!("\"{}\" ({})", signature_utf8, hex::encode(&raw[0..4])),
        );
        push_row("4-5", "Version", format!("{}", self.bitmap.version));
        push_row(
            "6-7",
            "Flags",
            format!(
                "0x{:04x} = {}",
                self.bitmap.flags,
                self.bitmap.flag_names().join(" | ")
            ),
        );
        push_row(
            "8-11",
            "Entry count",
            format!("{}", self.bitmap.entry_count),
        );

        let checksum_end = 12 + self.bitmap.checksum_size - 1;
        push_row(
            &format!("12-{checksum_end}"),
            "Pack checksum",
            hex::encode(&self.bitmap.pack_checksum),
        );

        lines.push(Line::from(""));

        // Explain what each flag means
        lines.push(Line::from(vec![
            Span::styled("  • FULL_DAG: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "every object's parents are in the same pack/MIDX (required)",
                Style::default().fg(Color::Gray),
            ),
        ]));
        if self.bitmap.has_hash_cache() {
            lines.push(Line::from(vec![
                Span::styled("  • HASH_CACHE: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "file ends with a path-name hash per object (delta heuristics)",
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        if self.bitmap.has_lookup_table() {
            lines.push(Line::from(vec![
                Span::styled("  • LOOKUP_TABLE: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "table at the end locates each commit's bitmap without scanning",
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        if self.bitmap.has_pseudo_merges() {
            lines.push(Line::from(vec![
                Span::styled("  • PSEUDO_MERGES: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "extra bitmaps covering groups of commits merged together",
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }

        lines.push(Line::from(""));
    }
}
