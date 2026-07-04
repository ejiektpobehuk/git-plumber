use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::scrollbar;
use ratatui::text::Text;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

/// Truncate a string to at most `max_bytes` bytes without splitting a UTF-8
/// character. Direct byte slicing (`&s[..n]`) panics when `n` falls inside a
/// multibyte character — including the 3-byte U+FFFD replacement characters
/// that `from_utf8_lossy` inserts for binary content.
pub fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// Helper function to render styled text with an integrated scroll bar
pub fn render_styled_paragraph_with_scrollbar(
    f: &mut ratatui::Frame,
    area: Rect,
    content: &Text,
    scroll_position: usize,
    title: &str,
    is_focused: bool,
) {
    let total_lines = content.lines.len();
    let visible_height = (area.height as usize).saturating_sub(2); // Account for borders

    // Prepare the displayed content with scrolling
    let max_start = total_lines.saturating_sub(visible_height);
    let start = scroll_position.min(max_start);
    let end = start + visible_height.min(total_lines.saturating_sub(start));

    // Create new Text with only the visible lines
    let displayed_content = Text::from(content.lines[start..end].to_vec());

    // Create the paragraph widget
    let paragraph = Paragraph::new(displayed_content)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .block(if is_focused {
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Plain)
                .border_style(Style::default().fg(Color::Yellow))
        } else {
            Block::default().title(title).borders(Borders::ALL)
        });

    f.render_widget(paragraph, area);

    // Render the built-in scroll bar if content is scrollable
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None)
            .style(if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let max_scroll = total_lines.saturating_sub(visible_height);
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll)
            .viewport_content_length(visible_height)
            .position(scroll_position);

        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

// Helper function to render a list with an integrated scroll bar
pub fn render_list_with_scrollbar<T>(
    f: &mut ratatui::Frame,
    area: Rect,
    items: &[T],
    selected_index: Option<usize>,
    scroll_position: usize,
    title: &str,
    is_focused: bool,
    mut item_renderer: impl FnMut(usize, &T, bool) -> ListItem,
) {
    let visible_height = (area.height as usize).saturating_sub(2); // Account for borders
    let total_items = items.len();

    // Prepare the displayed items with scrolling
    let max_start = total_items.saturating_sub(visible_height);
    let start = scroll_position.min(max_start);
    let end = start + visible_height.min(total_items.saturating_sub(start));

    // Create list items for the visible range
    let list_items: Vec<ListItem> = items[start..end]
        .iter()
        .enumerate()
        .map(|(relative_index, item)| {
            let absolute_index = start + relative_index;
            let is_selected = selected_index == Some(absolute_index);
            (absolute_index, item, is_selected)
        })
        .map(|(absolute_index, item, is_selected)| item_renderer(absolute_index, item, is_selected))
        .collect();

    // Create the list widget
    let list = List::new(list_items).block(if is_focused {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow))
    } else {
        Block::default().title(title).borders(Borders::ALL)
    });

    f.render_widget(list, area);

    // Render the built-in scroll bar if content is scrollable
    if total_items > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None)
            .style(if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let max_scroll = total_items.saturating_sub(visible_height);
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll)
            .viewport_content_length(visible_height)
            .position(scroll_position);

        f.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

// Helper function to render a list with scrollbar and out-of-view change indicators
pub fn render_list_with_scrollbar_indicators<T>(
    f: &mut ratatui::Frame,
    area: Rect,
    items: &[T],
    selected_index: Option<usize>,
    scroll_position: usize,
    title: &str,
    is_focused: bool,
    indicators: &[crate::tui::main_view::animations::ScrollbarIndicator],
    mut item_renderer: impl FnMut(usize, &T, bool) -> ListItem,
) {
    let visible_height = (area.height as usize).saturating_sub(2); // Account for borders
    let total_items = items.len();

    // Prepare the displayed items with scrolling
    let max_start = total_items.saturating_sub(visible_height);
    let start = scroll_position.min(max_start);
    let end = start + visible_height.min(total_items.saturating_sub(start));

    // Create list items for the visible range
    let list_items: Vec<ListItem> = items[start..end]
        .iter()
        .enumerate()
        .map(|(relative_index, item)| {
            let absolute_index = start + relative_index;
            let is_selected = selected_index == Some(absolute_index);
            (absolute_index, item, is_selected)
        })
        .map(|(absolute_index, item, is_selected)| item_renderer(absolute_index, item, is_selected))
        .collect();

    // Create the list widget
    let list = List::new(list_items).block(if is_focused {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow))
    } else {
        Block::default().title(title).borders(Borders::ALL)
    });

    f.render_widget(list, area);

    // Render the scrollbar with indicators if content is scrollable
    if total_items > visible_height {
        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        // Render the standard scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None)
            .style(if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        let max_scroll = total_items.saturating_sub(visible_height);
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll)
            .viewport_content_length(visible_height)
            .position(scroll_position);

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

        // Render change indicators on the scrollbar
        render_scrollbar_indicators(f.buffer_mut(), scrollbar_area, indicators);
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_at_char_boundary;

    #[test]
    fn ascii_within_limit_is_unchanged() {
        assert_eq!(truncate_at_char_boundary("hello", 10), "hello");
    }

    #[test]
    fn ascii_truncates_exactly_at_limit() {
        assert_eq!(truncate_at_char_boundary("hello world", 5), "hello");
    }

    #[test]
    fn does_not_split_multibyte_char() {
        // 'é' is 2 bytes; limit 3 falls mid-character
        assert_eq!(truncate_at_char_boundary("ééé", 3), "é");
    }

    #[test]
    fn handles_replacement_chars_from_lossy_binary() {
        // Binary data becomes a run of 3-byte U+FFFD chars; a limit of 200
        // falls inside one (198..201) — the original panic case.
        let binary = vec![0xFFu8; 300];
        let s = String::from_utf8_lossy(&binary);
        let truncated = truncate_at_char_boundary(&s, 200);
        assert_eq!(truncated.len(), 198);
        assert!(truncated.chars().all(|c| c == char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn zero_limit_returns_empty() {
        assert_eq!(truncate_at_char_boundary("é", 0), "");
    }
}

/// Render change indicators on the scrollbar track
fn render_scrollbar_indicators(
    buf: &mut Buffer,
    scrollbar_area: Rect,
    indicators: &[crate::tui::main_view::animations::ScrollbarIndicator],
) {
    if scrollbar_area.height < 2 {
        return; // Not enough space for indicators
    }

    // The scrollbar is on the right edge of the area
    let scrollbar_x = scrollbar_area.right().saturating_sub(1);
    let scrollbar_start_y = scrollbar_area.top();
    let scrollbar_height = f32::from(scrollbar_area.height);

    for indicator in indicators {
        // Calculate the Y position for this indicator
        // relative_position is in [0, 1], so the product fits comfortably in u16
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let indicator_y =
            scrollbar_start_y + (indicator.relative_position * scrollbar_height) as u16;

        // Make sure we're within bounds
        if indicator_y >= scrollbar_area.bottom() {
            continue;
        }

        // Render a colored character at this position
        // Use a medium shade block character to make it distinct from the scrollbar
        if scrollbar_x < buf.area.width && indicator_y < buf.area.height + buf.area.y {
            buf[(scrollbar_x, indicator_y)]
                .set_char('▒')
                .set_fg(indicator.color)
                .set_bg(Color::Reset);
        }
    }
}
