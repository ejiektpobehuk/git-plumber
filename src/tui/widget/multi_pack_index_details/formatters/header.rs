use crate::git::pack::MultiPackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct HeaderFormatter<'a> {
    multi_pack_index: &'a MultiPackIndex,
}

impl<'a> HeaderFormatter<'a> {
    #[must_use]
    pub const fn new(multi_pack_index: &'a MultiPackIndex) -> Self {
        Self { multi_pack_index }
    }

    pub fn format_header(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "MULTI-PACK-INDEX HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let border_style = Style::default().fg(Color::Gray);
        let value_style = Style::default().fg(Color::LightGreen);

        lines.push(Line::from(vec![
            Span::styled(" Byte ", border_style),
            Span::styled("│ ", border_style),
            Span::styled("Field           ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("│ ", border_style),
            Span::styled("Value", Style::default().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(
            "──────┼─────────────────┼─────────────────────",
            border_style,
        ));

        let raw = &self.multi_pack_index.raw_data;
        let signature_utf8: String = raw[0..4].iter().map(|&b| b as char).collect();

        let mut push_row = |byte_range: &str, field: &str, value: String| {
            lines.push(Line::from(vec![
                Span::styled(format!(" {byte_range:<5}"), border_style),
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
        push_row(
            "4",
            "Version",
            format!("{}", self.multi_pack_index.version),
        );
        push_row(
            "5",
            "Hash function",
            format!(
                "{} ({})",
                self.multi_pack_index.hash_function_id,
                self.multi_pack_index.hash_function_name()
            ),
        );
        push_row(
            "6",
            "Chunk count",
            format!("{}", self.multi_pack_index.chunk_count),
        );
        push_row(
            "7",
            "Base MIDX count",
            format!("{}", self.multi_pack_index.base_midx_count),
        );
        push_row(
            "8-11",
            "Number of packs",
            format!("{}", self.multi_pack_index.num_packs),
        );

        lines.push(Line::from(""));
    }
}
