use crate::tui::main_view::animations::AnimationManager;
use crate::tui::main_view::{FlatTreeRow, TreeFlattener};
use crate::tui::model::GitObject;

/// Service responsible for all tree operations including flattening, traversal, and manipulation
pub struct TreeService;

impl TreeService {
    /// Create a new `TreeService` instance
    pub const fn new() -> Self {
        Self
    }

    /// Flatten a tree with animation support
    pub fn flatten_tree_with_animations(
        tree: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Vec<FlatTreeRow> {
        let highlight_fn = |key: &str| animations.compute_highlight_info(key);
        TreeFlattener::flatten_tree_with_ghosts(
            tree,
            highlight_fn,
            selection_key_fn,
            &animations.ghosts,
        )
    }

    /// Find a node in the tree by its selection key
    pub fn find_node_by_key<'a>(
        tree: &'a [GitObject],
        key: &str,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Option<&'a GitObject> {
        fn search_in_children<'a>(
            children: &'a [GitObject],
            key: &str,
            selection_key_fn: fn(&GitObject) -> String,
        ) -> Option<&'a GitObject> {
            for child in children {
                if selection_key_fn(child) == key {
                    return Some(child);
                }
                if let Some(found) = search_in_children(&child.children, key, selection_key_fn) {
                    return Some(found);
                }
            }
            None
        }
        search_in_children(tree, key, selection_key_fn)
    }

    /// Get all keys from a tree
    pub fn collect_all_keys(
        tree: &[GitObject],
        selection_key_fn: fn(&GitObject) -> String,
    ) -> std::collections::HashSet<String> {
        let mut keys = std::collections::HashSet::new();

        fn collect_recursive(
            node: &GitObject,
            keys: &mut std::collections::HashSet<String>,
            selection_key_fn: fn(&GitObject) -> String,
        ) {
            keys.insert(selection_key_fn(node));
            for child in &node.children {
                collect_recursive(child, keys, selection_key_fn);
            }
        }

        for obj in tree {
            collect_recursive(obj, &mut keys, selection_key_fn);
        }

        keys
    }

    /// Apply natural sorting to a tree
    pub fn sort_tree_for_display(tree: &mut [GitObject]) {
        super::super::NaturalSorter::sort_tree_for_display(tree);
    }

    /// Restore state from an old tree to a new tree
    pub fn restore_tree_state(new_tree: &mut [GitObject], old_tree: &[GitObject]) {
        for new_obj in new_tree {
            new_obj.restore_state_from(old_tree);
        }
    }

    /// Populate empty caches for all nodes in the tree
    pub fn populate_tree_caches(tree: &mut [GitObject]) {
        for obj in tree {
            obj.populate_empty_caches_recursive();
            obj.refresh_empty_caches_for_collapsed();
        }
    }

    /// Count total nodes in a tree (including children)
    pub fn count_nodes(tree: &[GitObject]) -> usize {
        fn count_recursive(node: &GitObject) -> usize {
            1 + node.children.iter().map(count_recursive).sum::<usize>()
        }

        tree.iter().map(count_recursive).sum()
    }

    /// Get tree statistics
    pub fn get_tree_stats(tree: &[GitObject]) -> TreeStats {
        let total_nodes = Self::count_nodes(tree);
        let expanded_nodes = tree.iter().map(Self::count_expanded_nodes).sum();
        let max_depth = tree
            .iter()
            .map(|obj| Self::get_max_depth(obj, 0))
            .max()
            .unwrap_or(0);

        TreeStats {
            total_nodes,
            expanded_nodes,
            max_depth,
        }
    }

    fn count_expanded_nodes(node: &GitObject) -> usize {
        let current = usize::from(node.expanded);
        current
            + node
                .children
                .iter()
                .map(Self::count_expanded_nodes)
                .sum::<usize>()
    }

    fn get_max_depth(node: &GitObject, current_depth: usize) -> usize {
        if node.children.is_empty() {
            current_depth
        } else {
            node.children
                .iter()
                .map(|child| Self::get_max_depth(child, current_depth + 1))
                .max()
                .unwrap_or(current_depth)
        }
    }
}

impl Default for TreeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a tree structure
#[derive(Debug, Clone)]
pub struct TreeStats {
    pub total_nodes: usize,
    pub expanded_nodes: usize,
    pub max_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_service_creation() {
        let service = TreeService::new();
        // TreeService is a stateless service, so just verify it can be created
        let _ = service;
    }

    #[test]
    fn test_collect_all_keys_empty_tree() {
        let tree = vec![];
        let keys = TreeService::collect_all_keys(&tree, |_| "test".to_string());
        assert!(keys.is_empty());
    }

    #[test]
    fn test_count_nodes_empty_tree() {
        let tree = vec![];
        let count = TreeService::count_nodes(&tree);
        assert_eq!(count, 0);
    }
}
