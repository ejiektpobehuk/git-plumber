use crate::tui::model::{AppState, AppView};

impl AppState {
    // Helper method to update git objects scroll position for selection
    pub fn update_git_objects_scroll_for_selection(&mut self, new_index: usize) {
        let visible_height = self.layout_dimensions.git_objects_height;

        if let AppView::Main { state } = &mut self.view {
            state.update_git_objects_scroll_for_selection(visible_height, new_index)
        }
    }
}
