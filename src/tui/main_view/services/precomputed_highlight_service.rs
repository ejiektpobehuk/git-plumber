use crate::tui::main_view::HighlightInfo;
use crate::tui::main_view::animations::AnimationManager;
use crate::tui::model::{GitObject, GitObjectType};
use std::collections::HashMap;

/// Pre-computed highlight information for perfect alignment
#[derive(Debug, Clone)]
pub struct PrecomputedHighlights {
    /// Map from selection key to highlight info
    pub highlights: HashMap<String, HighlightInfo>,
}

impl PrecomputedHighlights {
    /// Create empty highlight map
    #[must_use]
    pub fn new() -> Self {
        Self {
            highlights: HashMap::new(),
        }
    }

    /// Get highlight info for a key (guaranteed alignment)
    #[must_use]
    pub fn get_highlight(&self, key: &str) -> HighlightInfo {
        self.highlights.get(key).cloned().unwrap_or_default()
    }

    /// Add a highlight for a specific key
    pub fn add_highlight(&mut self, key: String, highlight: HighlightInfo) {
        self.highlights.insert(key, highlight);
    }

    /// Check if there are any active highlights
    #[must_use]
    pub fn has_highlights(&self) -> bool {
        !self.highlights.is_empty()
    }
}

impl Default for PrecomputedHighlights {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for pre-computing all highlights before tree flattening
pub struct PrecomputedHighlightService;

impl PrecomputedHighlightService {
    /// Pre-compute all highlights for perfect alignment
    pub fn compute_all_highlights(
        tree: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> PrecomputedHighlights {
        let mut highlights = PrecomputedHighlights::new();

        // Step 1: Compute all direct file/object highlights
        Self::compute_direct_highlights(tree, animations, selection_key_fn, &mut highlights);

        // Step 2: Compute all folder highlights (only for closed folders)
        Self::compute_folder_highlights(tree, animations, selection_key_fn, &mut highlights);

        highlights
    }

    /// Compute direct highlights for files and objects
    fn compute_direct_highlights(
        tree: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
        highlights: &mut PrecomputedHighlights,
    ) {
        Self::walk_tree_for_direct_highlights(tree, animations, selection_key_fn, highlights);
    }

    /// Recursively walk tree to compute direct highlights
    fn walk_tree_for_direct_highlights(
        nodes: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
        highlights: &mut PrecomputedHighlights,
    ) {
        for node in nodes {
            let key = selection_key_fn(node);
            let highlight = animations.compute_highlight_info(&key);

            // Only store if there's an actual highlight
            if highlight.color.is_some() {
                highlights.add_highlight(key, highlight);
            }

            // Recursively process children
            Self::walk_tree_for_direct_highlights(
                &node.children,
                animations,
                selection_key_fn,
                highlights,
            );
        }
    }

    /// Compute folder highlights for closed folders
    fn compute_folder_highlights(
        tree: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
        highlights: &mut PrecomputedHighlights,
    ) {
        Self::walk_tree_for_folder_highlights(tree, animations, selection_key_fn, highlights);
    }

    /// Recursively walk tree to compute folder highlights
    fn walk_tree_for_folder_highlights(
        nodes: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
        highlights: &mut PrecomputedHighlights,
    ) {
        for node in nodes {
            match &node.obj_type {
                // Check all folder types
                GitObjectType::FileSystemFolder { .. }
                | GitObjectType::Category(_)
                | GitObjectType::PackFolder { .. } => {
                    if !node.expanded {
                        // This is a closed folder - check for dynamic highlights
                        let folder_key = selection_key_fn(node);
                        let files_inside =
                            Self::collect_all_files_in_folder(node, selection_key_fn);

                        if let Some(dynamic_highlight) =
                            animations.compute_folder_highlight(&folder_key, &files_inside)
                        {
                            let highlight_info = HighlightInfo {
                                color: Some(dynamic_highlight.color),
                                expires_at: Some(dynamic_highlight.expires_at),
                                animation_type:
                                    crate::tui::main_view::model::AnimationType::FolderBlink,
                            };
                            highlights.add_highlight(folder_key, highlight_info);
                        }
                    }

                    // Process children regardless of expansion state
                    Self::walk_tree_for_folder_highlights(
                        &node.children,
                        animations,
                        selection_key_fn,
                        highlights,
                    );
                }
                _ => {
                    // Process children of other node types
                    Self::walk_tree_for_folder_highlights(
                        &node.children,
                        animations,
                        selection_key_fn,
                        highlights,
                    );
                }
            }
        }
    }

    /// Collect all highlightable item keys inside a folder (recursively)
    /// Includes both files AND folders as highlightable items
    /// With full tree loading, all folders have their contents available immediately
    fn collect_all_files_in_folder(
        folder: &GitObject,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Vec<String> {
        let mut items = Vec::new();

        for child in &folder.children {
            match &child.obj_type {
                // Direct file types
                GitObjectType::FileSystemFile { .. } => {
                    items.push(selection_key_fn(child));
                }
                GitObjectType::Ref { .. } => {
                    items.push(selection_key_fn(child));
                }
                GitObjectType::LooseObject { .. } => {
                    items.push(selection_key_fn(child));
                }
                GitObjectType::PackFile { .. } => {
                    items.push(selection_key_fn(child));
                }

                // Folder types - include the folder itself AND recurse into it
                GitObjectType::FileSystemFolder { .. } => {
                    items.push(selection_key_fn(child)); // Include the folder as highlightable
                    items.extend(Self::collect_all_files_in_folder(child, selection_key_fn));
                }
                GitObjectType::Category(_) => {
                    items.push(selection_key_fn(child)); // Include the category as highlightable
                    items.extend(Self::collect_all_files_in_folder(child, selection_key_fn));
                }
                GitObjectType::PackFolder { .. } => {
                    items.push(selection_key_fn(child)); // Include the pack folder as highlightable
                    items.extend(Self::collect_all_files_in_folder(child, selection_key_fn));
                }
            }
        }

        items
    }

    /// Enhanced method that also checks for active highlights for animation detection
    pub fn has_active_highlights(
        tree: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> bool {
        // Check regular animations first
        if animations.has_active_animations() {
            return true;
        }

        // Check for any folder highlights
        Self::has_folder_highlights_recursive(tree, animations, selection_key_fn)
    }

    /// Recursively check for folder highlights
    fn has_folder_highlights_recursive(
        nodes: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> bool {
        for node in nodes {
            match &node.obj_type {
                // Check all folder types
                GitObjectType::FileSystemFolder { .. }
                | GitObjectType::Category(_)
                | GitObjectType::PackFolder { .. } => {
                    if !node.expanded {
                        let folder_key = selection_key_fn(node);
                        let files_inside =
                            Self::collect_all_files_in_folder(node, selection_key_fn);

                        if animations
                            .compute_folder_highlight(&folder_key, &files_inside)
                            .is_some()
                        {
                            return true;
                        }
                    }

                    if Self::has_folder_highlights_recursive(
                        &node.children,
                        animations,
                        selection_key_fn,
                    ) {
                        return true;
                    }
                }
                _ => {
                    if Self::has_folder_highlights_recursive(
                        &node.children,
                        animations,
                        selection_key_fn,
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::GitObject;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    #[test]
    fn test_precomputed_highlights() {
        // Create a test tree
        let mut root = GitObject::new_filesystem_folder(PathBuf::from("/test"), false);
        let mut src_folder = GitObject::new_filesystem_folder(PathBuf::from("/test/src"), false);
        let main_file = GitObject::new_filesystem_file(PathBuf::from("/test/src/main.rs"));

        src_folder.add_child(main_file);
        src_folder.expanded = false; // Folder is closed
        root.add_child(src_folder);
        root.expanded = true; // Root is open

        let tree = vec![root];

        // Create animation manager with a highlighted file
        let mut animations = AnimationManager::new();
        let now = Instant::now();
        animations.changed_keys.insert(
            "file:/test/src/main.rs".to_string(),
            now + Duration::from_secs(10),
        );

        let selection_key = |obj: &GitObject| -> String {
            match &obj.obj_type {
                GitObjectType::FileSystemFolder { path, .. } => {
                    format!("folder:{}", path.display())
                }
                GitObjectType::FileSystemFile { path, .. } => format!("file:{}", path.display()),
                _ => format!("other:{}", obj.name),
            }
        };

        // Pre-compute all highlights
        let highlights =
            PrecomputedHighlightService::compute_all_highlights(&tree, &animations, selection_key);

        // Test that we have highlights
        assert!(highlights.has_highlights());

        // Test that the file highlight is present
        let file_highlight = highlights.get_highlight("file:/test/src/main.rs");
        assert!(file_highlight.color.is_some());
        assert_eq!(file_highlight.color.unwrap(), ratatui::style::Color::Green);

        // Test that the closed folder highlight is present
        let folder_highlight = highlights.get_highlight("folder:/test/src");
        assert!(folder_highlight.color.is_some());
        assert_eq!(
            folder_highlight.color.unwrap(),
            ratatui::style::Color::Green
        );

        // Test that the open root folder has no highlight
        let root_highlight = highlights.get_highlight("folder:/test");
        assert!(root_highlight.color.is_none());
    }

    #[test]
    fn test_highlight_alignment() {
        let highlights = PrecomputedHighlights::new();

        // Test that non-existent keys return default
        let default_highlight = highlights.get_highlight("nonexistent");
        assert!(default_highlight.color.is_none());
        assert!(default_highlight.expires_at.is_none());
    }
}
