use crate::tui::main_view::OldPosition;
use crate::tui::model::GitObject;
use std::collections::{HashMap, HashSet};

/// Types and utilities for detecting changes in the Git object tree
pub mod types {
    use super::{GitObject, HashMap, OldPosition};

    /// Snapshot of tree positions for change detection
    #[derive(Debug, Clone)]
    pub struct TreeSnapshot {
        pub positions: HashMap<String, OldPosition>,
        pub nodes: HashMap<String, GitObject>,
    }
}

pub use types::*;

/// Service for detecting changes in Git object trees
pub struct ChangeDetectionService;

impl ChangeDetectionService {
    /// Create a snapshot of the current tree state for later comparison
    pub fn snapshot_tree_positions(
        objects: &[GitObject],
        selection_key: impl Fn(&GitObject) -> String,
    ) -> TreeSnapshot {
        fn walk(
            out_pos: &mut HashMap<String, OldPosition>,
            out_nodes: &mut HashMap<String, GitObject>,
            children: &[GitObject],
            parent_key: Option<String>,
            selection_key: &impl Fn(&GitObject) -> String,
        ) {
            for (idx, child) in children.iter().enumerate() {
                let key = selection_key(child);
                out_pos.insert(
                    key.clone(),
                    OldPosition {
                        parent_key: parent_key.clone(),
                        sibling_index: idx,
                    },
                );
                out_nodes.insert(key.clone(), child.clone());
                match &child.obj_type {
                    crate::tui::model::GitObjectType::Category(_)
                    | crate::tui::model::GitObjectType::FileSystemFolder { .. } => {
                        walk(
                            out_pos,
                            out_nodes,
                            &child.children,
                            Some(key),
                            selection_key,
                        );
                    }
                    _ => {}
                }
            }
        }

        let mut positions = HashMap::new();
        let mut nodes = HashMap::new();
        walk(&mut positions, &mut nodes, objects, None, &selection_key);

        TreeSnapshot { positions, nodes }
    }

    /// Collect all keys from the current tree
    pub fn collect_all_keys(
        objects: &[GitObject],
        selection_key: impl Fn(&GitObject) -> String,
    ) -> HashSet<String> {
        fn walk_keys(
            children: &[GitObject],
            acc: &mut HashSet<String>,
            selection_key: &impl Fn(&GitObject) -> String,
        ) {
            for child in children {
                let key = selection_key(child);
                acc.insert(key);
                match &child.obj_type {
                    crate::tui::model::GitObjectType::Category(_)
                    | crate::tui::model::GitObjectType::FileSystemFolder { .. } => {
                        walk_keys(&child.children, acc, selection_key);
                    }
                    _ => {}
                }
            }
        }

        let mut keys = HashSet::new();
        walk_keys(objects, &mut keys, &selection_key);
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::GitObject;

    #[test]
    fn test_snapshot_creation() {
        let mut root = GitObject::new_category("root");
        let child = GitObject::new_category("child");
        root.add_child(child);

        let objects = vec![root];
        let snapshot = ChangeDetectionService::snapshot_tree_positions(&objects, |obj| {
            format!("test:{}", obj.name)
        });

        assert_eq!(snapshot.positions.len(), 2); // root + child
        assert_eq!(snapshot.nodes.len(), 2);
    }

    #[test]
    fn test_key_collection() {
        let root = GitObject::new_category("root");
        let objects = vec![root];

        let keys =
            ChangeDetectionService::collect_all_keys(&objects, |obj| format!("test:{}", obj.name));

        assert_eq!(keys.len(), 1);
        assert!(keys.contains("test:root"));
    }
}
