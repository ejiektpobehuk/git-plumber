use crate::educational_content::EducationalContent;
use crate::tui::main_view::state_components::{ContentState, SessionState, TreeState};
use crate::tui::main_view::{ScrollSnapshot, SelectionIdentity};
use ratatui::text::Text;

/// Service responsible for state management, persistence, and synchronization
pub struct StateService;

impl StateService {
    /// Create a new `StateService` instance
    pub const fn new() -> Self {
        Self
    }

    /// Create initial state components
    pub fn create_initial_state(
        ed_provider: &EducationalContent,
    ) -> (TreeState, ContentState, SessionState) {
        (
            TreeState::new(),
            ContentState::new(ed_provider),
            SessionState::new(),
        )
    }

    /// Synchronize state between old and new structures
    pub fn sync_states(
        tree: &mut TreeState,
        content: &mut ContentState,
        session: &mut SessionState,
        // Legacy fields to sync from/to
        git_objects: &TreeState,
        git_object_info: &str,
        educational_content: &Text<'static>,
        last_selection: &Option<SelectionIdentity>,
        last_scroll_positions: &Option<ScrollSnapshot>,
        has_loaded_once: bool,
    ) {
        // Sync tree state
        tree.list = git_objects.list.clone();
        tree.flat_view = git_objects.flat_view.clone();
        tree.scroll_position = git_objects.scroll_position;
        tree.selected_index = git_objects.selected_index;

        // Sync content state
        content.git_object_info = git_object_info.to_string();
        content.educational_content = educational_content.clone();

        // Sync session state
        session.last_selection = last_selection.clone();
        session.last_scroll_positions = last_scroll_positions.clone();
        session.has_loaded_once = has_loaded_once;
    }

    /// Create backward compatibility state from new structure
    pub fn create_compatibility_state(
        tree: &TreeState,
        content: &ContentState,
        session: &SessionState,
    ) -> (
        TreeState,
        String,
        Text<'static>,
        Option<SelectionIdentity>,
        Option<ScrollSnapshot>,
        bool,
    ) {
        (
            tree.clone(),
            content.git_object_info.clone(),
            content.educational_content.clone(),
            session.last_selection.clone(),
            session.last_scroll_positions.clone(),
            session.has_loaded_once,
        )
    }

    /// Save current selection state
    pub fn save_selection_state(session: &mut SessionState, selection_key: String) {
        session.last_selection = Some(SelectionIdentity { key: selection_key });
    }

    /// Save current scroll positions
    pub const fn save_scroll_positions(
        session: &mut SessionState,
        git_list_scroll: usize,
        preview_scroll: usize,
        pack_list_scroll: usize,
    ) {
        session.last_scroll_positions = Some(ScrollSnapshot {
            git_list_scroll,
            preview_scroll,
            pack_list_scroll,
        });
    }

    /// Mark as loaded
    pub const fn mark_as_loaded(session: &mut SessionState) {
        session.has_loaded_once = true;
    }

    /// Update content information
    pub fn update_content_info(content: &mut ContentState, info: String) {
        content.git_object_info = info;
    }

    /// Update educational content
    pub fn update_educational_content(content: &mut ContentState, new_content: Text<'static>) {
        content.educational_content = new_content;
    }

    /// Restore selection from saved state
    pub fn restore_selection(session: &SessionState) -> Option<String> {
        session.last_selection.as_ref().map(|sel| sel.key.clone())
    }

    /// Restore scroll positions from saved state
    pub fn restore_scroll_positions(session: &SessionState) -> Option<(usize, usize, usize)> {
        session.last_scroll_positions.as_ref().map(|snap| {
            (
                snap.git_list_scroll,
                snap.preview_scroll,
                snap.pack_list_scroll,
            )
        })
    }

    /// Clear saved state (for reset operations)
    pub fn clear_session_state(session: &mut SessionState) {
        session.last_selection = None;
        session.last_scroll_positions = None;
        session.has_loaded_once = false;
    }

    /// Get state summary for debugging
    pub const fn get_state_summary(
        tree: &TreeState,
        content: &ContentState,
        session: &SessionState,
    ) -> StateSummary {
        StateSummary {
            tree_nodes: tree.list.len(),
            flat_nodes: tree.flat_view.len(),
            selected_index: tree.selected_index,
            scroll_position: tree.scroll_position,
            content_length: content.git_object_info.len(),
            has_selection: session.last_selection.is_some(),
            has_scroll_positions: session.last_scroll_positions.is_some(),
            has_loaded_once: session.has_loaded_once,
        }
    }
}

impl Default for StateService {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of current state for debugging and monitoring
#[derive(Debug, Clone)]
pub struct StateSummary {
    pub tree_nodes: usize,
    pub flat_nodes: usize,
    pub selected_index: usize,
    pub scroll_position: usize,
    pub content_length: usize,
    pub has_selection: bool,
    pub has_scroll_positions: bool,
    pub has_loaded_once: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::educational_content::EducationalContent;

    #[test]
    fn test_state_service_creation() {
        let service = StateService::new();
        let _ = service;
    }

    #[test]
    fn test_clear_session_state() {
        let mut session = SessionState::new();
        session.has_loaded_once = true;
        session.last_selection = Some(SelectionIdentity {
            key: "test".to_string(),
        });

        StateService::clear_session_state(&mut session);

        assert!(!session.has_loaded_once);
        assert!(session.last_selection.is_none());
        assert!(session.last_scroll_positions.is_none());
    }
}
