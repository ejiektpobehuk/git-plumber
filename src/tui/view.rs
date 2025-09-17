use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::model::{AppState, AppView};

// The main view function - renders the UI based on the current state
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut AppState) {
    let size = f.area();

    // Create layout - adjust constraints based on view type
    let chunks = if matches!(app.view, AppView::TerminalTooSmall { .. }) {
        // For TerminalTooSmall view, use simpler layout without footer
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
            .split(size)
    } else {
        // Normal layout with header, content, and footer
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(size)
    };

    // Header
    let title = Block::default()
        .title("───")
        .title(Line::from(" git-plumber ").left_aligned())
        .borders(Borders::TOP)
        .border_type(ratatui::widgets::BorderType::Plain);
    f.render_widget(title, chunks[0]);

    // Determine content area index based on layout
    let content_area = if matches!(app.view, AppView::TerminalTooSmall { .. }) {
        chunks[1] // Simple layout: header + content
    } else {
        chunks[1] // Normal layout: header + content + footer
    };

    let mut hints = match &app.view {
        AppView::Main { .. } => {
            crate::tui::main_view::render(f, app, content_area);
            crate::tui::main_view::navigation_hints(app)
        }
        AppView::PackObjectDetail { .. } => {
            crate::tui::pack_details::render(f, app, content_area);
            crate::tui::pack_details::navigation_hints(app)
        }
        AppView::LooseObjectDetail { .. } => {
            crate::tui::loose_details::render(f, app, content_area);
            crate::tui::loose_details::navigation_hints(app)
        }
        AppView::TerminalTooSmall {
            width,
            height,
            min_width,
            min_height,
        } => {
            render_terminal_too_small(f, content_area, *width, *height, *min_width, *min_height);
            vec![Span::from("Resize terminal to continue")]
        }
    };

    // Only render footer if not in TerminalTooSmall view
    if !matches!(app.view, AppView::TerminalTooSmall { .. }) {
        let mut decorated_hints = Vec::new();
        decorated_hints.push(Span::from("┤ "));
        decorated_hints.append(&mut hints);
        decorated_hints.push(Span::from(" ├──"));
        let footer = Block::default()
            .title(Line::from(decorated_hints).right_aligned())
            .borders(Borders::BOTTOM);
        f.render_widget(footer, chunks[2]);
    }
}

// Render the "terminal too small" message
fn render_terminal_too_small(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    current_width: u16,
    current_height: u16,
    min_width: u16,
    min_height: u16,
) {
    let message = vec![
        Line::from(Span::styled(
            "Terminal Too Small",
            Style::default()
                .fg(Color::Red)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Current: {}x{}", current_width, current_height)),
        Line::from(format!("Need: {}x{}", min_width, min_height)),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'q' to quit",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(message)
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}
