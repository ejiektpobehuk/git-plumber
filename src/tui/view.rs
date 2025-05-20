use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
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

    match &app.view {
        AppView::Main { state } => {
            crate::tui::main_view::render(f, app, chunks[1]);
        }
        AppView::PackObjectDetail { state } => {
            crate::tui::pack_details::render(f, app, chunks[1]);
        }
    };

    // Footer with navigation hints
    // let navigation_hints = match &app.view {
    //     AppView::Main {
    //         state:
    //             MainViewState::ObjectPreview {
    //                 selected_pack_object,
    //                 ..
    //             },
    //         ..
    //     } => {
    //         // Check if viewing pack objects
    //         let is_pack_object = selected_pack_object.is_some()
    //             && app
    //                 .selected_object()
    //                 .map(|obj| matches!(obj.obj_type, GitObjectType::Pack(_)))
    //                 .unwrap_or(false);
    //
    //         if is_pack_object {
    //             // Show pack navigation controls
    //             Block::default()
    //                 .title(
    //                     Line::from(vec![
    //                         Span::raw(" "),
    //                         Span::styled("Tab", Style::default().fg(Color::Blue)),
    //                         Span::raw(" to switch focus | "),
    //                         Span::styled("↕", Style::default().fg(Color::Blue)),
    //                         Span::raw(" to navigate | "),
    //                         Span::styled("Enter", Style::default().fg(Color::Blue)),
    //                         Span::raw(" to view detail | "),
    //                         Span::raw("("),
    //                         Span::styled("Q", Style::default().fg(Color::Blue)),
    //                         Span::raw(")uit ───"),
    //                     ])
    //                     .right_aligned(),
    //                 )
    //                 .borders(Borders::TOP)
    //         } else {
    //             // Show simple scroll controls
    //             Block::default()
    //                 .title(
    //                     Line::from(vec![
    //                         Span::raw(" "),
    //                         Span::styled("↕", Style::default().fg(Color::Blue)),
    //                         Span::raw(" to scroll | "),
    //                         Span::raw("("),
    //                         Span::styled("Q", Style::default().fg(Color::Blue)),
    //                         Span::raw(")uit ───"),
    //                     ])
    //                     .right_aligned(),
    //                 )
    //                 .borders(Borders::TOP)
    //         }
    //     }
    //     AppView::PackObjectDetail { .. } => Block::default()
    //         .title(
    //             Line::from(vec![
    //                 Span::raw(" "),
    //                 Span::styled("↕", Style::default().fg(Color::Blue)),
    //                 Span::raw(" to scroll | "),
    //                 Span::raw("("),
    //                 Span::styled("Q", Style::default().fg(Color::Blue)),
    //                 Span::raw(")uit ───"),
    //             ])
    //             .right_aligned(),
    //         )
    //         .borders(Borders::TOP),
    //     AppView::Main {
    //         state: MainViewState::ObjectList,
    //         ..
    //     } => Block::default()
    //         .title(
    //             Line::from(vec![
    //                 Span::raw(" "),
    //                 Span::styled("↕", Style::default().fg(Color::Blue)),
    //                 Span::raw(" to select | "),
    //                 Span::styled("Enter", Style::default().fg(Color::Blue)),
    //                 Span::raw(" to expand/preview | "),
    //                 Span::raw("("),
    //                 Span::styled("Q", Style::default().fg(Color::Blue)),
    //                 Span::raw(")uit ───"),
    //             ])
    //             .right_aligned(),
    //         )
    //         .borders(Borders::TOP),
    // };
    // f.render_widget(navigation_hints, chunks[2]);
}
