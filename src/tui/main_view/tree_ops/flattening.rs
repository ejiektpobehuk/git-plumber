use crate::tui::main_view::{FlatTreeRow, Ghost, HighlightInfo, RenderStatus};
use crate::tui::model::GitObject;
use std::collections::HashMap;

/// Tree flattening utilities for converting hierarchical `GitObject` trees into flat lists
pub struct TreeFlattener;

impl TreeFlattener {
    /// Flatten a tree of `GitObjects` into a flat list of `FlatTreeRows` for display
    /// This is the basic flattening without ghost overlays or complex highlighting
    pub fn flatten_tree_basic<F, G>(
        objects: &[GitObject],
        compute_highlight: F,
        selection_key: G,
    ) -> Vec<FlatTreeRow>
    where
        F: Fn(&str) -> HighlightInfo,
        G: Fn(&GitObject) -> String,
    {
        let mut flat_view = Vec::with_capacity(16);

        for obj in objects {
            Self::flatten_node_recursive(
                obj,
                0,
                &mut flat_view,
                &compute_highlight,
                &selection_key,
            );
        }

        flat_view
    }

    /// Comprehensive tree flattening with ghost overlay support
    pub fn flatten_tree_with_ghosts<F, G>(
        objects: &[GitObject],
        compute_highlight: F,
        selection_key: G,
        ghosts: &HashMap<String, Ghost>,
    ) -> Vec<FlatTreeRow>
    where
        F: Fn(&str) -> HighlightInfo,
        G: Fn(&GitObject) -> String,
    {
        // Start with basic flattening
        let flat_view = Self::flatten_tree_basic(objects, &compute_highlight, &selection_key);

        // Apply ghost overlay if there are any ghosts
        if ghosts.is_empty() {
            flat_view
        } else {
            Self::apply_ghost_overlay(flat_view, objects, ghosts, &selection_key)
        }
    }

    /// Apply ghost overlay to the flattened tree
    fn apply_ghost_overlay<G>(
        flat_view: Vec<FlatTreeRow>,
        objects: &[GitObject],
        ghosts: &HashMap<String, Ghost>,
        selection_key: G,
    ) -> Vec<FlatTreeRow>
    where
        G: Fn(&GitObject) -> String,
    {
        // Group ghosts by parent_key
        let mut by_parent: HashMap<Option<String>, Vec<(usize, String)>> = HashMap::new();
        for (k, g) in ghosts {
            by_parent
                .entry(g.parent_key.clone())
                .or_default()
                .push((g.sibling_index, k.clone()));
        }
        for v in by_parent.values_mut() {
            v.sort_by_key(|(idx, _)| *idx);
        }

        let mut output: Vec<FlatTreeRow> = flat_view;

        // Top-level ghosts: precise mapping by sibling_index against top-level order
        if let Some(top_list) = by_parent.get(&None) {
            let top_keys: Vec<String> = objects.iter().map(&selection_key).collect();

            let find_top_flat_index = |key: &str, flat: &[FlatTreeRow]| -> Option<usize> {
                flat.iter()
                    .position(|row| row.depth == 0 && selection_key(&row.object) == key)
            };

            let end_of_top_level = {
                let last_top_idx = output
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| row.depth == 0)
                    .map(|(i, _)| i)
                    .next_back();
                match last_top_idx {
                    Some(i) => {
                        let mut j = i + 1;
                        while j < output.len() {
                            if output[j].depth == 0 {
                                break;
                            }
                            j += 1;
                        }
                        j
                    }
                    None => 0,
                }
            };

            for (sibling_index, ghost_key) in top_list.iter().rev() {
                if let Some(g) = ghosts.get(ghost_key) {
                    let insert_at = if *sibling_index < top_keys.len() {
                        if let Some(idx) = find_top_flat_index(&top_keys[*sibling_index], &output) {
                            idx
                        } else {
                            end_of_top_level
                        }
                    } else {
                        end_of_top_level
                    };
                    let ghost_highlight = HighlightInfo {
                        color: Some(ratatui::style::Color::Red),
                        expires_at: Some(g.until),
                    };
                    output.insert(
                        insert_at,
                        FlatTreeRow {
                            depth: 0,
                            object: g.display.clone(),
                            render_status: RenderStatus::PendingRemoval,
                            highlight: ghost_highlight,
                        },
                    );
                }
            }
        }

        // Handle nested ghosts (simplified version for now)
        // This is complex logic that we can enhance later if needed

        output
    }

    /// Recursively flatten a single node and its children
    fn flatten_node_recursive<F, G>(
        node: &GitObject,
        depth: usize,
        flat_view: &mut Vec<FlatTreeRow>,
        compute_highlight: &F,
        selection_key: &G,
    ) where
        F: Fn(&str) -> HighlightInfo,
        G: Fn(&GitObject) -> String,
    {
        let key = selection_key(node);
        let highlight = compute_highlight(&key);

        flat_view.push(FlatTreeRow {
            depth,
            object: node.clone(),
            render_status: RenderStatus::Normal,
            highlight,
        });

        if node.expanded {
            for child in &node.children {
                Self::flatten_node_recursive(
                    child,
                    depth + 1,
                    flat_view,
                    compute_highlight,
                    selection_key,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::GitObject;

    #[test]
    fn test_basic_flattening() {
        // Create a simple test tree
        let mut root = GitObject::new_category("root");
        let child = GitObject::new_category("child");
        root.add_child(child);
        root.expanded = true;

        let objects = vec![root];

        let flat = TreeFlattener::flatten_tree_basic(
            &objects,
            |_| HighlightInfo::default(),
            |obj| format!("test:{}", obj.name),
        );

        assert_eq!(flat.len(), 2); // root + child
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[1].depth, 1);
    }
}
