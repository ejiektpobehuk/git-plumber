use crate::tui::main_view::MainViewState;
use crate::tui::main_view::services::TreeService;
use crate::tui::model::{AppState, AppView, GitObjectType};

use super::main_view::{PackPreViewState, PreviewState};

impl AppState {
    // Helper method to load pack objects if the selected object is a pack file
    pub fn load_pack_objects_if_needed(&mut self, _plumber: &crate::GitPlumber) {
        if let AppView::Main { state } = &self.view {
            let Some(row) = state.tree.flat_view.get(state.tree.selected_index) else {
                return;
            };

            // Check if we're in Pack preview state
            let PreviewState::Pack(PackPreViewState {
                pack_object_list,
                pack_file_path,
                ..
            }) = &state.preview_state
            else {
                return; // Not in pack preview state
            };

            // Check if current object is a pack object
            if !row.kind.is_pack() {
                return;
            }

            // Resolve the row back to its tree node to extract the pack file path
            let node = TreeService::find_node_by_key(
                &state.tree.list,
                &row.key,
                MainViewState::selection_key,
            );
            let path = match node.map(|n| &n.obj_type) {
                Some(GitObjectType::PackFolder { pack_group, .. }) => {
                    // Get the pack file path from the pack group
                    if let Some(pack_path) = &pack_group.pack_file {
                        pack_path
                    } else {
                        return; // No pack file in this group
                    }
                }
                Some(GitObjectType::PackFile { path, .. }) => path,
                _ => return, // This shouldn't happen due to the check above, but just in case
            };

            // Load if we don't have the same pack loaded OR if the object list is empty
            if pack_file_path != path || pack_object_list.is_empty() {
                let path = path.clone();
                self.effects
                    .push(crate::tui::message::Command::LoadPackObjects { path });
            }
        }
    }
}
