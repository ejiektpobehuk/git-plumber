use crate::git::pack::MultiPackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct ObjectsFormatter<'a> {
    multi_pack_index: &'a MultiPackIndex,
}

impl<'a> ObjectsFormatter<'a> {
    #[must_use]
    pub const fn new(multi_pack_index: &'a MultiPackIndex) -> Self {
        Self { multi_pack_index }
    }

    pub fn format_objects_overview(&self, lines: &mut Vec<Line<'static>>) {
        self.format_fanout_summary(lines);
        self.format_object_table(lines);
    }

    fn format_fanout_summary(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "OID FANOUT TABLE (OIDF)",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let gray = Style::default().fg(Color::Gray);
        lines.push(Line::from(vec![Span::styled(
            "  256 cumulative counts: entry i is the number of objects whose",
            gray,
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  object ID starts with a byte <= i. Entry 255 is the total count.",
            gray,
        )]));
        lines.push(Line::from(""));

        let fan_out = &self.multi_pack_index.fan_out;
        lines.push(Line::from(vec![
            Span::from("  Total objects: "),
            Span::styled(
                format!("{}", fan_out[255]),
                Style::default().fg(Color::LightGreen),
            ),
        ]));

        // Show a few sample buckets with a non-zero increment
        let mut samples = Vec::new();
        for i in 0..256usize {
            let previous = if i == 0 { 0 } else { fan_out[i - 1] };
            if fan_out[i] > previous {
                samples.push((i, fan_out[i] - previous, fan_out[i]));
            }
            if samples.len() >= 5 {
                break;
            }
        }
        if !samples.is_empty() {
            lines.push(Line::from("  Sample buckets:"));
            for (first_byte, bucket_count, cumulative) in samples {
                lines.push(Line::from(vec![
                    Span::styled(format!("   0x{first_byte:02x}"), Style::default().fg(Color::LightBlue)),
                    Span::styled(" │ ", gray),
                    Span::from(format!("{bucket_count} object(s), {cumulative} cumulative")),
                ]));
            }
        }

        lines.push(Line::from(""));
    }

    fn format_object_table(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "OBJECTS (OIDL + OOFF)",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let gray = Style::default().fg(Color::Gray);
        lines.push(Line::from(vec![Span::styled(
            "  Object IDs in lexicographic order (OIDL), each paired with the",
            gray,
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  pack that stores it and its offset there (OOFF). Offsets with the",
            gray,
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  high bit set redirect into the large offset table (LOFF).",
            gray,
        )]));
        lines.push(Line::from(""));

        let object_count = self.multi_pack_index.object_count();
        if object_count == 0 {
            lines.push(Line::from("No objects in this multi-pack-index."));
            lines.push(Line::from(""));
            return;
        }

        lines.push(Line::from(vec![
            Span::styled(" Index", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", gray),
            Span::styled("Object ID", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("    │ ", gray),
            Span::styled("Pack", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", gray),
            Span::styled("Offset", Style::default().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(
            "──────┼──────────────┼──────┼──────────────",
            gray,
        ));

        let entries_to_show = Self::determine_sample_indices(object_count);
        for (i, show_entry) in entries_to_show.iter().enumerate() {
            if *show_entry {
                let short_oid = self
                    .multi_pack_index
                    .oid_hex_at(i)
                    .map_or_else(|| "?".to_string(), |oid| oid[..12.min(oid.len())].to_string());
                let (pack_id, offset) = self.multi_pack_index.offset_at(i).unwrap_or((0, 0));
                let is_large = self
                    .multi_pack_index
                    .object_offsets
                    .get(i)
                    .is_some_and(|object_offset| object_offset.is_large());

                let mut spans = vec![
                    Span::styled(format!(" {i:4} "), Style::default().fg(Color::Cyan)),
                    Span::styled("│ ", gray),
                    Span::styled(short_oid, Style::default().fg(Color::Yellow)),
                    Span::styled(" │ ", gray),
                    Span::styled(format!("{pack_id:4}"), Style::default().fg(Color::LightBlue)),
                    Span::styled(" │ ", gray),
                    Span::styled(format!("{offset}"), Style::default().fg(Color::LightGreen)),
                ];
                if is_large {
                    spans.push(Span::styled(" (large)", Style::default().fg(Color::Magenta)));
                }
                lines.push(Line::from(spans));
            } else if i > 0 && entries_to_show[i - 1] {
                lines.push(Line::from(vec![
                    Span::styled("  ... ", gray),
                    Span::styled("│ ", gray),
                    Span::styled("...          ", gray),
                    Span::styled("│ ", gray),
                    Span::styled(" ... ", gray),
                    Span::styled("│ ", gray),
                    Span::styled("...", gray),
                ]));
            }
        }

        lines.push(Line::from(""));

        if let Some(reverse_index) = &self.multi_pack_index.reverse_index {
            lines.push(Line::from(vec![
                Span::from("  Reverse index (RIDX) present: "),
                Span::styled(
                    format!("{} entries", reverse_index.len()),
                    Style::default().fg(Color::LightGreen),
                ),
            ]));
            lines.push(Line::from(""));
        }
    }

    /// Pick which entries of a large table to display: all when small,
    /// otherwise the first few, a couple from the middle, and the last few
    fn determine_sample_indices(object_count: usize) -> Vec<bool> {
        let mut show_entry = vec![false; object_count];

        if object_count == 0 {
            return show_entry;
        }

        if object_count <= 10 {
            show_entry.fill(true);
            return show_entry;
        }

        let sample_size = 3;

        for item in show_entry.iter_mut().take(sample_size.min(object_count)) {
            *item = true;
        }

        let last_start = object_count.saturating_sub(sample_size);
        for item in show_entry
            .iter_mut()
            .skip(last_start)
            .take(object_count - last_start)
        {
            *item = true;
        }

        if object_count > 20 {
            let mid_start = (object_count / 2).saturating_sub(1);
            let mid_end = (mid_start + 2).min(object_count);

            for item in show_entry
                .iter_mut()
                .skip(mid_start)
                .take(mid_end - mid_start)
            {
                *item = true;
            }
        }

        show_entry
    }
}
