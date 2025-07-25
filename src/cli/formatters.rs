use crate::git::loose_object::LooseObject;
/// CLI formatters that reuse TUI formatting logic for consistent output
use crate::git::pack::{Object, ObjectHeader};
use crate::tui::model::PackObject;
use crate::tui::widget::loose_obj_details::LooseObjectWidget;
use crate::tui::widget::pack_obj_details::PackObjectWidget;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use sha1::Digest;
use std::fmt::Write;

pub struct CliPackFormatter;
pub struct CliLooseFormatter;

impl CliPackFormatter {
    /// Format a complete pack file with header and all objects
    #[must_use]
    pub fn format_pack_file(header: &crate::git::pack::Header, objects: &[Object]) -> String {
        let mut output = String::new();

        // Format pack header using educational content system
        Self::format_pack_header(&mut output, header);
        writeln!(&mut output).unwrap();

        // Format each object using TUI formatters
        for (i, object) in objects.iter().enumerate() {
            if i > 0 {
                writeln!(&mut output, "{}", "═".repeat(80)).unwrap();
            }
            Self::format_pack_object(&mut output, object, i + 1);
        }

        output
    }

    /// Format pack file header information reusing educational content
    fn format_pack_header(output: &mut String, header: &crate::git::pack::Header) {
        // Use the educational content system for pack header preview
        let educational_provider = crate::educational_content::EducationalContent::new();
        let header_preview = educational_provider.get_pack_preview(header);

        // Convert ratatui Text to ANSI colored string
        let colored_text = Self::text_to_ansi_string(&header_preview);
        writeln!(output, "\x1b[1mPACK FILE HEADER\x1b[0m").unwrap();
        writeln!(output, "{}", "─".repeat(50)).unwrap();
        writeln!(output).unwrap();
        writeln!(output, "{colored_text}").unwrap();
    }

    /// Format a single pack object using TUI formatters
    fn format_pack_object(output: &mut String, object: &Object, index: usize) {
        writeln!(output).unwrap();
        writeln!(output, "\x1b[1mOBJECT #{index}\x1b[0m").unwrap();
        writeln!(output, "{}", "─".repeat(40)).unwrap();
        writeln!(output).unwrap();

        // Create a PackObject from the Object (similar to what TUI loaders do)
        let pack_obj = Self::create_pack_object_from_object(object, index);

        // Use the TUI formatter to generate rich content
        let mut widget = PackObjectWidget::new(pack_obj);
        let formatted_text = widget.text();

        // Convert ratatui Text to ANSI colored string
        let colored_text = Self::text_to_ansi_string(&formatted_text);
        writeln!(output, "{colored_text}").unwrap();
    }

    /// Convert ratatui Text to ANSI colored string, preserving styling
    #[must_use]
    pub fn text_to_ansi_string(text: &Text) -> String {
        let mut result = String::new();

        for line in &text.lines {
            for span in &line.spans {
                // Convert ratatui style to ANSI escape codes
                let ansi_start = Self::style_to_ansi_start(&span.style);
                let ansi_end = if span.style == Style::default() {
                    ""
                } else {
                    "\x1b[0m" // Reset
                };

                write!(&mut result, "{}{}{}", ansi_start, span.content, ansi_end).unwrap();
            }
            result.push('\n');
        }

        // Remove trailing newline if present
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Convert ratatui Style to ANSI escape sequence
    fn style_to_ansi_start(style: &Style) -> String {
        let mut ansi = String::new();

        // Handle foreground color
        if let Some(color) = style.fg {
            ansi.push_str(&Self::color_to_ansi(color, true));
        }

        // Handle background color
        if let Some(color) = style.bg {
            ansi.push_str(&Self::color_to_ansi(color, false));
        }

        // Handle modifiers
        if style.add_modifier.contains(Modifier::BOLD) {
            ansi.push_str("\x1b[1m");
        }
        if style.add_modifier.contains(Modifier::DIM) {
            ansi.push_str("\x1b[2m");
        }
        if style.add_modifier.contains(Modifier::ITALIC) {
            ansi.push_str("\x1b[3m");
        }
        if style.add_modifier.contains(Modifier::UNDERLINED) {
            ansi.push_str("\x1b[4m");
        }
        if style.add_modifier.contains(Modifier::SLOW_BLINK) {
            ansi.push_str("\x1b[5m");
        }
        if style.add_modifier.contains(Modifier::RAPID_BLINK) {
            ansi.push_str("\x1b[6m");
        }
        if style.add_modifier.contains(Modifier::REVERSED) {
            ansi.push_str("\x1b[7m");
        }
        if style.add_modifier.contains(Modifier::HIDDEN) {
            ansi.push_str("\x1b[8m");
        }
        if style.add_modifier.contains(Modifier::CROSSED_OUT) {
            ansi.push_str("\x1b[9m");
        }

        ansi
    }

    /// Convert ratatui Color to ANSI color code
    fn color_to_ansi(color: Color, is_foreground: bool) -> String {
        let base = if is_foreground { 30 } else { 40 };

        match color {
            Color::Reset => "\x1b[0m".to_string(),
            Color::Black => format!("\x1b[{base}m"),
            Color::Red => format!("\x1b[{}m", base + 1),
            Color::Green => format!("\x1b[{}m", base + 2),
            Color::Yellow => format!("\x1b[{}m", base + 3),
            Color::Blue => format!("\x1b[{}m", base + 4),
            Color::Magenta => format!("\x1b[{}m", base + 5),
            Color::Cyan => format!("\x1b[{}m", base + 6),
            Color::Gray | Color::White => format!("\x1b[{}m", base + 7),
            Color::DarkGray => format!("\x1b[{}m", base + 60), // Bright black
            Color::LightRed => format!("\x1b[{}m", base + 61),
            Color::LightGreen => format!("\x1b[{}m", base + 62),
            Color::LightYellow => format!("\x1b[{}m", base + 63),
            Color::LightBlue => format!("\x1b[{}m", base + 64),
            Color::LightMagenta => format!("\x1b[{}m", base + 65),
            Color::LightCyan => format!("\x1b[{}m", base + 66),
            Color::Rgb(r, g, b) => {
                if is_foreground {
                    format!("\x1b[38;2;{r};{g};{b}m")
                } else {
                    format!("\x1b[48;2;{r};{g};{b}m")
                }
            }
            Color::Indexed(i) => {
                if is_foreground {
                    format!("\x1b[38;5;{i}m")
                } else {
                    format!("\x1b[48;5;{i}m")
                }
            }
        }
    }

    /// Create a `PackObject` from an Object (similar to TUI loader logic)
    fn create_pack_object_from_object(object: &Object, index: usize) -> PackObject {
        let obj_type = object.header.obj_type();
        let size = object.header.uncompressed_data_size();

        // Calculate SHA-1 hash like the TUI does
        let mut hasher = sha1::Sha1::new();
        let header = format!("{obj_type} {size}\0");
        hasher.update(header.as_bytes());
        hasher.update(&object.uncompressed_data);
        let sha1 = Some(format!("{:x}", hasher.finalize()));

        // Extract base info for delta objects
        let base_info = match &object.header {
            ObjectHeader::OfsDelta { base_offset, .. } => {
                Some(format!("Base offset: {base_offset}"))
            }
            ObjectHeader::RefDelta { base_ref, .. } => {
                Some(format!("Base ref: {}", hex::encode(base_ref)))
            }
            ObjectHeader::Regular { .. } => None,
        };

        PackObject {
            index,
            obj_type: obj_type.to_string(),
            size: u32::try_from(size).unwrap_or(u32::MAX),
            sha1,
            base_info,
            object_data: Some(object.clone()),
        }
    }
}

impl CliLooseFormatter {
    /// Format a loose object with rich formatting using TUI formatters
    #[must_use]
    pub fn format_loose_object(loose_obj: &LooseObject) -> String {
        let mut output = String::new();

        // Format loose object header
        writeln!(&mut output, "\x1b[1mLOOSE OBJECT\x1b[0m").unwrap();
        writeln!(&mut output, "{}", "─".repeat(40)).unwrap();
        writeln!(&mut output).unwrap();

        // Use the TUI formatter to generate rich content
        let mut widget = LooseObjectWidget::new(loose_obj.clone());
        let formatted_text = widget.text();

        // Convert ratatui Text to ANSI colored string
        let colored_text = CliPackFormatter::text_to_ansi_string(&formatted_text);
        writeln!(&mut output, "{colored_text}").unwrap();

        output
    }
}
