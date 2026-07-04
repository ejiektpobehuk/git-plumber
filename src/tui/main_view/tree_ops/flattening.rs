use crate::tui::main_view::{FlatTreeRow, Ghost, HighlightInfo, RenderStatus};
use crate::tui::model::GitObject;
use std::collections::HashMap;

/// Tree flattening utilities for converting hierarchical `GitObject` trees into flat lists
pub struct TreeFlattener;

impl TreeFlattener {
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
        let mut flat_view = if ghosts.is_empty() {
            flat_view
        } else {
            Self::apply_ghost_overlay(flat_view, objects, ghosts, &selection_key)
        };

        // Sibling relationships must include ghost rows, so compute them last
        Self::compute_row_relationships(&mut flat_view);
        flat_view
    }

    /// Fill `is_last_child` and `guides` for every row in O(rows).
    ///
    /// This used to be derived in the renderer by scanning the flat list
    /// backward and forward per row per frame (O(rows²) per redraw).
    fn compute_row_relationships(rows: &mut [FlatTreeRow]) {
        // Backward pass: a row is the last child when no row at the same depth
        // follows it before the tree returns to a shallower depth.
        let mut later_sibling_at: Vec<bool> = Vec::new();
        for row in rows.iter_mut().rev() {
            let d = row.depth;
            if later_sibling_at.len() <= d {
                later_sibling_at.resize(d + 1, false);
            }
            row.is_last_child = !later_sibling_at[d];
            later_sibling_at[d] = true;
            // Rows deeper than this one that come later (earlier in reverse
            // order) belong to this row's subtree, so those levels start fresh.
            for level in later_sibling_at.iter_mut().skip(d + 1) {
                *level = false;
            }
        }

        // Forward pass: ancestor_is_last[d] holds whether the ancestor
        // currently open at depth d is its parent's last child. Indent level
        // `l` needs a │ guide exactly when the ancestor at depth `l + 1` has
        // later siblings.
        let mut ancestor_is_last: Vec<bool> = Vec::new();
        for row in rows.iter_mut() {
            let d = row.depth;
            if ancestor_is_last.len() <= d {
                ancestor_is_last.resize(d + 1, true);
            }
            let mut guides: u64 = 0;
            for level in 0..d.saturating_sub(1).min(u64::BITS as usize) {
                if !ancestor_is_last[level + 1] {
                    guides |= 1 << level;
                }
            }
            row.guides = guides;
            ancestor_is_last[d] = row.is_last_child;
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
                flat.iter().position(|row| row.depth == 0 && row.key == key)
            };

            let end_of_top_level = {
                let last_top_idx = output
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| row.depth == 0)
                    .map(|(i, _)| i)
                    .next_back();
                last_top_idx.map_or(0, |i| {
                    let mut j = i + 1;
                    while j < output.len() {
                        if output[j].depth == 0 {
                            break;
                        }
                        j += 1;
                    }
                    j
                })
            };

            for (sibling_index, ghost_key) in top_list.iter().rev() {
                if let Some(g) = ghosts.get(ghost_key) {
                    let insert_at = if *sibling_index < top_keys.len() {
                        find_top_flat_index(&top_keys[*sibling_index], &output)
                            .unwrap_or(end_of_top_level)
                    } else {
                        end_of_top_level
                    };
                    let ghost_highlight = HighlightInfo {
                        color: Some(ratatui::style::Color::Red),
                        expires_at: Some(g.until),
                        animation_type: crate::tui::main_view::model::AnimationType::FileShrink,
                    };
                    output.insert(
                        insert_at,
                        FlatTreeRow::from_node(
                            &g.display,
                            0,
                            ghost_key.clone(),
                            RenderStatus::PendingRemoval,
                            ghost_highlight,
                        ),
                    );
                }
            }
        }

        // Handle nested ghosts - smart visibility with ancestor propagation
        for (parent_key_opt, ghost_list) in &by_parent {
            let Some(parent_key) = parent_key_opt else {
                continue; // Already handled top-level ghosts above
            };

            // Find all ancestor folders by parsing the parent key path
            let ancestor_folders = Self::find_ancestor_folders_from_key(parent_key);

            // Find the parent folder in the flattened tree, or the closest visible ancestor
            let (target_folder_key, target_flat_index) = if let Some(parent_flat_index) =
                output.iter().position(|row| row.key == *parent_key)
            {
                // Parent found in flattened tree
                (parent_key.clone(), parent_flat_index)
            } else {
                // Parent not in flattened tree - find closest visible ancestor
                if let Some((ancestor_key, ancestor_index)) =
                    Self::find_closest_visible_ancestor(&output, &ancestor_folders)
                {
                    (ancestor_key, ancestor_index)
                } else {
                    continue;
                }
            };

            let target_depth = output[target_flat_index].depth;

            // Check if target folder is expanded
            let is_target_expanded = output[target_flat_index].expanded;

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
                            animation_type: crate::tui::main_view::model::AnimationType::FileShrink,
                        };
                        let ghost_row = FlatTreeRow::from_node(
                            &ghost.display,
                            target_depth + 1,
                            ghost_key.clone(),
                            RenderStatus::PendingRemoval,
                            ghost_highlight,
                        );
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
                    animation_type: crate::tui::main_view::model::AnimationType::FolderBlink,
                };

                // Highlight all collapsed ancestor folders
                for ancestor_key in &ancestor_folders {
                    if let Some(ancestor_flat_index) =
                        output.iter().position(|row| row.key == *ancestor_key)
                        && !output[ancestor_flat_index].expanded
                    {
                        output[ancestor_flat_index].highlight = HighlightInfo {
                            color: Some(ratatui::style::Color::Red),
                            expires_at: ghost_expiration,
                            animation_type:
                                crate::tui::main_view::model::AnimationType::FolderBlink,
                        };
                    }
                }
            }
        }

        output
    }

    /// Find ancestor folders by parsing the parent key path
    /// For example: `folder:/path/.git/refs/tags` -> `["refs", "folder:/path/.git/refs"]`
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
                    }
                    // Deeper levels are always folders; the first level also gets a
                    // folder key in case it is a FileSystemFolder rather than a Category
                    ancestors.push(format!("folder:{current_path}"));
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
    fn find_closest_visible_ancestor(
        flat_view: &[FlatTreeRow],
        ancestor_folders: &[String],
    ) -> Option<(String, usize)> {
        // Check ancestors from most specific to least specific (reverse order)
        for ancestor_key in ancestor_folders.iter().rev() {
            if let Some(ancestor_index) = flat_view.iter().position(|row| row.key == *ancestor_key)
            {
                return Some((ancestor_key.clone(), ancestor_index));
            }
        }
        None
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

        flat_view.push(FlatTreeRow::from_node(
            node,
            depth,
            key,
            RenderStatus::Normal,
            highlight,
        ));

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
    use crate::tui::main_view::MainViewState;
    use crate::tui::main_view::services::precomputed_highlight_service::PrecomputedHighlights;
    use std::path::PathBuf;

    fn flatten(objects: &[GitObject]) -> Vec<FlatTreeRow> {
        TreeFlattener::flatten_tree_with_precomputed_highlights_and_ghosts(
            objects,
            &PrecomputedHighlights::new(),
            MainViewState::selection_key,
            &HashMap::new(),
        )
    }

    /// Reference implementation of the sibling scan the renderer used to run
    /// per row per frame; the precomputed fields must agree with it.
    fn naive_is_last_child(rows: &[FlatTreeRow], i: usize) -> bool {
        let depth = rows[i].depth;
        for row in &rows[i + 1..] {
            if row.depth == depth {
                return false;
            }
            if row.depth < depth {
                break;
            }
        }
        true
    }

    fn naive_guides(rows: &[FlatTreeRow], i: usize) -> u64 {
        let depth = rows[i].depth;
        let mut guides = 0_u64;
        for level in 0..depth.saturating_sub(1) {
            // Find the ancestor of row i at depth level + 1
            let mut ancestor_index = None;
            for k in (0..i).rev() {
                if rows[k].depth == level + 1 {
                    ancestor_index = Some(k);
                    break;
                } else if rows[k].depth <= level {
                    break;
                }
            }
            // The guide is drawn when that ancestor has later siblings
            if ancestor_index.is_some_and(|k| !naive_is_last_child(rows, k)) {
                guides |= 1 << level;
            }
        }
        guides
    }

    #[test]
    fn rows_carry_last_child_flags_and_guides() {
        let mut b = GitObject::new_filesystem_folder(PathBuf::from("/repo/.git/b"), false);
        b.expanded = true;
        b.add_child(GitObject::new_filesystem_file(PathBuf::from(
            "/repo/.git/b/c",
        )));
        b.add_child(GitObject::new_filesystem_file(PathBuf::from(
            "/repo/.git/b/d",
        )));

        let mut a = GitObject::new_category("a");
        a.add_child(b);
        a.add_child(GitObject::new_filesystem_file(PathBuf::from(
            "/repo/.git/e",
        )));

        let f = GitObject::new_filesystem_file(PathBuf::from("/repo/.git/f"));
        let rows = flatten(&[a, f]);

        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d", "e", "f"]);
        let depths: Vec<usize> = rows.iter().map(|r| r.depth).collect();
        assert_eq!(depths, [0, 1, 2, 2, 1, 0]);

        let last: Vec<bool> = rows.iter().map(|r| r.is_last_child).collect();
        assert_eq!(last, [false, false, false, true, true, true]);

        // c and d sit under b; the level-0 guide is drawn because b has a
        // later sibling (e)
        let guides: Vec<u64> = rows.iter().map(|r| r.guides).collect();
        assert_eq!(guides, [0, 0, 0b1, 0b1, 0, 0]);

        // And everything must agree with the renderer's old per-frame scans
        for i in 0..rows.len() {
            assert_eq!(
                rows[i].is_last_child,
                naive_is_last_child(&rows, i),
                "row {i}"
            );
            assert_eq!(rows[i].guides, naive_guides(&rows, i), "row {i}");
        }
    }

    #[test]
    fn collapsed_folders_hide_children() {
        let mut b = GitObject::new_filesystem_folder(PathBuf::from("/repo/.git/b"), false);
        b.expanded = false;
        b.add_child(GitObject::new_filesystem_file(PathBuf::from(
            "/repo/.git/b/c",
        )));

        let mut a = GitObject::new_category("a");
        a.add_child(b);

        let rows = flatten(&[a]);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
        assert!(rows[1].is_last_child);
        assert!(!rows[1].expanded);
        assert_eq!(rows[1].key, "folder:/repo/.git/b");
    }
}
