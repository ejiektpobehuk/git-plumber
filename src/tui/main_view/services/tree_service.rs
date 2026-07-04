use crate::tui::main_view::animations::AnimationManager;
use crate::tui::main_view::{FlatTreeRow, TreeFlattener};
use crate::tui::model::GitObject;

/// Service responsible for all tree operations including flattening, traversal, and manipulation
pub struct TreeService;

impl TreeService {
    /// Flatten a tree with pre-computed highlights for perfect alignment
    pub fn flatten_tree_with_precomputed_highlights(
        tree: &[GitObject],
        animations: &AnimationManager,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Vec<FlatTreeRow> {
        // Step 1: Pre-compute all highlights for perfect alignment
        let highlights = super::PrecomputedHighlightService::compute_all_highlights(
            tree,
            animations,
            selection_key_fn,
        );

        // Step 2: Flatten tree using pre-computed highlights WITH ghost overlay support!
        TreeFlattener::flatten_tree_with_precomputed_highlights_and_ghosts(
            tree,
            &highlights,
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

    /// Find a node in the tree by its selection key, mutably
    pub fn find_node_by_key_mut<'a>(
        tree: &'a mut [GitObject],
        key: &str,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Option<&'a mut GitObject> {
        for child in tree {
            if selection_key_fn(child) == key {
                return Some(child);
            }
            if let Some(found) =
                Self::find_node_by_key_mut(&mut child.children, key, selection_key_fn)
            {
                return Some(found);
            }
        }
        None
    }
}
