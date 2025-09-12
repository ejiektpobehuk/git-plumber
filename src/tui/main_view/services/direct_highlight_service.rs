use crate::tui::main_view::animations::AnimationManager;
use crate::tui::model::GitObject;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Service for directly applying highlights from file system watcher events
/// without needing full tree rebuilds
pub struct DirectHighlightService;

impl DirectHighlightService {
    /// Apply highlights directly from file system watcher events
    /// This bypasses the expensive tree rebuild and comparison process
    pub fn apply_watcher_changes(
        animations: &mut AnimationManager,
        added_files: &HashSet<PathBuf>,
        modified_files: &HashSet<PathBuf>,
        deleted_files: &HashSet<PathBuf>,
        animation_duration_secs: u64,
    ) {
        let now = Instant::now();
        let highlight_duration = Duration::from_secs(animation_duration_secs);

        // Convert file paths to selection keys and apply highlights
        for path in added_files {
            let key = Self::path_to_selection_key(path);
            animations
                .changed_keys
                .insert(key, now + highlight_duration);
        }

        for path in modified_files {
            let key = Self::path_to_selection_key(path);
            animations
                .modified_keys
                .insert(key, now + highlight_duration);
        }

        for path in deleted_files {
            let key = Self::path_to_selection_key(path);
            // Remove any existing highlights for deleted files
            animations.changed_keys.remove(&key);
            animations.modified_keys.remove(&key);
            // Note: We can't create proper ghosts here without tree context
            // Use apply_watcher_changes_with_tree or apply_filtered_watcher_changes instead
        }
    }

    /// Apply highlights with tree context for more intelligent handling
    pub fn apply_watcher_changes_with_tree(
        animations: &mut AnimationManager,
        tree: &[GitObject],
        added_files: &HashSet<PathBuf>,
        modified_files: &HashSet<PathBuf>,
        deleted_files: &HashSet<PathBuf>,
        selection_key_fn: fn(&GitObject) -> String,
        animation_duration_secs: u64,
    ) {
        let now = Instant::now();
        let highlight_duration = Duration::from_secs(animation_duration_secs);

        // Build a mapping from paths to selection keys using the actual tree
        let path_to_key_map = Self::build_path_to_key_map(tree, selection_key_fn);

        // Apply added file highlights
        for path in added_files {
            if let Some(key) = Self::find_key_for_path(&path_to_key_map, path) {
                // Remove any existing ghost for this key (file re-appeared)
                animations.ghosts.remove(&key);
                animations
                    .changed_keys
                    .insert(key, now + highlight_duration);
            }
        }

        // Apply modified file highlights
        for path in modified_files {
            if let Some(key) = Self::find_key_for_path(&path_to_key_map, path) {
                animations
                    .modified_keys
                    .insert(key, now + highlight_duration);
            }
        }

        // Handle deleted files - create ghosts and remove highlights
        for path in deleted_files {
            if let Some(key) = Self::find_key_for_path(&path_to_key_map, path) {
                // Remove from active highlights
                animations.changed_keys.remove(&key);
                animations.modified_keys.remove(&key);

                // Create a ghost for the deleted file
                Self::create_ghost_for_deleted_item(
                    animations,
                    tree,
                    &key,
                    selection_key_fn,
                    now,
                    highlight_duration,
                );
            }
        }

        // Clean up expired highlights (this normally happens during prune_timeouts)
        animations.changed_keys.retain(|_, until| *until > now);
        animations.modified_keys.retain(|_, until| *until > now);
        animations.ghosts.retain(|_, ghost| ghost.until > now);
    }

    /// Create a ghost for a deleted item by finding it in the current tree
    fn create_ghost_for_deleted_item(
        animations: &mut AnimationManager,
        tree: &[GitObject],
        deleted_key: &str,
        selection_key_fn: fn(&GitObject) -> String,
        now: Instant,
        ghost_duration: Duration,
    ) {
        // Don't create duplicate ghosts
        if animations.ghosts.contains_key(deleted_key) {
            return;
        }

        // Find the deleted item in the current tree (before it's removed)
        if let Some((deleted_node, parent_info)) =
            Self::find_node_with_parent_info(tree, deleted_key, selection_key_fn)
        {
            let (parent_key, sibling_index) = match parent_info {
                Some((parent_key, index)) => (Some(parent_key), index),
                None => (None, 0),
            };

            let ghost = crate::tui::main_view::Ghost {
                until: now + ghost_duration,
                parent_key,
                sibling_index,
                display: deleted_node,
            };
            animations.ghosts.insert(deleted_key.to_string(), ghost);
        }
    }

    /// Find a node in the tree along with its parent information
    fn find_node_with_parent_info(
        tree: &[GitObject],
        target_key: &str,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Option<(GitObject, Option<(String, usize)>)> {
        // Check top-level nodes first
        for node in tree {
            let node_key = selection_key_fn(node);
            if node_key == target_key {
                return Some((node.clone(), None)); // Top-level node has no parent
            }
        }

        // Recursively search in children
        for parent_node in tree {
            if let Some((found_node, child_index)) =
                Self::find_node_in_children(parent_node, target_key, selection_key_fn)
            {
                let parent_key = selection_key_fn(parent_node);
                return Some((found_node, Some((parent_key, child_index))));
            }
        }

        None
    }

    /// Recursively find a node within children of a parent node
    fn find_node_in_children(
        parent: &GitObject,
        target_key: &str,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Option<(GitObject, usize)> {
        // Check direct children
        for (index, child) in parent.children.iter().enumerate() {
            let child_key = selection_key_fn(child);
            if child_key == target_key {
                return Some((child.clone(), index));
            }
        }

        // Recursively search in grandchildren
        for child in &parent.children {
            if let Some((found_node, _)) =
                Self::find_node_in_children(child, target_key, selection_key_fn)
            {
                // For nested finds, we still return the direct child index, not the grandchild index
                // This is a simplification - for proper ghost positioning, we'd need more complex logic
                return Some((found_node, 0));
            }
        }

        None
    }

    /// Convert a file path to a selection key format
    fn path_to_selection_key(path: &Path) -> String {
        // Determine if this is a file or directory based on path characteristics
        // since the file may not exist on disk yet
        let path_str = path.to_string_lossy();
        let has_extension = path.extension().is_some();
        let looks_like_file = has_extension ||
            path_str.contains("/objects/") || // Git objects are files
            path_str.ends_with("/HEAD") ||
            path_str.ends_with("/index") ||
            path_str.contains("/refs/") && !path_str.ends_with("/refs") && !path_str.ends_with("/heads") && !path_str.ends_with("/tags");

        if looks_like_file {
            format!("file:{}", path.display())
        } else {
            format!("folder:{}", path.display())
        }
    }

    /// Build a mapping from file paths to selection keys using the current tree
    fn build_path_to_key_map(
        tree: &[GitObject],
        selection_key_fn: fn(&GitObject) -> String,
    ) -> std::collections::HashMap<PathBuf, String> {
        let mut map = std::collections::HashMap::new();

        fn walk_tree(
            node: &GitObject,
            map: &mut std::collections::HashMap<PathBuf, String>,
            selection_key_fn: fn(&GitObject) -> String,
        ) {
            // Extract path from the node if possible
            let path_opt = match &node.obj_type {
                crate::tui::model::GitObjectType::FileSystemFile { path, .. } => Some(path.clone()),
                crate::tui::model::GitObjectType::FileSystemFolder { path, .. } => {
                    Some(path.clone())
                }
                crate::tui::model::GitObjectType::PackFile { path, .. } => Some(path.clone()),
                crate::tui::model::GitObjectType::Ref { path, .. } => Some(path.clone()),
                _ => None,
            };

            if let Some(path) = path_opt {
                let key = selection_key_fn(node);
                map.insert(path, key);
            }

            // Recursively process children
            for child in &node.children {
                walk_tree(child, map, selection_key_fn);
            }
        }

        for obj in tree {
            walk_tree(obj, &mut map, selection_key_fn);
        }

        map
    }

    /// Find the selection key for a given path in the mapping
    fn find_key_for_path(
        path_to_key_map: &std::collections::HashMap<PathBuf, String>,
        target_path: &Path,
    ) -> Option<String> {
        // Direct lookup first
        if let Some(key) = path_to_key_map.get(target_path) {
            return Some(key.clone());
        }

        // Try to find by canonical path comparison
        if let Ok(canonical_target) = target_path.canonicalize() {
            for (path, key) in path_to_key_map {
                if let Ok(canonical_path) = path.canonicalize()
                    && canonical_path == canonical_target
                {
                    return Some(key.clone());
                }
            }
        }

        // Try string-based matching as fallback
        let target_str = target_path.to_string_lossy();
        for (path, key) in path_to_key_map {
            let path_str = path.to_string_lossy();
            if path_str == target_str {
                return Some(key.clone());
            }
        }

        None
    }

    /// Check if a path represents a git-related file that should be highlighted
    #[must_use]
    pub fn is_highlightable_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Include git objects, refs, packs, etc.
        path_str.contains("/objects/") ||
        path_str.contains("/refs/") ||
        path_str.contains("/packs/") ||
        path_str.ends_with("/HEAD") ||
        path_str.ends_with("/index") ||
        path_str.contains("/hooks/") ||
        // Also include any file in .git directory
        path_str.contains("/.git/")
    }

    /// Enhanced method that only processes relevant git files
    pub fn apply_filtered_watcher_changes(
        animations: &mut AnimationManager,
        tree: &[GitObject],
        added_files: &HashSet<PathBuf>,
        modified_files: &HashSet<PathBuf>,
        deleted_files: &HashSet<PathBuf>,
        selection_key_fn: fn(&GitObject) -> String,
        animation_duration_secs: u64,
    ) {
        // Filter to only relevant paths
        let relevant_added: HashSet<PathBuf> = added_files
            .iter()
            .filter(|path| Self::is_highlightable_path(path))
            .cloned()
            .collect();

        let relevant_modified: HashSet<PathBuf> = modified_files
            .iter()
            .filter(|path| Self::is_highlightable_path(path))
            .cloned()
            .collect();

        let relevant_deleted: HashSet<PathBuf> = deleted_files
            .iter()
            .filter(|path| Self::is_highlightable_path(path))
            .cloned()
            .collect();

        // Only proceed if we have relevant changes
        if relevant_added.is_empty() && relevant_modified.is_empty() && relevant_deleted.is_empty()
        {
            return;
        }

        // Apply the filtered changes
        Self::apply_watcher_changes_with_tree(
            animations,
            tree,
            &relevant_added,
            &relevant_modified,
            &relevant_deleted,
            selection_key_fn,
            animation_duration_secs,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::GitObject;
    use std::path::PathBuf;

    #[test]
    fn test_path_to_selection_key() {
        let file_path = PathBuf::from("/repo/.git/objects/ab/cd1234");
        let key = DirectHighlightService::path_to_selection_key(&file_path);
        assert_eq!(key, "file:/repo/.git/objects/ab/cd1234");

        let dir_path = PathBuf::from("/repo/.git/refs/heads");
        let key = DirectHighlightService::path_to_selection_key(&dir_path);
        assert_eq!(key, "folder:/repo/.git/refs/heads");
    }

    #[test]
    fn test_is_highlightable_path() {
        assert!(DirectHighlightService::is_highlightable_path(
            &PathBuf::from("/repo/.git/objects/ab/cd1234")
        ));
        assert!(DirectHighlightService::is_highlightable_path(
            &PathBuf::from("/repo/.git/refs/heads/main")
        ));
        assert!(DirectHighlightService::is_highlightable_path(
            &PathBuf::from("/repo/.git/HEAD")
        ));

        assert!(!DirectHighlightService::is_highlightable_path(
            &PathBuf::from("/repo/src/main.rs")
        ));
        assert!(!DirectHighlightService::is_highlightable_path(
            &PathBuf::from("/repo/README.md")
        ));
    }

    #[test]
    fn test_direct_highlighting() {
        let mut animations = AnimationManager::new();

        let mut added_files = HashSet::new();
        added_files.insert(PathBuf::from("/repo/.git/objects/ab/cd1234"));

        let mut modified_files = HashSet::new();
        modified_files.insert(PathBuf::from("/repo/.git/refs/heads/main"));

        let deleted_files = HashSet::new();

        DirectHighlightService::apply_watcher_changes(
            &mut animations,
            &added_files,
            &modified_files,
            &deleted_files,
            10, // Test with 10 seconds
        );

        // Check that highlights were applied
        assert!(
            animations
                .changed_keys
                .contains_key("file:/repo/.git/objects/ab/cd1234")
        );
        assert!(
            animations
                .modified_keys
                .contains_key("file:/repo/.git/refs/heads/main")
        );
    }
}
