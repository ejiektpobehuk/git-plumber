use crate::git::pack::PackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

pub struct FanoutFormatter<'a> {
    pack_index: &'a PackIndex,
}

impl<'a> FanoutFormatter<'a> {
    #[must_use]
    pub const fn new(pack_index: &'a PackIndex) -> Self {
        Self { pack_index }
    }

    /// Calculate the 1-based byte position in the .idx file for a fanout entry
    const fn calculate_fanout_byte_position(&self, fanout_index: usize) -> u64 {
        let header_size = 8u64; // magic (4) + version (4)
        let fanout_start = header_size; // Fan-out table starts at byte 8 (0-based)

        // Each fanout entry is 4 bytes
        // Add 1 to convert from 0-based to 1-based indexing
        fanout_start + (fanout_index as u64 * 4) + 1
    }

    pub fn format_fanout_table(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "FAN-OUT TABLE",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(20)));
        lines.push(Line::from(""));

        lines.push(Line::from(
            "Helps to minimize the number of objects to search through.",
        ));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "Structure: 256 entries (one per possible first byte 0x00-0xFF)",
        ));
        lines.push(Line::from(
            "Each entry: Cumulative count of objects with first byte ≤ index",
        ));
        lines.push(Line::from(""));

        // Show distribution statistics
        self.add_distribution_stats(lines);

        // Show some sample entries
        self.add_sample_entries(lines);

        // Show search optimization explanation
        self.add_search_explanation(lines);
    }

    fn add_distribution_stats(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "Distribution Analysis:",
            Style::default().add_modifier(Modifier::BOLD),
        ));

        let mut non_empty_buckets = 0;
        let mut max_bucket_size = 0;
        let mut prev_count = 0;

        for &count in &self.pack_index.fan_out {
            let bucket_size = count - prev_count;
            if bucket_size > 0 {
                non_empty_buckets += 1;
                max_bucket_size = max_bucket_size.max(bucket_size);
            }
            prev_count = count;
        }

        lines.push(Line::from(format!(
            "• Non-empty buckets: {non_empty_buckets} / 256"
        )));
        lines.push(Line::from(format!(
            "• Largest bucket: {max_bucket_size} objects"
        )));
        lines.push(Line::from(""));
    }

    fn add_sample_entries(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "Complete Fanout Table:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        // Table header
        lines.push(Line::from(
            "┌──────┬──────┬─────────────┬─────────┬────────┐",
        ));
        lines.push(Line::styled(
            "│ Byte │ Index│ Hex Value   │ Decimal │ Bucket │",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(
            "├──────┼──────┼─────────────┼─────────┼────────┤",
        ));

        let entries_to_show = self.determine_entries_to_show();
        let mut prev_count = 0;
        let mut last_shown_index = None;

        for (i, show_entry) in entries_to_show.iter().enumerate() {
            let count = self.pack_index.fan_out[i];

            if *show_entry {
                // Check if we need to show a skip indicator
                if let Some(last_idx) = last_shown_index
                    && i > last_idx + 1
                {
                    let line_parts = vec![
                        ("│  ".to_string(), Style::default()),
                        ("...".to_string(), Style::default().fg(Color::Gray)),
                        (" │  ".to_string(), Style::default()),
                        ("...".to_string(), Style::default().fg(Color::Gray)),
                        (" │    ".to_string(), Style::default()),
                        ("....".to_string(), Style::default().fg(Color::Gray)),
                        ("     │   ".to_string(), Style::default()),
                        ("...".to_string(), Style::default().fg(Color::Gray)),
                        ("   │  ".to_string(), Style::default()),
                        ("...".to_string(), Style::default().fg(Color::Gray)),
                        ("   │".to_string(), Style::default()),
                    ];

                    let mut line = Line::default();
                    for (text, style) in line_parts {
                        line.spans.push(ratatui::text::Span::styled(text, style));
                    }
                    lines.push(line);
                }

                let bucket_size = count - prev_count;
                let byte_pos = self.calculate_fanout_byte_position(i);
                let hex_value = self.format_hex_value(count);

                if bucket_size > 0 {
                    // Active row: different colors for different data types
                    let line_parts = vec![
                        ("│ ".to_string(), Style::default()),
                        (format!("{byte_pos:4}"), Style::default().fg(Color::Cyan)),
                        (" │ ".to_string(), Style::default()),
                        (format!("0x{i:02x}"), Style::default().fg(Color::Cyan)),
                        (" │ ".to_string(), Style::default()),
                        (hex_value, Style::default().fg(Color::Blue)),
                        (" │ ".to_string(), Style::default()),
                        (format!("{count:7}"), Style::default().fg(Color::Cyan)),
                        (" │ ".to_string(), Style::default()),
                        (format!("{bucket_size:6}"), Style::default().fg(Color::Cyan)),
                        (" │".to_string(), Style::default()),
                    ];

                    let mut line = Line::default();
                    for (text, style) in line_parts {
                        line.spans.push(ratatui::text::Span::styled(text, style));
                    }
                    lines.push(line);
                } else {
                    // Inactive row: unstyled borders, gray content
                    let line_parts = vec![
                        ("│ ".to_string(), Style::default()),
                        (format!("{byte_pos:4}"), Style::default().fg(Color::Gray)),
                        (" │ ".to_string(), Style::default()),
                        (format!("0x{i:02x}"), Style::default().fg(Color::Gray)),
                        (" │ ".to_string(), Style::default()),
                        (hex_value, Style::default().fg(Color::Gray)),
                        (" │ ".to_string(), Style::default()),
                        (format!("{count:7}"), Style::default().fg(Color::Gray)),
                        (" │ ".to_string(), Style::default()),
                        (format!("{bucket_size:6}"), Style::default().fg(Color::Gray)),
                        (" │".to_string(), Style::default()),
                    ];

                    let mut line = Line::default();
                    for (text, style) in line_parts {
                        line.spans.push(ratatui::text::Span::styled(text, style));
                    }
                    lines.push(line);
                }

                last_shown_index = Some(i);
            }

            prev_count = count;
        }

        lines.push(Line::from(
            "└──────┴──────┴─────────────┴─────────┴────────┘",
        ));
        lines.push(Line::from(""));

        // Add legend
        self.add_table_legend(lines);
    }

    fn add_table_legend(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "Column Legend:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "• Byte:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ));
        lines.push(Line::from(
            "  Position in the .idx file of the first byte out of 4 for the entry",
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "• Index:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ));
        lines.push(Line::from(
            "  Corresponds to the first byte value (0x00-0xFF) of the object",
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "• Hex Value:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ));
        lines.push(Line::from("  raw value of the fanout table entry."));
        lines.push(Line::from(
            "  represents the number of objects with first byte ≤ this index.",
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "• Decimal:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ));
        lines.push(Line::from(
            "  Converted cumulative count of objects with first byte ≤ this index",
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "• Bucket:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ));
        lines.push(Line::from(
            "  Number of objects with first byte exactly equal to this index",
        ));
        lines.push(Line::from(
            "  (calculated as: current_count - previous_count)",
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "Color Coding:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            "• Blue: Raw data from the fanout table (hex values)",
            Style::default().fg(Color::Blue),
        ));
        lines.push(Line::styled(
            "• Cyan: Calculated values for non-empty buckets",
            Style::default().fg(Color::Cyan),
        ));
        lines.push(Line::styled(
            "• Gray: Empty buckets (no objects)",
            Style::default().fg(Color::Gray),
        ));
        lines.push(Line::from(""));
    }

    /// Determines which fanout entries should be shown based on smart skipping logic
    fn determine_entries_to_show(&self) -> Vec<bool> {
        let mut show_entry = vec![false; 256];
        let mut prev_count = 0;

        // First pass: identify active (non-empty) entries
        let mut active_entries = vec![false; 256];
        for (i, &count) in self.pack_index.fan_out.iter().enumerate() {
            let bucket_size = count - prev_count;
            if bucket_size > 0 {
                active_entries[i] = true;
            }
            prev_count = count;
        }

        // Always show first 2 and last 2 entries
        show_entry[0] = true;
        show_entry[1] = true;
        show_entry[254] = true;
        show_entry[255] = true;

        // Show active entries and their context (1 before, 1 after)
        for i in 0..256 {
            if active_entries[i] {
                // Show the active entry itself
                show_entry[i] = true;

                // Show 1 entry before (if exists and not already shown)
                if i > 0 {
                    show_entry[i - 1] = true;
                }

                // Show 1 entry after (if exists and not already shown)
                if i < 255 {
                    show_entry[i + 1] = true;
                }
            }
        }

        show_entry
    }

    /// Format a 32-bit value as hex bytes (4 bytes, big-endian)
    fn format_hex_value(&self, value: u32) -> String {
        format!(
            "{:02x} {:02x} {:02x} {:02x}",
            (value >> 24) & 0xff,
            (value >> 16) & 0xff,
            (value >> 8) & 0xff,
            value & 0xff
        )
    }

    fn add_search_explanation(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "Binary Search Optimization:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        lines.push(Line::from("To find object with SHA-1 starting with 0x42:"));
        lines.push(Line::from(""));

        let first_byte = 0x42;
        let start_idx = if first_byte == 0 {
            0
        } else {
            self.pack_index.fan_out[first_byte - 1]
        };
        let end_idx = self.pack_index.fan_out[first_byte];
        let range_size = end_idx - start_idx;

        lines.push(Line::styled(
            format!("1. Check fan_out[0x41] = {start_idx} (start of range)"),
            Style::default().fg(Color::Yellow),
        ));
        lines.push(Line::styled(
            format!("2. Check fan_out[0x42] = {end_idx} (end of range)"),
            Style::default().fg(Color::Yellow),
        ));
        lines.push(Line::styled(
            format!(
                "3. Binary search within {} objects (indices {}-{})",
                range_size,
                start_idx,
                end_idx - 1
            ),
            Style::default().fg(Color::Green),
        ));
        lines.push(Line::from(""));

        let total_objects = self.pack_index.object_count();
        let search_reduction = if total_objects > 0 {
            (f64::from(range_size) / total_objects as f64) * 100.0
        } else {
            0.0
        };

        lines.push(Line::styled(
            format!("Search space reduced to {search_reduction:.1}% of total objects!"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));
    }
}
