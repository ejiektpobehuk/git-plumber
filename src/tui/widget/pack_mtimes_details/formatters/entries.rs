use crate::git::pack::PackMtimes;
use crate::tui::widget::formatters_utils::{format_epoch_utc, format_u32_as_hex_bytes};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[must_use]
pub fn format_entries(mtimes: &PackMtimes) -> Vec<Line<'static>> {
    let mut lines = vec![
        // Section title
        Line::styled(
            "Table of modification times",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from("─".repeat(27)),
        Line::from(""),
        // Column descriptions
        Line::from(vec![
            Span::styled("  • Byte: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "index of the first byte of 4-byte mtime record",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Index: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "records follow object ID order from the .idx file",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Raw Hex: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "actual value of a 4-byte mtime record",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Date: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "the record as seconds since the Unix epoch, in UTC",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
        // Table header
        Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled("Index", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │   ", Style::default().fg(Color::Gray)),
            Span::styled("Raw Hex", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("   │ ", Style::default().fg(Color::Gray)),
            Span::styled("Date", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![Span::styled(
            "──────┼───────┼─────────────┼────────────────────────",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let total_objects = mtimes.object_count();

    for index_pos in 0..total_objects {
        if let Some(mtime) = mtimes.mtime_at(index_pos) {
            let header_size = 12;
            let record_size = 4;
            let byte_position = header_size + 1 + index_pos * record_size;
            let hex_value = format_u32_as_hex_bytes(mtime);

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {byte_position:4}"),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(" │ ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("0x{index_pos:02x}"),
                    Style::default().fg(Color::LightBlue),
                ),
                Span::styled("  │ ", Style::default().fg(Color::Gray)),
                Span::styled(hex_value, Style::default().fg(Color::LightGreen)),
                Span::styled(" │ ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_epoch_utc(mtime),
                    Style::default().fg(Color::LightGreen),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));

    lines
}
