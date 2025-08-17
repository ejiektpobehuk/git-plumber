use crate::git::pack::PackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

pub struct ObjectsFormatter<'a> {
    pack_index: &'a PackIndex,
}

impl<'a> ObjectsFormatter<'a> {
    pub fn new(pack_index: &'a PackIndex) -> Self {
        Self { pack_index }
    }

    /// Calculate the 1-based byte position in the .idx file for a given object entry
    ///
    /// Pack index file structure:
    /// - Header: 8 bytes (magic + version)
    /// - Fan-out table: 256 * 4 = 1024 bytes
    /// - Object names: N * 20 bytes (where N = object count)
    /// - CRC32 checksums: N * 4 bytes
    /// - Offsets: N * 4 bytes
    /// - Large offsets: variable (if any)
    /// - Pack checksum: 20 bytes
    /// - Index checksum: 20 bytes
    fn calculate_object_byte_position(&self, object_index: usize) -> u64 {
        let header_size = 8u64; // magic (4) + version (4)
        let fanout_size = 256 * 4u64; // 256 entries * 4 bytes each
        let object_names_start = header_size + fanout_size;

        // Each object name is 20 bytes
        // Add 1 to convert from 0-based to 1-based indexing
        object_names_start + (object_index as u64 * 20) + 1
    }

    /// Calculate the 1-based byte position in the .idx file for a CRC32 entry
    fn calculate_crc32_byte_position(&self, object_index: usize) -> u64 {
        let header_size = 8u64; // magic (4) + version (4)
        let fanout_size = 256 * 4u64; // 256 entries * 4 bytes each
        let object_names_size = self.pack_index.object_count() as u64 * 20; // N * 20 bytes
        let crc32_start = header_size + fanout_size + object_names_size;

        // Each CRC32 is 4 bytes
        // Add 1 to convert from 0-based to 1-based indexing
        crc32_start + (object_index as u64 * 4) + 1
    }

    /// Calculate the 1-based byte position in the .idx file for an offset entry
    fn calculate_offset_byte_position(&self, object_index: usize) -> u64 {
        let header_size = 8u64; // magic (4) + version (4)
        let fanout_size = 256 * 4u64; // 256 entries * 4 bytes each
        let object_names_size = self.pack_index.object_count() as u64 * 20; // N * 20 bytes
        let crc32_size = self.pack_index.object_count() as u64 * 4; // N * 4 bytes
        let offsets_start = header_size + fanout_size + object_names_size + crc32_size;

        // Each offset is 4 bytes
        // Add 1 to convert from 0-based to 1-based indexing
        offsets_start + (object_index as u64 * 4) + 1
    }

    pub fn format_objects_overview(&self, lines: &mut Vec<Line<'static>>) {
        self.format_object_names_table(lines);
        self.format_crc32_table(lines);
        self.format_offsets_table(lines);
    }

    fn format_object_names_table(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "OBJECT NAMES TABLE",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(25)));
        lines.push(Line::from(""));

        lines.push(Line::from(
            "SHA-1 hashes sorted lexicographically (20 bytes each)",
        ));
        lines.push(Line::from(format!(
            "Total objects: {}",
            self.pack_index.object_count()
        )));
        lines.push(Line::from(""));

        if self.pack_index.object_names.is_empty() {
            lines.push(Line::from("No objects in this index."));
            lines.push(Line::from(""));
            return;
        }

        // Table header
        lines.push(Line::from(
            "┌──────┬──────────────────────────────────────────┐",
        ));
        lines.push(Line::styled(
            "│ Byte │ SHA-1 Hash                               │",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(
            "├──────┼──────────────────────────────────────────┤",
        ));

        let entries_to_show = self.determine_sample_indices();

        for (i, show_entry) in entries_to_show.iter().enumerate() {
            if *show_entry {
                let hash = hex::encode(self.pack_index.object_names[i]);
                let byte_pos = self.calculate_object_byte_position(i);

                let line_parts = vec![
                    ("│ ".to_string(), Style::default()),
                    (format!("{:4}", byte_pos), Style::default().fg(Color::Cyan)),
                    (" │ ".to_string(), Style::default()),
                    (hash, Style::default().fg(Color::Yellow)),
                    (" │".to_string(), Style::default()),
                ];

                let mut line = Line::default();
                for (text, style) in line_parts {
                    line.spans.push(ratatui::text::Span::styled(text, style));
                }
                lines.push(line);
            } else if i > 0 && entries_to_show[i - 1] {
                // Show ellipsis after last shown entry
                let line_parts = vec![
                    ("│  ".to_string(), Style::default()),
                    ("...".to_string(), Style::default().fg(Color::Gray)),
                    (" │ ".to_string(), Style::default()),
                    (
                        "...                                     ".to_string(),
                        Style::default().fg(Color::Gray),
                    ),
                    (" │".to_string(), Style::default()),
                ];

                let mut line = Line::default();
                for (text, style) in line_parts {
                    line.spans.push(ratatui::text::Span::styled(text, style));
                }
                lines.push(line);
            }
        }

        lines.push(Line::from(
            "└──────┴──────────────────────────────────────────┘",
        ));
        lines.push(Line::from(""));
    }

    fn format_crc32_table(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "CRC32 CHECKSUMS TABLE",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        lines.push(Line::from(
            "CRC32 checksums for integrity verification (4 bytes each)",
        ));
        lines.push(Line::from(format!(
            "Total checksums: {}",
            self.pack_index.crc32_checksums.len()
        )));
        lines.push(Line::from(""));

        if self.pack_index.crc32_checksums.is_empty() {
            lines.push(Line::from("No CRC32 checksums available."));
            lines.push(Line::from(""));
            return;
        }

        // Table header
        lines.push(Line::from("┌──────┬─────────────┐"));
        lines.push(Line::styled(
            "│ Byte │ Hex Bytes   │",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("├──────┼─────────────┤"));

        let entries_to_show = self.determine_sample_indices();

        for (i, show_entry) in entries_to_show.iter().enumerate() {
            if *show_entry {
                let crc32 = self.pack_index.crc32_checksums[i];
                let byte_pos = self.calculate_crc32_byte_position(i);
                let hex_bytes = format!(
                    "{:02x} {:02x} {:02x} {:02x}",
                    (crc32 >> 24) & 0xff,
                    (crc32 >> 16) & 0xff,
                    (crc32 >> 8) & 0xff,
                    crc32 & 0xff
                );

                let line_parts = vec![
                    ("│ ".to_string(), Style::default()),
                    (format!("{:4}", byte_pos), Style::default().fg(Color::Cyan)),
                    (" │ ".to_string(), Style::default()),
                    (
                        format!("{:11}", hex_bytes),
                        Style::default().fg(Color::Blue),
                    ),
                    (" │".to_string(), Style::default()),
                ];

                let mut line = Line::default();
                for (text, style) in line_parts {
                    line.spans.push(ratatui::text::Span::styled(text, style));
                }
                lines.push(line);
            } else if i > 0 && entries_to_show[i - 1] {
                // Show ellipsis after last shown entry
                let line_parts = vec![
                    ("│  ".to_string(), Style::default()),
                    ("...".to_string(), Style::default().fg(Color::Gray)),
                    (" │       ".to_string(), Style::default()),
                    ("...".to_string(), Style::default().fg(Color::Gray)),
                    ("   │".to_string(), Style::default()),
                ];

                let mut line = Line::default();
                for (text, style) in line_parts {
                    line.spans.push(ratatui::text::Span::styled(text, style));
                }
                lines.push(line);
            }
        }

        lines.push(Line::from("└──────┴─────────────┘"));
        lines.push(Line::from(""));
    }

    fn format_offsets_table(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "PACK FILE OFFSETS TABLE",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        lines.push(Line::from("Pack file byte positions (4 bytes each)"));
        lines.push(Line::from(format!(
            "Total offsets: {}",
            self.pack_index.offsets.len()
        )));

        // Show large offset info if present
        let large_offset_count = self
            .pack_index
            .offsets
            .iter()
            .filter(|&&offset| offset & 0x80000000 != 0)
            .count();

        if large_offset_count > 0 {
            lines.push(Line::styled(
                format!(
                    "Large offsets: {} (MSB set, using 8-byte table)",
                    large_offset_count
                ),
                Style::default().fg(Color::Yellow),
            ));
        }
        lines.push(Line::from(""));

        if self.pack_index.offsets.is_empty() {
            lines.push(Line::from("No offsets available."));
            lines.push(Line::from(""));
            return;
        }

        // Table header
        lines.push(Line::from("┌──────┬─────────────┬───────────┐"));
        lines.push(Line::styled(
            "│ Byte │ Hex Bytes   │ Offset    │",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("├──────┼─────────────┼───────────┤"));

        let entries_to_show = self.determine_sample_indices();

        for (i, show_entry) in entries_to_show.iter().enumerate() {
            if *show_entry {
                let raw_offset = self.pack_index.offsets[i];
                let actual_offset = self.pack_index.get_object_offset(i);
                let byte_pos = self.calculate_offset_byte_position(i);
                let hex_bytes = format!(
                    "{:02x} {:02x} {:02x} {:02x}",
                    (raw_offset >> 24) & 0xff,
                    (raw_offset >> 16) & 0xff,
                    (raw_offset >> 8) & 0xff,
                    raw_offset & 0xff
                );

                let offset_display = if raw_offset & 0x80000000 != 0 {
                    format!("{:>8} L", actual_offset) // L for Large offset
                } else {
                    format!("{:>9}", actual_offset)
                };

                let line_parts = vec![
                    ("│ ".to_string(), Style::default()),
                    (format!("{:4}", byte_pos), Style::default().fg(Color::Cyan)),
                    (" │ ".to_string(), Style::default()),
                    (
                        format!("{:10}", hex_bytes),
                        Style::default().fg(Color::Blue),
                    ),
                    (" │ ".to_string(), Style::default()),
                    (
                        offset_display,
                        if raw_offset & 0x80000000 != 0 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::Magenta)
                        },
                    ),
                    (" │".to_string(), Style::default()),
                ];

                let mut line = Line::default();
                for (text, style) in line_parts {
                    line.spans.push(ratatui::text::Span::styled(text, style));
                }
                lines.push(line);
            } else if i > 0 && entries_to_show[i - 1] {
                // Show ellipsis after last shown entry
                let line_parts = vec![
                    ("│  ".to_string(), Style::default()),
                    ("...".to_string(), Style::default().fg(Color::Gray)),
                    (" │     ".to_string(), Style::default()),
                    ("...".to_string(), Style::default().fg(Color::Gray)),
                    ("     │   ".to_string(), Style::default()),
                    ("...".to_string(), Style::default().fg(Color::Gray)),
                    ("     │".to_string(), Style::default()),
                ];

                let mut line = Line::default();
                for (text, style) in line_parts {
                    line.spans.push(ratatui::text::Span::styled(text, style));
                }
                lines.push(line);
            }
        }

        lines.push(Line::from("└──────┴─────────────┴───────────┘"));
        lines.push(Line::from(""));

        // Add large offset table info if present
        if let Some(ref large_offsets) = self.pack_index.large_offsets {
            self.add_large_offsets_info(lines, large_offsets);
        }
    }

    fn add_large_offsets_info(&self, lines: &mut Vec<Line<'static>>, large_offsets: &[u64]) {
        lines.push(Line::styled(
            "Large Offset Table (8-byte offsets for pack files > 4GB):",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(format!("Entries: {}", large_offsets.len())));

        if !large_offsets.is_empty() {
            let min_large = large_offsets.iter().min().unwrap_or(&0);
            let max_large = large_offsets.iter().max().unwrap_or(&0);
            lines.push(Line::from(format!(
                "Range: {} - {} bytes",
                min_large, max_large
            )));
        }
        lines.push(Line::from(""));
    }

    /// Determines which object indices should be shown in tables
    /// Shows first few, last few, and some from the middle if there are many objects
    fn determine_sample_indices(&self) -> Vec<bool> {
        let object_count = self.pack_index.object_count();
        let mut show_entry = vec![false; object_count];

        if object_count == 0 {
            return show_entry;
        }

        // For small collections, show all entries
        if object_count <= 10 {
            show_entry.fill(true);
            return show_entry;
        }

        // For larger collections, show strategic samples
        let sample_size = 3; // Show first 3, last 3, and some from middle

        // Always show first few entries
        for item in show_entry.iter_mut().take(sample_size.min(object_count)) {
            *item = true;
        }

        // Always show last few entries
        let last_start = object_count.saturating_sub(sample_size);
        for item in show_entry
            .iter_mut()
            .skip(last_start)
            .take(object_count - last_start)
        {
            *item = true;
        }

        // Show some from the middle if we have enough objects
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
