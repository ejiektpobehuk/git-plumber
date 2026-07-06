use crate::git::pack::PackBitmap;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

#[must_use]
pub fn format_entries(bitmap: &PackBitmap) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::styled(
            "COMMIT REACHABILITY BITMAPS",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from("─".repeat(30)),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  One entry per selected commit. Bit n of an entry's bitmap is set",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![Span::styled(
            "  when the n-th object is reachable from that commit.",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  • Pos: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "the commit's position in the pack index / MIDX",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • XOR: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "0 = stored verbatim, y = XORed with the entry y rows above",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Set bits: ", Style::default().fg(Color::Gray)),
            Span::styled(
                "of the stored (possibly XOR-encoded) bitmap, not the final one",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(""),
    ];

    if bitmap.entries.is_empty() {
        lines.push(Line::from("  (no commit bitmaps)"));
        lines.push(Line::from(""));
        return lines;
    }

    lines.push(Line::from(vec![
        Span::styled(" Byte", Style::default().fg(Color::Gray)),
        Span::styled("   │ ", Style::default().fg(Color::Gray)),
        Span::styled("Pos", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("      │ ", Style::default().fg(Color::Gray)),
        Span::styled("XOR", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::Gray)),
        Span::styled("Flags", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::Gray)),
        Span::styled("Set bits", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::Gray)),
        Span::styled("Bytes", Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![Span::styled(
        "────────┼──────────┼─────┼───────┼──────────┼──────",
        Style::default().fg(Color::Gray),
    )]));

    // Entries start after the header, pack checksum and 4 type bitmaps
    let mut byte_position = 12
        + bitmap.checksum_size
        + [
            &bitmap.commits_bitmap,
            &bitmap.trees_bitmap,
            &bitmap.blobs_bitmap,
            &bitmap.tags_bitmap,
        ]
        .iter()
        .map(|b| b.compressed_byte_size())
        .sum::<usize>();

    for entry in &bitmap.entries {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {byte_position:6}"),
                Style::default().fg(Color::Gray),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:8}", entry.object_pos),
                Style::default().fg(Color::LightBlue),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:3}", entry.xor_offset),
                Style::default().fg(Color::LightGreen),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("0x{:02x} ", entry.flags),
                Style::default().fg(Color::LightGreen),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:8}", entry.bitmap.count_set_bits()),
                Style::default().fg(Color::LightGreen),
            ),
            Span::styled(" │ ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", entry.bitmap.compressed_byte_size()),
                Style::default().fg(Color::LightGreen),
            ),
        ]));
        // 4-byte position + 1-byte xor offset + 1-byte flags + the bitmap
        byte_position += 6 + entry.bitmap.compressed_byte_size();
    }

    lines.push(Line::from(""));

    lines
}
