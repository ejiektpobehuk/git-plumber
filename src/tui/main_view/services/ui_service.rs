/// Service responsible for UI state management and user interactions
pub struct UIService;

impl UIService {
    /// Update scroll position for git objects based on selection
    #[must_use]
    pub fn update_git_objects_scroll_for_selection(
        flat_view: &[super::super::FlatTreeRow],
        selected_index: usize,
        current_scroll: usize,
        visible_height: usize,
    ) -> usize {
        if flat_view.is_empty() || visible_height == 0 {
            return 0;
        }

        let safe_selected = selected_index.min(flat_view.len().saturating_sub(1));

        // Calculate maximum valid scroll position
        let max_scroll = flat_view.len().saturating_sub(visible_height);

        // If selection is above visible area, scroll up
        if safe_selected < current_scroll {
            return safe_selected.min(max_scroll);
        }

        // If selection is below visible area, scroll down
        let visible_end = current_scroll + visible_height;
        if safe_selected >= visible_end {
            let new_scroll = safe_selected.saturating_sub(visible_height.saturating_sub(1));
            return new_scroll.min(max_scroll);
        }

        // Selection is within visible area, but ensure scroll position is still valid
        current_scroll.min(max_scroll)
    }

    /// Ensure scroll position is valid for the current tree size
    #[must_use]
    pub fn clamp_scroll_position(
        flat_view: &[super::super::FlatTreeRow],
        scroll_position: usize,
        visible_height: usize,
    ) -> usize {
        if flat_view.is_empty() || visible_height == 0 {
            return 0;
        }

        let max_scroll = flat_view.len().saturating_sub(visible_height);
        scroll_position.min(max_scroll)
    }
}
