use crate::tui::main_view::HighlightInfo;
use crate::tui::main_view::animations::AnimationManager;
use crate::tui::model::{GitObject, GitObjectType};

/// Service for computing dynamic folder highlights based on active file highlights
pub struct DynamicFolderService;

impl DynamicFolderService {
    /// Compute highlight info that includes dynamic folder highlighting
    pub fn compute_highlight_with_folders(
        animations: &AnimationManager,
        tree: &[GitObject],
        key: &str,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> HighlightInfo {
        // First check if it's a regular file/ghost highlight
        let regular_highlight = animations.compute_highlight_info(key);
        if regular_highlight.color.is_some() {
            return regular_highlight;
        }

        // Check if this is a folder key and if the folder is closed
        if let Some(folder) = Self::find_folder_by_key(tree, key, selection_key_fn)
            && !folder.expanded
        {
            // This is a closed folder - check for dynamic highlights
            let files_inside = Self::collect_all_files_in_folder(folder, selection_key_fn);

            if let Some(dynamic_highlight) = animations.compute_folder_highlight(key, &files_inside)
            {
                return HighlightInfo {
                    color: Some(dynamic_highlight.color),
                    expires_at: Some(dynamic_highlight.expires_at),
                };
            }
        }

        // No highlight
        HighlightInfo::default()
    }

    /// Find a folder object by its key (includes all folder-like objects)
    fn find_folder_by_key<'a>(
        tree: &'a [GitObject],
        key: &str,
        selection_key_fn: fn(&GitObject) -> String,
    ) -> Option<&'a GitObject> {
        for obj in tree {
            if selection_key_fn(obj) == key {
                // Check if this is any kind of folder-like object that can contain files
                match &obj.obj_type {
                    GitObjectType::FileSystemFolder { .. } => return Some(obj),
                    GitObjectType::Category(_) => return Some(obj),
                    GitObjectType::PackFolder { .. } => return Some(obj),
                    _ => {} // Not a folder type
                }
            }
            // Recursively search in children
            if let Some(found) = Self::find_folder_by_key(&obj.children, key, selection_key_fn) {
                return Some(found);
            }
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::GitObject;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    #[test]
    fn test_dynamic_folder_highlighting() {
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
            now + Duration::from_secs(5),
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

        // Test that the closed src folder gets highlighted
        let highlight = DynamicFolderService::compute_highlight_with_folders(
            &animations,
            &tree,
            "folder:/test/src",
            selection_key,
        );

        assert!(highlight.color.is_some());
        assert_eq!(highlight.color.unwrap(), ratatui::style::Color::Green);

        // Test that the open root folder does not get highlighted
        let root_highlight = DynamicFolderService::compute_highlight_with_folders(
            &animations,
            &tree,
            "folder:/test",
            selection_key,
        );

        assert!(root_highlight.color.is_none());
    }
}
