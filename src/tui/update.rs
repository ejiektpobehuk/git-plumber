use crate::tui::main_view::{ChangeDetectionService, MainViewState, PreviewState};
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
                    let old_snapshot = if state.session.has_loaded_once {
                        Some(ChangeDetectionService::snapshot_tree_positions(
                            &state.tree.list,
                            MainViewState::selection_key,
                        ))
                    } else {
                        None
                    };

                    // Clone old tree before clearing for state restoration
                    let old_tree = state.tree.list.clone();

                    // Set the new tree
                    state.tree.list = data.git_objects_list;

                    // Ensure natural sorting for categories except "objects"
                    crate::tui::main_view::NaturalSorter::sort_tree_for_display(
                        &mut state.tree.list,
                    );

                    // Sync to new structure

                    // Restore expansion and loading state from old tree if this isn't the first load
                    if state.session.has_loaded_once {
                        for new_obj in &mut state.tree.list {
                            new_obj.restore_state_from(&old_tree);
                        }
                    }

                    // Populate empty state caches to ensure consistent comparison
                    for obj in &mut state.tree.list {
                        obj.populate_empty_caches_recursive();
                        // Refresh caches for collapsed folders to detect content changes
                        obj.refresh_empty_caches_for_collapsed();
                    }

                    // Sync to new structure after all modifications

                    // NOW detect changes after full restoration and cache population
                    if let Some(old_snapshot) = old_snapshot {
                        let _ = state.detect_tree_changes(
                            &old_snapshot.positions,
                            &old_snapshot.nodes,
                            self.animation_duration_secs,
                        );
                    }

                    state.flatten_tree();

                    // Try to restore previous selection
                    if let Some(sel) = state.session.last_selection.take() {
                        if let Some(idx) =
                            state.tree.flat_view.iter().position(|row| {
                                MainViewState::selection_key(&row.object) == sel.key
                            })
                        {
                            state.tree.selected_index = idx;
                        } else if state.tree.selected_index >= state.tree.flat_view.len() {
                            state.tree.selected_index = 0;
                        }
                    } else if state.tree.selected_index >= state.tree.flat_view.len() {
                        state.tree.selected_index = 0;
                    }

                    // Restore scrolls
                    if let Some(snap) = state.session.last_scroll_positions.take() {
                        // Use actual layout dimensions for proper scroll clamping
                        let visible_height = self.layout_dimensions.git_objects_height;
                        state.tree.scroll_position =
                            super::main_view::services::UIService::clamp_scroll_position(
                                &state.tree.flat_view,
                                snap.git_list_scroll,
                                visible_height,
                            );
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
                    let should_load = !state.tree.flat_view.is_empty();
                    // Mark first successful load complete to enable highlighting on subsequent refreshes
                    state.session.has_loaded_once = true;
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
                        state.content.git_object_info = info;
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
                        state.content.educational_content = preview;
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
                    state.session.last_selection = state
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
                        PreviewState::Regular(_) => 0,
                    };
                    state.session.last_scroll_positions = Some(super::main_view::ScrollSnapshot {
                        git_list_scroll: state.tree.scroll_position,
                        preview_scroll,
                        pack_list_scroll,
                    });

                    // Ghosts are pruned during flatten_tree; no explicit cleanup needed here.
                }
                // Reload everything from scratch
                self.effects.push(crate::tui::message::Command::LoadInitial);
            }

            Message::DirectFileChanges {
                added_files,
                modified_files,
                deleted_files,
            } => {
                // Apply highlights directly without full tree rebuild
                if let AppView::Main { state } = &mut self.view {
                    super::main_view::services::DirectHighlightService::apply_filtered_watcher_changes(
                        &mut state.animations,
                        &state.tree.list,
                        &added_files,
                        &modified_files,
                        &deleted_files,
                        super::main_view::MainViewState::selection_key,
                        self.animation_duration_secs,
                    );

                    // Re-flatten tree to apply new highlights (but no tree rebuild needed!)
                    state.flatten_tree();
                }
            }

            Message::MainNavigation(_)
            | Message::OpenMainView
            | Message::OpenPackView
            | Message::OpenLooseObjectView => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            Message::PackNavigation(_) => {
                return self.handle_pack_view_mode_message(msg);
            }

            Message::LooseObjectNavigation(_) => {
                return self.handle_loose_object_view_mode_message(msg);
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
