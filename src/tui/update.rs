use crate::tui::main_view::{MainViewState, PreviewState};
use crate::tui::message::Message;
use crate::tui::model::{AppState, AppView};
use crate::tui::widget::PackObjectWidget;

impl AppState {
    // Handle load result messages
    pub fn handle_load_result_message(
        &mut self,
        msg: Message,
        plumber: &crate::GitPlumber,
    ) -> bool {
        match msg {
            Message::LoadGitObjects(result) => match result {
                Ok(()) => {
                    // No-op here; prefer GitObjectsLoaded payload for success
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(e);
                }
            },

            Message::GitObjectsLoaded(data) => {
                // Apply the loaded git objects list to the MainView state
                if let AppView::Main { state } = &mut self.view {
                    state.git_objects.list = data.git_objects_list;
                    state.flatten_tree();

                    // Try to restore previous selection
                    if let Some(sel) = state.last_selection.take() {
                        if let Some(idx) = state
                            .git_objects
                            .flat_view
                            .iter()
                            .position(|(_, o)| MainViewState::selection_key(o) == sel.key)
                        {
                            state.git_objects.selected_index = idx;
                        } else if state.git_objects.selected_index
                            >= state.git_objects.flat_view.len()
                        {
                            state.git_objects.selected_index = 0;
                        }
                    } else if state.git_objects.selected_index >= state.git_objects.flat_view.len()
                    {
                        state.git_objects.selected_index = 0;
                    }

                    // Restore scrolls
                    if let Some(snap) = state.last_scroll_positions.take() {
                        state.git_objects.scroll_position = snap
                            .git_list_scroll
                            .min(state.git_objects.flat_view.len().saturating_sub(1));
                        match &mut state.preview_state {
                            PreviewState::Regular(r) => {
                                r.preview_scroll_position = snap.preview_scroll
                            }
                            PreviewState::Pack(p) => {
                                p.educational_scroll_position = snap.preview_scroll;
                                p.pack_object_list_scroll_position = snap
                                    .pack_list_scroll
                                    .min(p.pack_object_list.len().saturating_sub(1));
                            }
                        }
                    }

                    // If there are any items, trigger details and educational content loads
                    let should_load = !state.git_objects.flat_view.is_empty();
                    // Mark first successful load complete to enable highlighting on subsequent refreshes
                    state.has_loaded_once = true;
                    let _ = state;

                    if should_load {
                        let details_msg = self.load_git_object_details(plumber);
                        self.update(details_msg, plumber);
                        let content_msg = self.load_educational_content(plumber);
                        self.update(content_msg, plumber);
                    }
                    self.error = None;
                }
            }

            Message::LoadGitObjectInfo(result) => match result {
                Ok(info) => {
                    if let AppView::Main { state } = &mut self.view {
                        state.git_object_info = info;
                        self.error = None;
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                }
            },

            Message::LoadEducationalContent(result) => match result {
                Ok(preview) => {
                    if let AppView::Main { state } = &mut self.view {
                        state.educational_content = preview;
                        self.error = None;
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                }
            },

            Message::LoadPackObjects { path, result } => match result {
                Ok(objects) => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state: PreviewState::Pack(preview_state),
                                ..
                            },
                    } = &mut self.view
                    {
                        // Only apply if the message is for the currently previewed pack
                        if preview_state.pack_file_path == path {
                            preview_state.pack_object_list = objects;
                            // Reset selection to first object and update widget
                            preview_state.selected_pack_object = 0;
                            preview_state.pack_object_list_scroll_position = 0;
                            if !preview_state.pack_object_list.is_empty() {
                                preview_state.pack_object_widget_state = PackObjectWidget::new(
                                    preview_state.pack_object_list[0].clone(),
                                );
                            }
                            self.error = None;
                        }
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                }
            },

            _ => unreachable!("handle_load_result_message called with non-load-result message"),
        }
        true
    }

    // Update the application state based on a message
    pub fn update(&mut self, msg: Message, plumber: &crate::GitPlumber) -> bool {
        match msg {
            Message::Quit => return false,

            Message::Refresh => {
                // Capture selection + scroll before reload to preserve focus/context
                if let AppView::Main { state } = &mut self.view {
                    state.last_selection = state
                        .current_selection_key()
                        .map(|key| super::main_view::SelectionIdentity { key });
                    // capture scrolls
                    let preview_scroll = match &state.preview_state {
                        PreviewState::Regular(r) => r.preview_scroll_position,
                        PreviewState::Pack(p) => p.educational_scroll_position,
                    };
                    let pack_list_scroll = match &state.preview_state {
                        PreviewState::Pack(p) => p.pack_object_list_scroll_position,
                        _ => 0,
                    };
                    state.last_scroll_positions = Some(super::main_view::ScrollSnapshot {
                        git_list_scroll: state.git_objects.scroll_position,
                        preview_scroll,
                        pack_list_scroll,
                    });
                }
                // Reload everything from scratch
                self.effects.push(crate::tui::message::Command::LoadInitial);
            }

            Message::MainNavigation(_) | Message::OpenPackView | Message::OpenLooseObjectView => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            Message::PackNavigation(_) => {
                return self.handle_pack_view_mode_message(msg);
            }

            Message::LooseObjectNavigation(_) => {
                return self.handle_loose_object_view_mode_message(msg);
            }

            Message::OpenMainView => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            // Load result messages
            Message::LoadGitObjects(_)
            | Message::LoadGitObjectInfo(_)
            | Message::LoadEducationalContent(_)
            | Message::LoadPackObjects { .. }
            | Message::GitObjectsLoaded(_) => {
                return self.handle_load_result_message(msg, plumber);
            }
        }

        true // Continue running
    }
}
