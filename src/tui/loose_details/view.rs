use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::tui::model::{AppState, AppView};

use super::LooseObjectViewState;

pub fn render(f: &mut ratatui::Frame, app: &mut AppState, area: ratatui::layout::Rect) {
    if let AppView::LooseObjectDetail {
        state:
            LooseObjectViewState {
                loose_widget: loose_widget_state,
                ..
            },
    } = &mut app.view
    {
        let is_focused = true;
        loose_widget_state.render(f, area, is_focused);
    }
}

pub fn navigation_hints(app: &AppState) -> Vec<Span> {
    match &app.view {
        AppView::LooseObjectDetail { .. } => {
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
