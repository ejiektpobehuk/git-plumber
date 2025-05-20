use crate::tui::main_view::{PackFocus, PreviewState};
use crate::tui::message::{MainNavigation, Message};
use crate::tui::model::{AppState, AppView};
use crossterm::event::{KeyCode, KeyEvent};

use super::RegularFocus;

/// Handle key events for the main view and convert them to appropriate messages
pub fn handle_key_event(key: KeyEvent, app: &AppState) -> Option<Message> {
    match &app.view {
        AppView::Main { state } => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::Quit),
            KeyCode::Char('h') | KeyCode::Left => {
                if state.are_git_objects_focused() {
                    None
                } else {
                    match &state.preview_state {
                        PreviewState::Regular(state) => match state.focus {
                            RegularFocus::Preview => {
                                Some(Message::MainNavigation(MainNavigation::FocusGitObjects))
                            }
                            _ => None,
                        },
                        PreviewState::Pack(state) => match state.focus {
                            PackFocus::PackObjectsList | PackFocus::Educational => {
                                Some(Message::MainNavigation(MainNavigation::FocusGitObjects))
                            }
                            PackFocus::PackObjectDetails => Some(Message::MainNavigation(
                                MainNavigation::FocusEducationalOrList,
                            )),
                            PackFocus::GitObjects => None,
                        },
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Right => match &state.preview_state {
                PreviewState::Regular(state) => match state.focus {
                    RegularFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::FocusEducationalOrList,
                    )),
                    _ => None,
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects | PackFocus::PackObjectDetails => Some(
                        Message::MainNavigation(MainNavigation::FocusEducationalOrList),
                    ),
                    PackFocus::Educational => {
                        if app.is_wide_screen() {
                            Some(Message::MainNavigation(
                                MainNavigation::FocusPackObjectDetails,
                            ))
                        } else {
                            None
                        }
                    }
                    PackFocus::PackObjectsList => {
                        if app.is_wide_screen() {
                            Some(Message::MainNavigation(
                                MainNavigation::FocusPackObjectDetails,
                            ))
                        } else {
                            Some(Message::OpenPackView)
                        }
                    }
                },
            },
            KeyCode::Up | KeyCode::Char('k') => match &state.preview_state {
                PreviewState::Regular(state) => match state.focus {
                    RegularFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::SelectPreviouwGitObject,
                    )),
                    RegularFocus::Preview => {
                        Some(Message::MainNavigation(MainNavigation::ScrollPreviewUp))
                    }
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::SelectPreviouwGitObject,
                    )),
                    PackFocus::Educational => {
                        Some(Message::MainNavigation(MainNavigation::ScrollEducationalUp))
                    }
                    PackFocus::PackObjectsList => Some(Message::MainNavigation(
                        MainNavigation::SelectPreviousPackObject,
                    )),
                    PackFocus::PackObjectDetails => {
                        Some(Message::MainNavigation(MainNavigation::ScrollPreviewUp))
                    }
                },
            },
            KeyCode::Down | KeyCode::Char('j') => match &state.preview_state {
                PreviewState::Regular(state) => match state.focus {
                    RegularFocus::GitObjects => {
                        Some(Message::MainNavigation(MainNavigation::SelectNextGitObject))
                    }
                    RegularFocus::Preview => {
                        Some(Message::MainNavigation(MainNavigation::ScrollPreviewDown))
                    }
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects => {
                        Some(Message::MainNavigation(MainNavigation::SelectNextGitObject))
                    }
                    PackFocus::Educational => Some(Message::MainNavigation(
                        MainNavigation::ScrollEducationalDown,
                    )),
                    PackFocus::PackObjectsList => Some(Message::MainNavigation(
                        MainNavigation::SelectNextPackObject,
                    )),
                    PackFocus::PackObjectDetails => {
                        Some(Message::MainNavigation(MainNavigation::ScrollPreviewDown))
                    }
                },
            },
            KeyCode::PageUp | KeyCode::Char('g') => match &state.preview_state {
                PreviewState::Regular(state) => match state.focus {
                    RegularFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::SelectFirstGitObject,
                    )),
                    RegularFocus::Preview => {
                        Some(Message::MainNavigation(MainNavigation::ScrollPreviewToTop))
                    }
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::SelectFirstGitObject,
                    )),
                    PackFocus::Educational => Some(Message::MainNavigation(
                        MainNavigation::ScrollEducationalToTop,
                    )),
                    PackFocus::PackObjectsList => Some(Message::MainNavigation(
                        MainNavigation::SelectFirstPackObject,
                    )),
                    PackFocus::PackObjectDetails => {
                        Some(Message::MainNavigation(MainNavigation::ScrollPreviewToTop))
                    }
                },
            },
            KeyCode::PageDown | KeyCode::Char('G') => match &state.preview_state {
                PreviewState::Regular(state) => match state.focus {
                    RegularFocus::GitObjects => {
                        Some(Message::MainNavigation(MainNavigation::SelectLastGitObject))
                    }
                    RegularFocus::Preview => Some(Message::MainNavigation(
                        MainNavigation::ScrollPreviewToBottom,
                    )),
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects => {
                        Some(Message::MainNavigation(MainNavigation::SelectLastGitObject))
                    }
                    PackFocus::Educational => Some(Message::MainNavigation(
                        MainNavigation::ScrollEducationalToBottom,
                    )),
                    PackFocus::PackObjectsList => Some(Message::MainNavigation(
                        MainNavigation::SelectLastPackObject,
                    )),
                    PackFocus::PackObjectDetails => Some(Message::MainNavigation(
                        MainNavigation::ScrollPreviewToBottom,
                    )),
                },
            },
            _ => None,
        },
        _ => None,
    }
}
