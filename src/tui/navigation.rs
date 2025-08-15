use crate::tui::main_view::MainViewState;
use crate::tui::model::{AppState, AppView, GitObjectType};

use super::main_view::{GitObjectsState, PackPreViewState, PreviewState};

impl AppState {
    // Helper method to load pack objects if the selected object is a pack file
    pub fn load_pack_objects_if_needed(&mut self, _plumber: &crate::GitPlumber) {
        if let AppView::Main {
            state:
                MainViewState {
                    git_objects:
                        GitObjectsState {
                            flat_view,
                            selected_index,
                            ..
                        },
                    preview_state:
                        PreviewState::Pack(PackPreViewState {
                            pack_file_path,
                            pack_object_list,
                            ..
                        }),
                    ..
                },
        } = &mut self.view
            && *selected_index < flat_view.len()
            && match &flat_view[*selected_index].1.obj_type {
                GitObjectType::PackFolder { .. } => true,
                GitObjectType::PackFile { file_type, .. } => file_type == "packfile",
                _ => false,
            }
        {
            // Extract the pack file path from the object type
            let path = match &flat_view[*selected_index].1.obj_type {
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
