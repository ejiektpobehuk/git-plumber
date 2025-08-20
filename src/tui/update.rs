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
                    // Snapshot for change detection if we already had a successful load
                    let (old_positions, old_nodes) = if state.has_loaded_once {
                        state.snapshot_old_positions()
                    } else {
                        (
                            std::collections::HashMap::new(),
                            std::collections::HashMap::new(),
                        )
                    };

                    // Clone old tree before clearing for state restoration
                    let old_tree = state.git_objects.list.clone();

                    // Set the new tree
                    state.git_objects.list = data.git_objects_list;

                    // Ensure natural sorting for categories except "objects"
                    MainViewState::sort_tree_for_display(&mut state.git_objects.list);

                    // Restore expansion and loading state from old tree if this isn't the first load
                    if state.has_loaded_once {
                        for new_obj in &mut state.git_objects.list {
                            new_obj.restore_state_from(&old_tree);
                        }

                        // NOW detect changes after full restoration
                        let _ = state.detect_tree_changes(&old_positions, &old_nodes);
                    }

                    state.flatten_tree();

                    // Try to restore previous selection
                    if let Some(sel) = state.last_selection.take() {
                        if let Some(idx) = state
                            .git_objects
                            .flat_view
                            .iter()
                            .position(|(_, o, _)| MainViewState::selection_key(o) == sel.key)
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
                                if r.pack_index_widget.is_none() {
                                    r.preview_scroll_position = snap.preview_scroll;
                                }
                                // PackIndex widget manages its own scrolling, no restoration needed
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
                        // Only reset to regular state if we're not in a pack preview state
                        match &mut state.preview_state {
                            PreviewState::Regular(regular_state) => {
                                // Clear pack index widget for regular state
                                regular_state.pack_index_widget = None;
                            }
                            PreviewState::Pack(_) => {
                                // Preserve pack preview state - don't reset it!
                            }
                        }
                        self.error = None;
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                }
            },

            Message::LoadPackIndexDetails(result) => match *result {
                Ok(pack_index) => {
                    if let AppView::Main { state } = &mut self.view {
                        // Switch to Regular preview state with pack index widget
                        state.preview_state = PreviewState::Regular(
                            crate::tui::main_view::RegularPreViewState::new_with_pack_index(
                                pack_index,
                            ),
                        );
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
                        PreviewState::Regular(r) => {
                            if r.pack_index_widget.is_some() {
                                0 // PackIndex widget manages its own scrolling
                            } else {
                                r.preview_scroll_position
                            }
                        }
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

                    // Ghosts are pruned during flatten_tree; no explicit cleanup needed here.
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
            | Message::LoadPackIndexDetails(_)
            | Message::LoadPackObjects { .. }
            | Message::GitObjectsLoaded(_) => {
                return self.handle_load_result_message(msg, plumber);
            }

            Message::TimerTick => {
                // Handle animation timer tick
                if let AppView::Main { state } = &mut self.view
                    && state.prune_timeouts()
                {
                    state.flatten_tree();
                }
            }

            Message::TerminalResize(width, height) => {
                // Handle terminal resize event
                let new_size = ratatui::layout::Size { width, height };
                self.check_terminal_resize(new_size);
            }

            Message::KeyEvent(key) => {
                // Handle keyboard event
                if let Some(msg) = match self.view {
                    AppView::Main { .. } => crate::tui::main_view::handle_key_event(key, self),
                    AppView::PackObjectDetail { .. } => {
                        crate::tui::pack_details::handle_key_event(key, self)
                    }
                    AppView::LooseObjectDetail { .. } => {
                        crate::tui::loose_details::handle_key_event(key, self)
                    }
                } {
                    return self.update(msg, plumber);
                }
            }
        }

        true // Continue running
    }
}
