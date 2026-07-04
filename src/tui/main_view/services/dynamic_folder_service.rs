use crate::tui::main_view::animations::AnimationManager;
use crate::tui::model::{GitObject, GitObjectType};

/// Service for computing dynamic folder highlights based on active file highlights
pub struct DynamicFolderService;

impl DynamicFolderService {
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
                GitObjectType::FileSystemFile { .. }
                | GitObjectType::Ref { .. }
                | GitObjectType::LooseObject { .. }
                | GitObjectType::PackFile { .. } => {
                    items.push(selection_key_fn(child));
                }

                // Folder types - include the folder itself as highlightable AND recurse into it
                GitObjectType::FileSystemFolder { .. }
                | GitObjectType::Category(_)
                | GitObjectType::PackFolder { .. } => {
                    items.push(selection_key_fn(child));
                    items.extend(Self::collect_all_files_in_folder(child, selection_key_fn));
                }
            }
        }

        items
    }

    /// Check if any files inside closed folders have active highlights (for animation detection)
    pub fn has_active_folder_highlights(
        animations: &AnimationManager,
        tree: &[GitObject],
        selection_key_fn: fn(&GitObject) -> String,
    ) -> bool {
        Self::check_tree_for_folder_highlights(animations, tree, selection_key_fn)
    }

    /// Recursively check tree for any closed folders with active file highlights
    fn check_tree_for_folder_highlights(
        animations: &AnimationManager,
        tree: &[GitObject],
        selection_key_fn: fn(&GitObject) -> String,
    ) -> bool {
        for obj in tree {
            match &obj.obj_type {
                // Check all folder types
                GitObjectType::FileSystemFolder { .. }
                | GitObjectType::Category(_)
                | GitObjectType::PackFolder { .. } => {
                    if !obj.expanded {
                        // This is a closed folder - check if it would have highlights
                        let files_inside = Self::collect_all_files_in_folder(obj, selection_key_fn);
                        let folder_key = selection_key_fn(obj);

                        if animations
                            .compute_folder_highlight(&folder_key, &files_inside)
                            .is_some()
                        {
                            return true;
                        }
                    }

                    // Also check children (whether expanded or not)
                    if Self::check_tree_for_folder_highlights(
                        animations,
                        &obj.children,
                        selection_key_fn,
                    ) {
                        return true;
                    }
                }
                _ => {
                    // For non-folder types, check children
                    if Self::check_tree_for_folder_highlights(
                        animations,
                        &obj.children,
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
