pub mod config;
pub mod formatters;

use crate::tui::helpers::render_styled_paragraph_with_scrollbar;
use crate::tui::model::PackObject;
use ratatui::text::{Text, ToText};

use formatters::{ContentFormatter, DeltaFormatter, HeaderFormatter};

#[derive(Debug, Clone)]
pub enum PackObjectWidget {
    Uninitiolized,
    Initiolized {
        pack_obj: PackObject,
        scroll_position: usize,
        max_scroll: usize,
        text_cache: Option<ratatui::text::Text<'static>>,
    },
}

impl PackObjectWidget {
    pub fn new(pack_obj: PackObject) -> Self {
        Self::Initiolized {
            pack_obj,
            scroll_position: 0,
            max_scroll: 0,
            text_cache: None,
        }
    }

    pub fn uninitiolized() -> Self {
        Self::Uninitiolized
    }

    pub fn text(&mut self) -> ratatui::text::Text<'static> {
        match self {
            &mut Self::Initiolized {
                ref pack_obj,
                ref mut text_cache,
                ..
            } => {
                if let Some(cached_content) = text_cache {
                    return cached_content.clone();
                }
                let content = PackObjectFormatter::new(pack_obj).generate_content();
                *text_cache = Some(content.clone());
                content
            }
            Self::Uninitiolized => "Initializing Pack Object Preview ...".to_text(),
        }
    }

    pub fn scroll_up(&mut self) {
        if let Self::Initiolized {
            scroll_position, ..
        } = self
        {
            *scroll_position = scroll_position.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        if let Self::Initiolized {
            scroll_position,
            max_scroll,
            ..
        } = self
        {
            *scroll_position = (*scroll_position + 1).min(*max_scroll);
        }
    }

    pub fn scroll_to_top(&mut self) {
        if let Self::Initiolized {
            scroll_position, ..
        } = self
        {
            *scroll_position = 0;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        if let Self::Initiolized {
            scroll_position,
            max_scroll,
            ..
        } = self
        {
            *scroll_position = *max_scroll;
        }
    }

    fn scroll_position(&self) -> usize {
        match self {
            Self::Initiolized {
                scroll_position, ..
            } => *scroll_position,
            Self::Uninitiolized => 0,
        }
    }

    pub fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
    ) {
        let content = self.text();

        if let Self::Initiolized { max_scroll, .. } = self {
            let total_lines = content.lines.len();
            let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
            *max_scroll = total_lines.saturating_sub(visible_height);
        }

        let title = "Pack Object Details";
        render_styled_paragraph_with_scrollbar(
            f,
            area,
            content,
            self.scroll_position(),
            title,
            is_focused,
        );
    }
}

struct PackObjectFormatter<'a> {
    pack_obj: &'a PackObject,
}

impl<'a> PackObjectFormatter<'a> {
    fn new(pack_obj: &'a PackObject) -> Self {
        Self { pack_obj }
    }

    fn generate_content(&self) -> Text<'static> {
        let mut lines = Vec::new();

        if let Some(ref object_data) = self.pack_obj.object_data {
            Self::add_header_section(&mut lines, object_data);
            Self::add_content_section(&mut lines, object_data);
        } else {
            self.add_basic_info_section(&mut lines);
        }

        Text::from(lines)
    }

    fn add_header_section(
        lines: &mut Vec<ratatui::text::Line<'static>>,
        object_data: &crate::git::pack::Object,
    ) {
        let header_formatter = HeaderFormatter::new(&object_data.header);
        header_formatter.format_header(lines);
    }

    fn add_content_section(
        lines: &mut Vec<ratatui::text::Line<'static>>,
        object_data: &crate::git::pack::Object,
    ) {
        let obj_type = object_data.header.obj_type();

        if obj_type == crate::git::pack::ObjectType::OfsDelta
            || obj_type == crate::git::pack::ObjectType::RefDelta
        {
            let delta_formatter = DeltaFormatter::new(&object_data.uncompressed_data);
            delta_formatter.format_delta_instructions(lines);
        } else {
            let content_formatter = ContentFormatter::new(object_data);
            content_formatter.format_object_content(lines);
        }
    }

    fn add_basic_info_section(&self, lines: &mut Vec<ratatui::text::Line<'static>>) {
        use ratatui::text::Line;

        lines.push(Line::from("BASIC OBJECT INFO"));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Object Type: {}",
            self.pack_obj.obj_type
        )));
        lines.push(Line::from(format!(
            "Uncompressed Size: {} bytes",
            self.pack_obj.size
        )));

        if let Some(ref base_info) = self.pack_obj.base_info {
            lines.push(Line::from(format!("Base Info: {base_info}")));
        }

        Self::add_basic_info_explanation(lines);
    }

    fn add_basic_info_explanation(lines: &mut Vec<ratatui::text::Line<'static>>) {
        use ratatui::text::Line;

        lines.push(Line::from(""));
        lines.push(Line::from(
            "This object is stored compressed within the pack file.",
        ));
        lines.push(Line::from(
            "To view the actual content, use git cat-file or similar tools.",
        ));
        lines.push(Line::from(""));
        lines.push(Line::from("Pack objects can be:"));
        lines.push(Line::from("• Blob: File contents"));
        lines.push(Line::from("• Tree: Directory structure"));
        lines.push(Line::from("• Commit: Commit information"));
        lines.push(Line::from("• Tag: Annotated tag"));
        lines.push(Line::from("• OFS Delta: Delta relative to offset"));
        lines.push(Line::from("• REF Delta: Delta relative to reference"));
    }
}
