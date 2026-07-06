use crate::git::pack::{PackBitmap, bitmap::EwahBitmap};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[must_use]
pub fn format_type_bitmaps(bitmap: &PackBitmap) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(
            "TYPE INDEX BITMAPS",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from("─".repeat(30)),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Four EWAH-compressed bitmaps, one per object type. Bit n is set",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  when the n-th object (in pack/MIDX order) has that type, so the",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  set-bit counts below are the object counts per type.",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Byte", Style::default().fg(Color::Gray)),
            Span::styled("  │ ", Style::default().fg(Color::Gray)),
            Span::styled("Type   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled("Set bits", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                "Stored bytes",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            "───────┼─────────┼──────────┼─────────────",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let type_bitmaps: [(&str, &EwahBitmap); 4] = [
        ("Commits", &bitmap.commits_bitmap),
        ("Trees", &bitmap.trees_bitmap),
        ("Blobs", &bitmap.blobs_bitmap),
        ("Tags", &bitmap.tags_bitmap),
    ];

    let mut byte_position = 12 + bitmap.checksum_size;
    for (name, ewah) in type_bitmaps {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {byte_position:5}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{name:<7}"), Style::default().fg(Color::LightBlue)),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:8}", ewah.count_set_bits()),
                Style::default().fg(Color::LightGreen),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", ewah.compressed_byte_size()),
                Style::default().fg(Color::LightGreen),
            ),
        ]));
        byte_position += ewah.compressed_byte_size();
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Objects covered: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", bitmap.object_count()),
            Style::default().fg(Color::LightGreen),
        ),
        Span::styled(
            " (each object is of exactly one type)",
            Style::default().fg(Color::Gray),
        ),
    ]));
    lines.push(Line::from(""));

    lines
}
