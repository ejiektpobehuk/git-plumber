use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::tui::model::{AppState, AppView};

// The main view function - renders the UI based on the current state
pub fn draw_ui(f: &mut ratatui::Frame, app: &AppState) {
    let size = f.area();

    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(size);

    // Header
    let title = Block::default()
        .title("───")
        .title(Line::from(" git-plumber ").left_aligned())
        .borders(Borders::TOP)
        .border_type(ratatui::widgets::BorderType::Plain);
    f.render_widget(title, chunks[0]);

    let mut hints = match &app.view {
        AppView::Main { .. } => {
            crate::tui::main_view::render(f, app, chunks[1]);
            crate::tui::main_view::navigation_hints(app)
        }
        AppView::PackObjectDetail { .. } => {
            crate::tui::pack_details::render(f, app, chunks[1]);
            crate::tui::pack_details::navigation_hints(app)
        }
    };

    let mut decorated_hints = Vec::new();
    decorated_hints.push(Span::from("┤ "));
    decorated_hints.append(&mut hints);
    decorated_hints.push(Span::from(" ├──"));
    let footer = Block::default()
        .title(Line::from(decorated_hints).right_aligned())
        .borders(Borders::BOTTOM);
    f.render_widget(footer, chunks[2]);
}
