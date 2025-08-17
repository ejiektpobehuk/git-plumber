use crate::git::loose_object::LooseObject;
use crate::tui::widget::ScrollableTextWidget;
use ratatui::text::Text;

pub mod formatters;

use formatters::{BlobFormatter, CommitFormatter, TagFormatter, TreeFormatter};

#[derive(Debug, Clone)]
pub struct LooseObjectWidget {
    loose_obj: LooseObject,
    scrollable_widget: ScrollableTextWidget,
}

impl LooseObjectWidget {
    pub fn new(loose_obj: LooseObject) -> Self {
        let mut scrollable_widget = ScrollableTextWidget::new();
        // Pre-generate and cache the content
        let content = LooseObjectFormatter::new(&loose_obj).generate_content();
        scrollable_widget.set_text(content);

        Self {
            loose_obj,
            scrollable_widget,
        }
    }

    pub fn text(&self) -> ratatui::text::Text<'static> {
        self.scrollable_widget.text()
    }

    /// Get the underlying loose object
    pub fn loose_object(&self) -> &LooseObject {
        &self.loose_obj
    }

    pub fn scroll_up(&mut self) {
        self.scrollable_widget.scroll_up();
    }

    pub fn scroll_down(&mut self) {
        self.scrollable_widget.scroll_down();
    }

    pub fn scroll_to_top(&mut self) {
        self.scrollable_widget.scroll_to_top();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scrollable_widget.scroll_to_bottom();
    }

    pub fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
    ) {
        self.scrollable_widget
            .render(f, area, "Loose Object Details", is_focused);
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
