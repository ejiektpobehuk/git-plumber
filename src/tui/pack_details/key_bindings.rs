use crate::tui::message::{Message, PackNavigation};
use crate::tui::model::{AppState, AppView};
use crossterm::event::{KeyCode, KeyEvent};

/// Handle key events for the main view and convert them to appropriate messages
pub fn handle_key_event(key: KeyEvent, app: &AppState) -> Option<Message> {
    match &app.view {
        AppView::PackObjectDetail { .. } => match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                Some(Message::OpenMainView)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                Some(Message::PackNavigation(PackNavigation::ScrollUp))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                Some(Message::PackNavigation(PackNavigation::ScrollDown))
            }
            KeyCode::PageUp | KeyCode::Char('g') => {
                Some(Message::PackNavigation(PackNavigation::ScrollToTop))
            }
            KeyCode::PageDown | KeyCode::Char('G') => {
                Some(Message::PackNavigation(PackNavigation::ScrollToBottom))
            }
            _ => None,
        },
        _ => None,
    }
}
