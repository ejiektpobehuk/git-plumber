use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

pub struct BlobFormatter<'a> {
    content: &'a [u8],
    is_binary: bool,
}

impl<'a> BlobFormatter<'a> {
    #[must_use]
    pub const fn new(content: &'a [u8], is_binary: bool) -> Self {
        Self { content, is_binary }
    }

    pub fn format_blob(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "BLOB DETAILS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(40)));
        lines.push(Line::from(""));

        lines.push(Line::from(format!(
            "Content size: {} bytes",
            self.content.len()
        )));
        lines.push(Line::from(format!(
            "Content type: {}",
            if self.is_binary { "Binary" } else { "Text" }
        )));
        lines.push(Line::from(""));

        if self.is_binary {
            self.format_binary_content(lines);
        } else {
            self.format_text_content(lines);
        }
    }

    fn format_binary_content(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "Binary Content Analysis:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(25)));
        lines.push(Line::from(""));

        // Show hex dump of first 256 bytes
        let preview_size = self.content.len().min(256);
        lines.push(Line::from(format!(
            "Hex dump (first {preview_size} bytes):"
        )));
        lines.push(Line::from(""));

        for (i, chunk) in self.content[..preview_size].chunks(16).enumerate() {
            let offset = i * 16;
            let hex_part: String = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ascii_part: String = chunk
                .iter()
                .map(|&b| {
                    if (32..=126).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();

            lines.push(Line::from(format!(
                "{offset:08x}: {hex_part:<48} |{ascii_part}|"
            )));
        }

        if self.content.len() > 256 {
            lines.push(Line::from(""));
            lines.push(Line::from("... (truncated)"));
        }
    }

    fn format_text_content(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "Text Content:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(15)));
        lines.push(Line::from(""));

        let content_str = String::from_utf8_lossy(self.content);

        // Show line count and encoding info
        let line_count = content_str.lines().count();
        lines.push(Line::from(format!("Lines: {line_count}")));
        lines.push(Line::from(""));

        // Show content (limit to first 1000 characters for performance)
        let preview_content = if content_str.len() > 1000 {
            format!(
                "{}...\n\n(Content truncated - showing first 1000 characters)",
                &content_str[..1000]
            )
        } else {
            content_str.to_string()
        };

        // Add line numbers for better readability
        for (line_num, line) in preview_content.lines().enumerate() {
            if line_num >= 50 {
                // Limit to first 50 lines
                lines.push(Line::from("... (more lines truncated)"));
                break;
            }
            lines.push(Line::from(format!("{:3}: {}", line_num + 1, line)));
        }

        if content_str.is_empty() {
            lines.push(Line::from("(empty file)"));
        }
    }
}
