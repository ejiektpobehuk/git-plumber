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
                    self.error = None;

                    if let AppView::Main { state } = &mut self.view {
                        // If we have objects, select the first one and load its details
                        if !state.git_objects.flat_view.is_empty() {
                            // Reset index if it's out of bounds
                            if state.git_objects.selected_index >= state.git_objects.flat_view.len()
                            {
                                state.git_objects.selected_index = 0;
                            }
                            // Load details and educational content for the selected object
                            let details_msg = self.load_git_object_details(plumber);
                            self.update(details_msg, plumber);
                            let content_msg = self.load_educational_content(plumber);
                            self.update(content_msg, plumber);
                        }
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                }
            },

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

            Message::LoadPackObjects(result) => match result {
                Ok(objects) => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state: PreviewState::Pack(preview_state),
                                ..
                            },
                    } = &mut self.view
                    {
                        preview_state.pack_object_list = objects;
                        // Reset selection to first object and update widget
                        preview_state.selected_pack_object = 0;
                        preview_state.pack_object_list_scroll_position = 0;
                        if !preview_state.pack_object_list.is_empty() {
                            preview_state.pack_object_widget_state =
                                PackObjectWidget::new(preview_state.pack_object_list[0].clone());
                        }
                        self.error = None;
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
                let objects_msg = self.load_git_objects(plumber);
                self.update(objects_msg, plumber);
            }

            // Navigation messages
            Message::SelectNext
            | Message::SelectPrevious
            | Message::SelectFirst
            | Message::SelectLast => {}

            // Main view mode messages - delegate to main_view module
            Message::TogglePackFocus => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            // Pack object detail mode messages
            Message::ExitPackObjectDetail
            | Message::HandlePackObjectDetailAction
            | Message::BackFromObjectDetail => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            Message::MainNavigation(_) | Message::OpenPackView => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            Message::PackNavigation(_) => {
                return self.handle_pack_view_mode_message(msg);
            }

            Message::OpenMainView => {
                return self.handle_main_view_mode_message(msg, plumber);
            }

            // Load result messages
            Message::LoadGitObjects(_)
            | Message::LoadGitObjectInfo(_)
            | Message::LoadEducationalContent(_)
            | Message::LoadPackObjects(_) => {
                return self.handle_load_result_message(msg, plumber);
            }
        }

        true // Continue running
    }
}
