use crate::git::loose_object::LooseObject;
use crate::tui::helpers::render_styled_paragraph_with_scrollbar;
use ratatui::text::Text;

pub mod formatters;

use formatters::{BlobFormatter, CommitFormatter, TagFormatter, TreeFormatter};

#[derive(Debug, Clone)]
pub struct LooseObjectWidget {
    loose_obj: LooseObject,
    scroll_position: usize,
    max_scroll: usize,
    text_cache: Option<ratatui::text::Text<'static>>,
}

impl LooseObjectWidget {
    pub const fn new(loose_obj: LooseObject) -> Self {
        Self {
            loose_obj,
            scroll_position: 0,
            max_scroll: 0,
            text_cache: None,
        }
    }

    pub fn text(&mut self) -> ratatui::text::Text<'static> {
        if let Some(cached_content) = &self.text_cache {
            return cached_content.clone();
        }
        let content = LooseObjectFormatter::new(&self.loose_obj).generate_content();
        self.text_cache = Some(content.clone());
        content
    }

    pub const fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_position = (self.scroll_position + 1).min(self.max_scroll);
    }

    pub const fn scroll_to_top(&mut self) {
        self.scroll_position = 0;
    }

    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.max_scroll;
    }

    const fn scroll_position(&self) -> usize {
        self.scroll_position
    }

    pub fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
    ) {
        let content = self.text();

        let total_lines = content.lines.len();
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        self.max_scroll = total_lines.saturating_sub(visible_height);

        let title = "Loose Object Details";
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

struct LooseObjectFormatter<'a> {
    loose_obj: &'a LooseObject,
}

impl<'a> LooseObjectFormatter<'a> {
    const fn new(loose_obj: &'a LooseObject) -> Self {
        Self { loose_obj }
    }

    fn generate_content(&self) -> Text<'static> {
        let mut lines = Vec::new();

        // Add object header information
        Self::add_object_header(&mut lines, self.loose_obj);

        // Add type-specific formatted content
        match self.loose_obj.get_parsed_content() {
            Some(crate::git::loose_object::ParsedContent::Commit(commit)) => {
                let formatter = CommitFormatter::new(commit);
                formatter.format_commit(&mut lines);
            }
            Some(crate::git::loose_object::ParsedContent::Tree(tree)) => {
                let formatter = TreeFormatter::new(tree);
                formatter.format_tree(&mut lines);
            }
            Some(crate::git::loose_object::ParsedContent::Blob(content)) => {
                let formatter = BlobFormatter::new(content, self.loose_obj.is_binary());
                formatter.format_blob(&mut lines);
            }
            Some(crate::git::loose_object::ParsedContent::Tag(tag)) => {
                let formatter = TagFormatter::new(tag);
                formatter.format_tag(&mut lines);
            }
            None => {
                Self::add_unparsed_content(&mut lines, self.loose_obj);
            }
        }

        Text::from(lines)
    }

    fn add_object_header(lines: &mut Vec<ratatui::text::Line<'static>>, loose_obj: &LooseObject) {
        use ratatui::style::{Modifier, Style};
        use ratatui::text::Line;

        lines.push(Line::styled(
            "LOOSE OBJECT HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(40)));
        lines.push(Line::from(""));
        lines.push(Line::from(format!("Object ID: {}", loose_obj.object_id)));
        lines.push(Line::from(format!("Type: {}", loose_obj.object_type)));
        lines.push(Line::from(format!("Size: {} bytes", loose_obj.size)));
        lines.push(Line::from(""));
        lines.push(Line::from("Storage format: zlib-compressed"));
        lines.push(Line::from("Header format: <type> <size>\\0<content>"));
        lines.push(Line::from(""));
    }

    fn add_unparsed_content(
        lines: &mut Vec<ratatui::text::Line<'static>>,
        loose_obj: &LooseObject,
    ) {
        use ratatui::style::{Modifier, Style};
        use ratatui::text::Line;

        lines.push(Line::styled(
            "UNPARSED CONTENT",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));
        lines.push(Line::from("Failed to parse object content."));
        lines.push(Line::from("Raw content (first 200 bytes):"));
        lines.push(Line::from(""));

        let content_str = String::from_utf8_lossy(&loose_obj.content);
        let preview = if content_str.len() > 200 {
            format!("{}...", &content_str[..200])
        } else {
            content_str.to_string()
        };
        lines.push(Line::from(preview));
    }
}
