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
                    if !state.git_objects.flat_view.is_empty() {
                        if state.git_objects.selected_index >= state.git_objects.flat_view.len() {
                            state.git_objects.selected_index = 0;
                        }
                        // Trigger details and educational content loads like before
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
