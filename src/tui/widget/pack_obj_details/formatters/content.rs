use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

use crate::tui::widget::pack_obj_details::config::PREVIEW_SIZE_LIMIT;
use crate::tui::widget::pack_obj_details::formatters::compression::{
    Adler32Formatter, DeflateBlockFormatter, ZlibHeaderFormatter,
};

pub struct ContentFormatter<'a> {
    object_data: &'a crate::git::pack::Object,
}

impl<'a> ContentFormatter<'a> {
    pub const fn new(object_data: &'a crate::git::pack::Object) -> Self {
        Self { object_data }
    }

    pub fn format_object_content(&self, lines: &mut Vec<Line<'static>>) {
        self.format_compression_header(lines);
        self.format_zlib_header(lines);
        self.format_deflate_block(lines);
        self.format_adler32_checksum(lines);
        self.format_data_preview(lines);
    }

    fn format_compression_header(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "OBJECT DATA",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(" [zlib header][deflate blocks][checksum]"));
        lines.push(Line::from(" [2 bytes]    [variable]     [4 bytes]"));
        lines.push(Line::from(""));
        lines.push(Line::from(" [Decompressed data preview]"));
        lines.push(Line::from(""));
    }

    fn format_zlib_header(&self, lines: &mut Vec<Line<'static>>) {
        const DEFLATE_HEADER_SIZE: usize = 2;
        if self.object_data.compressed_data.len() < DEFLATE_HEADER_SIZE {
            lines.push(Line::from("No compressed data available to analyze."));
            return;
        }

        lines.push(Line::styled(
            "ZLIB COMPRESSION HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "The object data is compressed using zlib (RFC 1950):",
        ));
        lines.push(Line::from(""));

        let zlib_formatter = ZlibHeaderFormatter::new(&self.object_data.compressed_data);
        zlib_formatter.format_header(lines);
    }

    fn format_deflate_block(&self, lines: &mut Vec<Line<'static>>) {
        if self.object_data.compressed_data.len() < 3 {
            return;
        }

        lines.push(Line::styled(
            "DEFLATE BLOCK HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        let deflate_formatter = DeflateBlockFormatter::new(&self.object_data.compressed_data);
        deflate_formatter.format_block_header(lines);
    }

    fn format_adler32_checksum(&self, lines: &mut Vec<Line<'static>>) {
        if self.object_data.compressed_data.len() < 6 {
            return;
        }

        lines.push(Line::from(""));
        lines.push(Line::styled(
            "ADLER-32 CHECKSUM",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        let checksum_formatter = Adler32Formatter::new(
            &self.object_data.compressed_data,
            &self.object_data.uncompressed_data,
        );
        checksum_formatter.format_checksum(lines);
    }

    fn format_data_preview(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(""));
        lines.push(Line::from("Calculated values:"));
        lines.push(Line::from(format!(
            "  - Compressed data size: {} bytes",
            self.object_data.compressed_size
        )));
        lines.push(Line::from(format!(
            "  - Compression ratio: {:.1}%",
            (self.object_data.compressed_size as f64
                / self.object_data.uncompressed_data.len() as f64)
                * 100.0
        )));

        lines.push(Line::from(""));
        let content_str = String::from_utf8_lossy(&self.object_data.uncompressed_data);
        if content_str.len() <= PREVIEW_SIZE_LIMIT {
            lines.push(Line::styled(
                "DATA PREVIEW",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(""));
            lines.push(Line::from(content_str.to_string()));
        } else {
            lines.push(Line::styled(
                "DATA PREVIEW (truncated)",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(content_str[..PREVIEW_SIZE_LIMIT].to_string()));
            lines.push(Line::from("... (truncated)"));
        }
    }
}
