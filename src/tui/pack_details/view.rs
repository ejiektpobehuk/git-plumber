use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::fmt::Write;

use crate::tui::model::PackObject;
use crate::tui::model::{AppState, AppView};
use crate::tui::widget::PackObjectWidget;

use super::PackViewState;

pub fn render(f: &mut ratatui::Frame, app: &AppState, area: ratatui::layout::Rect) {
    if let AppView::PackObjectDetail {
        state:
            PackViewState {
                pack_widget: pack_widget_state,
                ..
            },
    } = &app.view
    {
        let is_focused = true;
        pack_widget_state.render(f, area, is_focused);
    }
}

pub fn navigation_hints(app: &AppState) -> Vec<Span> {
    match &app.view {
        AppView::PackObjectDetail { .. } => {
            vec![
                Span::styled("↕", Style::default().fg(Color::Blue)),
                Span::raw(" to scroll | "),
                Span::raw(""),
                Span::styled("Q", Style::default().fg(Color::Blue)),
                Span::styled("/", Style::default().fg(Color::Gray)),
                Span::styled("←", Style::default().fg(Color::Blue)),
                Span::raw(" - go back"),
            ]
        }
        _ => Vec::new(),
    }
}
