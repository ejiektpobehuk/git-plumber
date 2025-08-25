use crate::tui::main_view::MainViewState;
use crate::tui::main_view::state_components::TreeState;
use crate::tui::model::{AppState, AppView, GitObjectType};

use super::main_view::{PackPreViewState, PreviewState};

impl AppState {
    // Helper method to load pack objects if the selected object is a pack file
    pub fn load_pack_objects_if_needed(&mut self, _plumber: &crate::GitPlumber) {
        if let AppView::Main {
            state:
                MainViewState {
                    tree:
                        TreeState {
                            flat_view,
                            selected_index,
                            ..
                        },
                    preview_state,
                    ..
                },
        } = &mut self.view
        {
            if *selected_index >= flat_view.len() {
                return;
            }

            // Check if we're in Pack preview state
            let (pack_object_list, pack_file_path) = match preview_state {
                PreviewState::Pack(PackPreViewState {
                    pack_object_list,
                    pack_file_path,
                    ..
                }) => (pack_object_list, pack_file_path),
                _ => return, // Not in pack preview state
            };

            // Check if current object is a pack object
            let is_pack_object = match &flat_view[*selected_index].object.obj_type {
                GitObjectType::PackFolder { .. } => true,
                GitObjectType::PackFile { file_type, .. } => {
                    file_type == "packfile" || file_type == "pack"
                }
                _ => false,
            };

            if !is_pack_object {
                return;
            }

            // Extract the pack file path from the object type
            let path = match &flat_view[*selected_index].object.obj_type {
                GitObjectType::PackFolder { pack_group, .. } => {
                    // Get the pack file path from the pack group
                    if let Some(pack_path) = &pack_group.pack_file {
                        pack_path
                    } else {
                        return; // No pack file in this group
                    }
                }
                GitObjectType::PackFile { path, .. } => path,
                _ => return, // This shouldn't happen due to the match above, but just in case
            };

            // Load if we don't have the same pack loaded OR if the object list is empty
            if pack_file_path != path || pack_object_list.is_empty() {
                self.effects
                    .push(crate::tui::message::Command::LoadPackObjects { path: path.clone() });
            }
        }
    }
}
