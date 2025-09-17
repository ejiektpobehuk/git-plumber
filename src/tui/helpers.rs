use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::scrollbar;
use ratatui::text::Text;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

// Helper function to render styled text with an integrated scroll bar
pub fn render_styled_paragraph_with_scrollbar(
    f: &mut ratatui::Frame,
    area: Rect,
    content: Text,
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
    let paragraph = Paragraph::new(displayed_content).block(if is_focused {
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
