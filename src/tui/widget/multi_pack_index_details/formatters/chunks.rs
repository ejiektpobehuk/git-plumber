use crate::git::pack::MultiPackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct ChunksFormatter<'a> {
    multi_pack_index: &'a MultiPackIndex,
}

impl<'a> ChunksFormatter<'a> {
    #[must_use]
    pub const fn new(multi_pack_index: &'a MultiPackIndex) -> Self {
        Self { multi_pack_index }
    }

    pub fn format_chunk_table(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "CHUNK LOOKUP TABLE",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let gray = Style::default().fg(Color::Gray);
        lines.push(Line::from(vec![
            Span::styled("  A table of contents right after the header: each entry is a", gray),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  4-byte chunk ID plus an 8-byte file offset. A terminating entry", gray),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  with ID 0 marks where the chunks end (and the checksum begins).", gray),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Chunk sizes are derived from consecutive offsets.", gray),
        ]));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled(" ID   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("│ ", gray),
            Span::styled("Hex      ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("│ ", gray),
            Span::styled("Offset ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("│ ", gray),
            Span::styled("Size   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("│ ", gray),
            Span::styled("Purpose", Style::default().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(
            "──────┼──────────┼────────┼────────┼──────────────────────────────────",
            gray,
        ));

        for chunk in &self.multi_pack_index.chunks {
            let id_style = if chunk.is_known() {
                Style::default().fg(Color::LightBlue)
            } else {
                Style::default().fg(Color::Yellow)
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {:<5}", chunk.id_str()), id_style),
                Span::styled("│ ", gray),
                Span::styled(format!("{:08x} ", chunk.id), gray),
                Span::styled("│ ", gray),
                Span::styled(
                    format!("{:6} ", chunk.offset),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("│ ", gray),
                Span::styled(
                    format!("{:6} ", chunk.size),
                    Style::default().fg(Color::LightGreen),
                ),
                Span::styled("│ ", gray),
                Span::from(chunk.description()),
            ]));
        }

        // Terminating entry: its offset is where the trailing checksum starts
        let trailer_offset = self
            .multi_pack_index
            .chunks
            .last()
            .map_or(0, |c| c.offset + c.size);
        lines.push(Line::from(vec![
            Span::styled(" ---- ", gray),
            Span::styled("│ ", gray),
            Span::styled("00000000 ", gray),
            Span::styled("│ ", gray),
            Span::styled(format!("{trailer_offset:6} "), Style::default().fg(Color::Cyan)),
            Span::styled("│ ", gray),
            Span::styled("     - ", gray),
            Span::styled("│ ", gray),
            Span::styled("(terminator)", gray),
        ]));

        lines.push(Line::from(""));
    }
}
