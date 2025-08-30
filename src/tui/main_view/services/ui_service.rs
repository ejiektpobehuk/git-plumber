use crate::tui::main_view::{PackFocus, PreviewState, RegularFocus, RegularPreViewState};
use crate::tui::model::GitObject;

/// Service responsible for UI state management and user interactions
pub struct UIService;

impl UIService {
    /// Create a new `UIService` instance
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Create initial preview state
    #[must_use]
    pub const fn create_initial_preview_state() -> PreviewState {
        PreviewState::Regular(RegularPreViewState::new())
    }

    /// Check if git objects are currently focused
    #[must_use]
    pub fn are_git_objects_focused(preview_state: &PreviewState) -> bool {
        match preview_state {
            PreviewState::Pack(state) => state.focus == PackFocus::GitObjects,
            PreviewState::Regular(state) => state.focus == RegularFocus::GitObjects,
        }
    }

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

    /// Find the best selection index after tree changes
    pub fn find_best_selection_index(
        flat_view: &[super::super::FlatTreeRow],
        target_key: Option<&str>,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> usize {
        if let Some(key) = target_key {
            // Try to find the exact key
            for (i, row) in flat_view.iter().enumerate() {
                if selection_key_fn(&row.object) == key {
                    return i;
                }
            }
        }

        // Default to first item (0 is safe even for empty lists)
        0
    }

    /// Navigate selection up
    #[must_use]
    pub const fn navigate_up(
        current_selection: usize,
        flat_view: &[super::super::FlatTreeRow],
    ) -> usize {
        if flat_view.is_empty() {
            return 0;
        }
        current_selection.saturating_sub(1)
    }

    /// Navigate selection down
    #[must_use]
    pub fn navigate_down(
        current_selection: usize,
        flat_view: &[super::super::FlatTreeRow],
    ) -> usize {
        if flat_view.is_empty() {
            return 0;
        }
        (current_selection + 1).min(flat_view.len().saturating_sub(1))
    }

    /// Navigate to first item
    #[must_use]
    pub const fn navigate_to_first() -> usize {
        0
    }

    /// Navigate to last item
    #[must_use]
    pub const fn navigate_to_last(flat_view: &[super::super::FlatTreeRow]) -> usize {
        flat_view.len().saturating_sub(1)
    }

    /// Page up navigation
    #[must_use]
    pub fn navigate_page_up(current_selection: usize, visible_height: usize) -> usize {
        current_selection.saturating_sub(visible_height.max(1))
    }

    /// Page down navigation
    #[must_use]
    pub fn navigate_page_down(
        current_selection: usize,
        flat_view: &[super::super::FlatTreeRow],
        visible_height: usize,
    ) -> usize {
        if flat_view.is_empty() {
            return 0;
        }
        (current_selection + visible_height.max(1)).min(flat_view.len().saturating_sub(1))
    }

    /// Get the currently selected object
    #[must_use]
    pub fn get_selected_object(
        flat_view: &[super::super::FlatTreeRow],
        selected_index: usize,
    ) -> Option<&GitObject> {
        flat_view.get(selected_index).map(|row| &row.object)
    }

    /// Check if an index is within bounds
    #[must_use]
    pub const fn is_valid_index(flat_view: &[super::super::FlatTreeRow], index: usize) -> bool {
        index < flat_view.len()
    }

    /// Get safe selection index (ensures it's within bounds)
    #[must_use]
    pub fn get_safe_selection_index(
        flat_view: &[super::super::FlatTreeRow],
        requested_index: usize,
    ) -> usize {
        if flat_view.is_empty() {
            0
        } else {
            requested_index.min(flat_view.len().saturating_sub(1))
        }
    }

    /// Calculate visible range for rendering
    #[must_use]
    pub fn calculate_visible_range(
        flat_view: &[super::super::FlatTreeRow],
        scroll_position: usize,
        visible_height: usize,
    ) -> (usize, usize) {
        if flat_view.is_empty() {
            return (0, 0);
        }

        let start = scroll_position.min(flat_view.len());
        let end = (start + visible_height).min(flat_view.len());
        (start, end)
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

impl Default for UIService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_service_creation() {
        let service = UIService::new();
        let _ = service;
    }

    #[test]
    fn test_navigate_up_at_beginning() {
        let flat_view = vec![];
        let result = UIService::navigate_up(0, &flat_view);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_navigate_down_empty_list() {
        let flat_view = vec![];
        let result = UIService::navigate_down(0, &flat_view);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_is_valid_index() {
        let flat_view = vec![];
        assert!(!UIService::is_valid_index(&flat_view, 0));
        assert!(!UIService::is_valid_index(&flat_view, 5));
    }

    #[test]
    fn test_calculate_visible_range_empty() {
        let flat_view = vec![];
        let (start, end) = UIService::calculate_visible_range(&flat_view, 0, 10);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }
}
