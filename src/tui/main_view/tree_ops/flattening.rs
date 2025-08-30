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

    /// Flatten a tree using pre-computed highlights for perfect alignment
    pub fn flatten_tree_with_precomputed_highlights<G>(
        objects: &[GitObject],
        highlights: &crate::tui::main_view::services::precomputed_highlight_service::PrecomputedHighlights,
        selection_key: G,
    ) -> Vec<FlatTreeRow>
    where
        G: Fn(&GitObject) -> String,
    {
        let mut flat_view = Vec::with_capacity(16);

        for obj in objects {
            Self::flatten_node_with_precomputed_highlights(
                obj,
                0,
                &mut flat_view,
                highlights,
                &selection_key,
            );
        }

        flat_view
    }

    /// Flatten a tree with pre-computed highlights AND ghost overlay support
    pub fn flatten_tree_with_precomputed_highlights_and_ghosts<G>(
        objects: &[GitObject],
        highlights: &crate::tui::main_view::services::precomputed_highlight_service::PrecomputedHighlights,
        selection_key: G,
        ghosts: &HashMap<String, Ghost>,
    ) -> Vec<FlatTreeRow>
    where
        G: Fn(&GitObject) -> String,
    {
        // Start with pre-computed highlighting flattening
        let flat_view =
            Self::flatten_tree_with_precomputed_highlights(objects, highlights, &selection_key);

        // Apply ghost overlay if there are any ghosts
        if ghosts.is_empty() {
            flat_view
        } else {
            Self::apply_ghost_overlay(flat_view, objects, ghosts, &selection_key)
        }
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

        // Handle nested ghosts - smart visibility with ancestor propagation
        for (parent_key_opt, ghost_list) in &by_parent {
            if parent_key_opt.is_none() {
                continue; // Already handled top-level ghosts above
            }

            let parent_key = parent_key_opt.as_ref().unwrap();

            // Find all ancestor folders by parsing the parent key path
            let ancestor_folders = Self::find_ancestor_folders_from_key(parent_key);

            // Find the parent folder in the flattened tree, or the closest visible ancestor
            let (target_folder_key, target_flat_index) = if let Some(parent_flat_index) = output
                .iter()
                .position(|row| selection_key(&row.object) == *parent_key)
            {
                // Parent found in flattened tree
                (parent_key.clone(), parent_flat_index)
            } else {
                // Parent not in flattened tree - find closest visible ancestor
                if let Some((ancestor_key, ancestor_index)) =
                    Self::find_closest_visible_ancestor(&output, &ancestor_folders, &selection_key)
                {
                    (ancestor_key, ancestor_index)
                } else {
                    continue;
                }
            };

            let target_object = &output[target_flat_index].object;
            let target_depth = output[target_flat_index].depth;

            // Check if target folder is expanded
            let is_target_expanded = target_object.expanded;

            if is_target_expanded && target_folder_key == *parent_key {
                // Parent is expanded - show ghosts inside the folder

                // Find where to insert ghosts - after the target folder and its existing children
                let mut insert_index = target_flat_index + 1;

                // Skip existing children to find the insertion point
                while insert_index < output.len() && output[insert_index].depth > target_depth {
                    insert_index += 1;
                }

                // Insert ghosts in reverse order (since we're inserting at the same index)
                for (_sibling_index, ghost_key) in ghost_list.iter().rev() {
                    if let Some(ghost) = ghosts.get(ghost_key) {
                        let ghost_highlight = HighlightInfo {
                            color: Some(ratatui::style::Color::Red),
                            expires_at: Some(ghost.until),
                        };
                        let ghost_row = FlatTreeRow {
                            depth: target_depth + 1,
                            object: ghost.display.clone(),
                            render_status: RenderStatus::PendingRemoval,
                            highlight: ghost_highlight,
                        };
                        output.insert(insert_index, ghost_row);
                    }
                }
            } else {
                // Target folder is collapsed (or we're using an ancestor) - highlight it

                let ghost_expiration = ghost_list
                    .iter()
                    .filter_map(|(_, ghost_key)| ghosts.get(ghost_key))
                    .map(|ghost| ghost.until)
                    .max(); // Use the latest expiration time

                // Highlight the target folder (could be parent or ancestor)
                output[target_flat_index].highlight = HighlightInfo {
                    color: Some(ratatui::style::Color::Red),
                    expires_at: ghost_expiration,
                };

                // Highlight all collapsed ancestor folders
                for ancestor_key in &ancestor_folders {
                    if let Some(ancestor_flat_index) = output
                        .iter()
                        .position(|row| selection_key(&row.object) == *ancestor_key)
                    {
                        let ancestor_object = &output[ancestor_flat_index].object;
                        if !ancestor_object.expanded {
                            output[ancestor_flat_index].highlight = HighlightInfo {
                                color: Some(ratatui::style::Color::Red),
                                expires_at: ghost_expiration,
                            };
                        }
                    }
                }
            }
        }

        output
    }

    /// Find ancestor folders by parsing the parent key path
    /// For example: "<folder:/path/.git/refs/tags>" -> ["refs", "<folder:/path/.git/refs>"]
    fn find_ancestor_folders_from_key(parent_key: &str) -> Vec<String> {
        let mut ancestors = Vec::new();

        // Parse the key to extract path components
        if let Some(path_part) = parent_key.strip_prefix("folder:") {
            // Remove "folder:" prefix
            let path_components: Vec<&str> = path_part.split('/').collect();

            // Find the .git directory index
            if let Some(git_index) = path_components.iter().position(|&p| p == ".git") {
                // Build ancestor keys for each level after .git
                let mut current_path = path_components[..=git_index].join("/");

                for (idx, component) in path_components.iter().enumerate().skip(git_index + 1) {
                    current_path = format!("{current_path}/{component}");
                    if idx == git_index + 1 {
                        // First level after .git could be either a Category or a FileSystemFolder
                        // Add both possible keys since we can't know which format it uses
                        ancestors.push(format!("category:{component}")); // Try Category first
                        ancestors.push(format!("folder:{current_path}")); // Also try FileSystemFolder
                    } else {
                        // Deeper levels are always folders
                        ancestors.push(format!("folder:{current_path}"));
                    }
                }
            }
        }

        // Remove the last element (which is the parent itself)
        if !ancestors.is_empty() {
            ancestors.pop();
        }

        ancestors
    }

    /// Find the closest visible ancestor folder in the flattened tree
    fn find_closest_visible_ancestor<G>(
        flat_view: &[FlatTreeRow],
        ancestor_folders: &[String],
        selection_key: G,
    ) -> Option<(String, usize)>
    where
        G: Fn(&GitObject) -> String,
    {
        // Check ancestors from most specific to least specific (reverse order)
        for ancestor_key in ancestor_folders.iter().rev() {
            if let Some(ancestor_index) = flat_view
                .iter()
                .position(|row| selection_key(&row.object) == *ancestor_key)
            {
                return Some((ancestor_key.clone(), ancestor_index));
            }
        }
        None
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

    /// Recursively flatten a single node using pre-computed highlights
    fn flatten_node_with_precomputed_highlights<G>(
        node: &GitObject,
        depth: usize,
        flat_view: &mut Vec<FlatTreeRow>,
        highlights: &crate::tui::main_view::services::precomputed_highlight_service::PrecomputedHighlights,
        selection_key: &G,
    ) where
        G: Fn(&GitObject) -> String,
    {
        let key = selection_key(node);
        let highlight = highlights.get_highlight(&key);

        flat_view.push(FlatTreeRow {
            depth,
            object: node.clone(),
            render_status: RenderStatus::Normal,
            highlight,
        });

        if node.expanded {
            for child in &node.children {
                Self::flatten_node_with_precomputed_highlights(
                    child,
                    depth + 1,
                    flat_view,
                    highlights,
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
