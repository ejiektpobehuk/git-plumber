use crate::tui::helpers::render_styled_paragraph_with_scrollbar;
use ratatui::text::Text;

/// A unified widget for displaying scrollable text content.
/// This eliminates code duplication across `LooseObjectWidget`, `PackIndexWidget`, and `PackObjectWidget`.
#[derive(Debug, Clone)]
pub struct ScrollableTextWidget {
    text_cache: Option<Text<'static>>,
    scroll_position: usize,
    max_scroll: usize,
}

impl ScrollableTextWidget {
    /// Create a new scrollable text widget
    #[must_use]
    pub const fn new() -> Self {
        Self {
            text_cache: None,
            scroll_position: 0,
            max_scroll: 0,
        }
    }

    /// Set the text content for the widget
    pub fn set_text(&mut self, text: Text<'static>) {
        self.text_cache = Some(text);
        // Reset scroll position when content changes
        self.scroll_position = 0;
        self.max_scroll = 0;
    }

    /// Get the cached text content, or return a default if not set
    #[must_use]
    pub fn text(&self) -> Text<'static> {
        self.text_cache
            .clone()
            .unwrap_or_else(|| Text::from("Loading..."))
    }

    /// Scroll up by one line
    pub const fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        self.scroll_position = (self.scroll_position + 1).min(self.max_scroll);
    }

    /// Scroll to the top of the content
    pub const fn scroll_to_top(&mut self) {
        self.scroll_position = 0;
    }

    /// Scroll to the bottom of the content
    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.max_scroll;
    }

    /// Get the current scroll position
    #[must_use]
    pub const fn scroll_position(&self) -> usize {
        self.scroll_position
    }

    /// Render the widget with scrollbar
    pub fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        title: &str,
        is_focused: bool,
    ) {
        let content = self.text();

        // Update max_scroll based on current content and area
        let total_lines = content.lines.len();
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        self.max_scroll = total_lines.saturating_sub(visible_height);

        render_styled_paragraph_with_scrollbar(
            f,
            area,
            content,
            self.scroll_position,
            title,
            is_focused,
        );
    }

    /// Check if the widget has content
    #[must_use]
    pub const fn has_content(&self) -> bool {
        self.text_cache.is_some()
    }

    /// Clear the cached content
    pub fn clear(&mut self) {
        self.text_cache = None;
        self.scroll_position = 0;
        self.max_scroll = 0;
    }
}

impl Default for ScrollableTextWidget {
    fn default() -> Self {
        Self::new()
    }
}
