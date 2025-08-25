use crate::educational_content::EducationalContent;
use crate::tui::model::GitObject;
use ratatui::text::Text;

/// Manages the git object tree data and its flattened representation
#[derive(Debug, Clone)]
pub struct TreeState {
    pub list: Vec<GitObject>,
    pub flat_view: Vec<super::FlatTreeRow>,
    pub scroll_position: usize,
    pub selected_index: usize,
}

impl TreeState {
    pub const fn new() -> Self {
        Self {
            list: Vec::new(),
            flat_view: Vec::new(),
            scroll_position: 0,
            selected_index: 0,
        }
    }
}

impl Default for TreeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages content display and information
#[derive(Debug, Clone)]
pub struct ContentState {
    pub git_object_info: String,
    pub educational_content: Text<'static>,
}

impl ContentState {
    pub fn new(ed_provider: &EducationalContent) -> Self {
        Self {
            git_object_info: String::new(),
            educational_content: ed_provider.get_default_content(),
        }
    }
}

/// Manages session persistence data
#[derive(Debug, Clone)]
pub struct SessionState {
    pub last_selection: Option<super::SelectionIdentity>,
    pub last_scroll_positions: Option<super::ScrollSnapshot>,
    pub has_loaded_once: bool,
}

impl SessionState {
    pub const fn new() -> Self {
        Self {
            last_selection: None,
            last_scroll_positions: None,
            has_loaded_once: false,
        }
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_state_creation() {
        let tree_state = TreeState::new();
        assert_eq!(tree_state.list.len(), 0);
        assert_eq!(tree_state.flat_view.len(), 0);
        assert_eq!(tree_state.scroll_position, 0);
        assert_eq!(tree_state.selected_index, 0);
    }

    #[test]
    fn test_session_state_creation() {
        let session_state = SessionState::new();
        assert!(session_state.last_selection.is_none());
        assert!(session_state.last_scroll_positions.is_none());
        assert!(!session_state.has_loaded_once);
    }
}
