use crate::tui::main_view::{PackFocus, PackPreViewState, PreviewState, RegularPreViewState};
use crate::tui::message::{MainNavigation, Message};
use crate::tui::model::{AppState, AppView, GitObjectType};
use crossterm::event::{KeyCode, KeyEvent};

use super::RegularFocus;

/// Handle key events for the main view and convert them to appropriate messages
pub fn handle_key_event(key: KeyEvent, app: &AppState) -> Option<Message> {
    match &app.view {
        AppView::Main { state } => match key.code {
            KeyCode::Char('r') => Some(Message::Refresh),
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::Quit),
            KeyCode::Char('t') => Some(Message::MainNavigation(MainNavigation::ToggleExpand)),
            KeyCode::Char('h') | KeyCode::Left => {
                match &state.preview_state {
                    PreviewState::Regular(RegularPreViewState {
                        focus: RegularFocus::GitObjects,
                        ..
                    })
                    | PreviewState::Pack(PackPreViewState {
                        focus: PackFocus::GitObjects,
                        ..
                    }) => {
                        if !state.git_objects.flat_view.is_empty()
                            && state.git_objects.selected_index < state.git_objects.flat_view.len()
                        {
                            let (current_depth, selected_obj, _) =
                                &state.git_objects.flat_view[state.git_objects.selected_index];
                            match &selected_obj.obj_type {
                                GitObjectType::Category(_)
                                | GitObjectType::FileSystemFolder { .. }
                                | GitObjectType::PackFolder { .. } => {
                                    Some(Message::MainNavigation(MainNavigation::ToggleExpand))
                                }
                                _ => {
                                    if *current_depth > 0 {
                                        // Jump to parent category
                                        Some(Message::MainNavigation(
                                            MainNavigation::JumpToParentCategory,
                                        ))
                                    } else {
                                        None
                                    }
                                }
                            }
                        } else {
                            None
                        }
                    }
                    PreviewState::Pack(pack_state) => match pack_state.focus {
                        PackFocus::PackObjectsList | PackFocus::Educational => {
                            Some(Message::MainNavigation(MainNavigation::FocusGitObjects))
                        }
                        PackFocus::PackObjectDetails => Some(Message::MainNavigation(
                            MainNavigation::FocusEducationalOrList,
                        )),
                        PackFocus::GitObjects => None,
                    },

                    PreviewState::Regular(RegularPreViewState {
                        focus: RegularFocus::Preview,
                        ..
                    }) => Some(Message::MainNavigation(MainNavigation::FocusGitObjects)),
                }
            }
            KeyCode::Enter => match &state.preview_state {
                PreviewState::Regular(preview_state) => match preview_state.focus {
                    RegularFocus::GitObjects => {
                        // Check if the selected object is a loose object
                        if let Some((_, git_object, _)) = state
                            .git_objects
                            .flat_view
                            .get(state.git_objects.selected_index)
                        {
                            if matches!(git_object.obj_type, GitObjectType::LooseObject { .. }) {
                                Some(Message::OpenLooseObjectView)
                            } else {
                                Some(Message::MainNavigation(
                                    MainNavigation::FocusEducationalOrList,
                                ))
                            }
                        } else {
                            Some(Message::MainNavigation(
                                MainNavigation::FocusEducationalOrList,
                            ))
                        }
                    }
                    _ => None,
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::FocusEducationalOrList,
                    )),
                    PackFocus::PackObjectsList => {
                        if app.is_wide_screen() {
                            Some(Message::MainNavigation(
                                MainNavigation::FocusPackObjectDetails,
                            ))
                        } else {
                            Some(Message::OpenPackView)
                        }
                    }
                    PackFocus::Educational | PackFocus::PackObjectDetails => None,
                },
            },
            KeyCode::Char('l') | KeyCode::Right => match &state.preview_state {
                PreviewState::Regular(state) => match state.focus {
                    RegularFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::FocusEducationalOrList,
                    )),
                    _ => None,
                },
                PreviewState::Pack(state) => match state.focus {
                    PackFocus::GitObjects => Some(Message::MainNavigation(
                        MainNavigation::FocusEducationalOrList,
                    )),
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
                    PackFocus::PackObjectDetails => None,
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
            KeyCode::Tab => Some(Message::MainNavigation(MainNavigation::FocusToggle)),
            _ => None,
        },
        _ => None,
    }
}
