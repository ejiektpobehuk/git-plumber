use crate::tui::message::{LooseObjectNavigation, Message};
use crate::tui::model::{AppState, AppView};
use crossterm::event::{KeyCode, KeyEvent};

/// Handle key events for the loose object detail view
pub fn handle_key_event(key: KeyEvent, app: &AppState) -> Option<Message> {
    match &app.view {
        AppView::LooseObjectDetail { .. } => match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                Some(Message::OpenMainView)
            }
            KeyCode::Up | KeyCode::Char('k') => Some(Message::LooseObjectNavigation(
                LooseObjectNavigation::ScrollUp,
            )),
            KeyCode::Down | KeyCode::Char('j') => Some(Message::LooseObjectNavigation(
                LooseObjectNavigation::ScrollDown,
            )),
            KeyCode::PageUp | KeyCode::Char('g') => Some(Message::LooseObjectNavigation(
                LooseObjectNavigation::ScrollToTop,
            )),
            KeyCode::PageDown | KeyCode::Char('G') => Some(Message::LooseObjectNavigation(
                LooseObjectNavigation::ScrollToBottom,
            )),
            _ => None,
        },
        _ => None,
    }
}
