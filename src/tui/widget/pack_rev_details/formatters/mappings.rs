use crate::git::pack::PackReverseIndex;
use crate::tui::widget::formatters_utils::format_u32_as_hex_bytes;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn format_mappings(reverse_index: &PackReverseIndex) -> Vec<Line<'static>> {
    let mut lines = vec![
        // Section title
        Line::styled(
            "Table of index positions",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from("─".repeat(24)),
        Line::from(""),
        // Column descriptions
        Line::from(vec![
            Span::styled("  • Byte: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "index of the first byte of 4-byte index record",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Offset: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Objects are sorted by their offsets in the packfile",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Raw Hex: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "actual value of a 4-byte index record",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Decimal: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "human friendly form of an index record",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        // Table header
        Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled("Offset", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │   ", Style::default().fg(Color::Gray)),
            Span::styled("Raw Hex", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("   │ ", Style::default().fg(Color::Gray)),
            Span::styled("Decimal", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![Span::styled(
            "──────┼────────┼─────────────┼─────────",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let total_objects = reverse_index.object_count();

    for pack_pos in 0..total_objects {
        if let Some(offset_pos) = reverse_index.pack_pos_to_index(pack_pos) {
            let header_size = 12;
            let index_size = 4;
            let byte_position = header_size + 1 + pack_pos * index_size;
            let hex_value = format_u32_as_hex_bytes(offset_pos);

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {byte_position:4}"),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(" │  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("0x{pack_pos:02x}"),
                    Style::default().fg(Color::LightBlue),
                ),
                Span::styled("  │ ", Style::default().fg(Color::Gray)),
                Span::styled(hex_value, Style::default().fg(Color::LightGreen)),
                Span::styled(" │", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{offset_pos:8}"),
                    Style::default().fg(Color::LightGreen),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));

    lines
}
