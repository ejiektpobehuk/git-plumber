use crate::tui::model::{GitObject, GitObjectType};
use std::path::PathBuf;

/// Service responsible for Git repository operations and data loading
pub struct GitRepositoryService;

impl GitRepositoryService {
    /// Create a new `GitRepositoryService` instance
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Load git objects from the repository
    pub fn load_git_objects(plumber: &crate::GitPlumber) -> Result<Vec<GitObject>, String> {
        crate::tui::git_tree::build_git_file_tree(plumber)
    }

    /// Check if an object has been modified by comparing modification times
    #[must_use]
    pub fn is_object_modified(old: &GitObject, new: &GitObject) -> bool {
        Self::is_object_modified_static(old, new)
    }

    /// Static version of `is_object_modified` for use in closures
    #[must_use]
    pub fn is_object_modified_static(old: &GitObject, new: &GitObject) -> bool {
        match (&old.obj_type, &new.obj_type) {
            (
                GitObjectType::PackFolder {
                    base_name: old_name,
                    ..
                },
                GitObjectType::PackFolder {
                    base_name: new_name,
                    ..
                },
            ) => old_name != new_name, // Pack folders are different if base names differ
            (GitObjectType::LooseObject { .. }, GitObjectType::LooseObject { .. }) => {
                // Loose objects are content-addressable and immutable
                // Same object_id = same content, different object_id = different object
                // There's no concept of "modification" for loose objects
                false
            }
            (
                GitObjectType::Ref { path: old_path, .. },
                GitObjectType::Ref { path: new_path, .. },
            ) => Self::compare_file_mtime_paths(old_path, new_path),
            (
                GitObjectType::FileSystemFile { path: old_path, .. },
                GitObjectType::FileSystemFile { path: new_path, .. },
            ) => Self::compare_file_mtime_paths(old_path, new_path),
            (
                GitObjectType::FileSystemFolder { path: old_path, .. },
                GitObjectType::FileSystemFolder { path: new_path, .. },
            ) => {
                if old_path != new_path {
                    return true;
                }
                // For folders, we don't consider them "modified" in the traditional sense
                // Content changes (files added/removed) are handled by the change detection system
                false
            }
            _ => false,
        }
    }

    /// Compare modification times using Path instead of `PathBuf`
    fn compare_file_mtime_paths(old_path: &std::path::Path, new_path: &std::path::Path) -> bool {
        if old_path != new_path {
            return false; // Different paths, not the same file
        }
        Self::is_file_recently_modified_path(old_path)
    }

    /// Check if a file was modified recently using Path
    fn is_file_recently_modified_path(path: &std::path::Path) -> bool {
        if let Ok(meta) = std::fs::metadata(path)
            && let Ok(mtime) = meta.modified()
            && let Ok(elapsed) = mtime.elapsed()
        {
            return elapsed.as_secs() <= 2;
        }
        false
    }

    /// Expand or collapse a folder in the tree
    pub fn toggle_folder_expansion(
        tree: &mut [GitObject],
        target_path: &PathBuf,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        fn find_and_toggle_folder(
            obj: &mut GitObject,
            target_path: &PathBuf,
        ) -> Result<bool, Box<dyn std::error::Error>> {
            if let GitObjectType::FileSystemFolder {
                path, is_loaded, ..
            } = &mut obj.obj_type
                && path == target_path
            {
                if !obj.expanded && !*is_loaded {
                    // Load folder contents before expanding
                    obj.load_folder_contents()?;
                }
                obj.expanded = !obj.expanded;
                return Ok(true);
            }
            for child in &mut obj.children {
                if find_and_toggle_folder(child, target_path)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        for obj in tree {
            if find_and_toggle_folder(obj, target_path)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get repository statistics
    #[must_use]
    pub fn get_repository_stats(tree: &[GitObject]) -> RepositoryStats {
        let mut stats = RepositoryStats::default();

        fn collect_stats(node: &GitObject, stats: &mut RepositoryStats, depth: usize) {
            stats.total_objects += 1;
            stats.max_depth = stats.max_depth.max(depth);

            match &node.obj_type {
                GitObjectType::FileSystemFile { .. } => stats.files += 1,
                GitObjectType::FileSystemFolder { .. } => {
                    stats.folders += 1;
                    if node.expanded {
                        stats.expanded_folders += 1;
                    }
                }
                GitObjectType::PackFolder { .. } => stats.pack_folders += 1,
                GitObjectType::PackFile { .. } => stats.pack_folders += 1,
                GitObjectType::LooseObject { .. } => stats.loose_objects += 1,
                GitObjectType::Category(_) => stats.categories += 1,
                GitObjectType::Ref { .. } => stats.categories += 1,
            }

            for child in &node.children {
                collect_stats(child, stats, depth + 1);
            }
        }

        for obj in tree {
            collect_stats(obj, &mut stats, 0);
        }

        stats
    }

    /// Validate repository structure
    #[must_use]
    pub fn validate_repository_structure(tree: &[GitObject]) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        fn validate_node(node: &GitObject, issues: &mut Vec<ValidationIssue>, path: &str) {
            match &node.obj_type {
                GitObjectType::FileSystemFile {
                    path: file_path, ..
                } => {
                    if !file_path.exists() {
                        issues.push(ValidationIssue {
                            issue_type: ValidationIssueType::MissingFile,
                            path: path.to_string(),
                            message: format!("File does not exist: {file_path:?}"),
                        });
                    }
                }
                GitObjectType::FileSystemFolder {
                    path: folder_path, ..
                } => {
                    if !folder_path.exists() {
                        issues.push(ValidationIssue {
                            issue_type: ValidationIssueType::MissingFolder,
                            path: path.to_string(),
                            message: format!("Folder does not exist: {folder_path:?}"),
                        });
                    }
                }
                _ => {} // Other types don't need filesystem validation
            }

            for (i, child) in node.children.iter().enumerate() {
                validate_node(child, issues, &format!("{path}/[{i}]"));
            }
        }

        for (i, obj) in tree.iter().enumerate() {
            validate_node(obj, &mut issues, &format!("[{i}]"));
        }

        issues
    }
}

impl Default for GitRepositoryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Repository statistics
#[derive(Debug, Default, Clone)]
pub struct RepositoryStats {
    pub total_objects: usize,
    pub files: usize,
    pub folders: usize,
    pub expanded_folders: usize,
    pub pack_folders: usize,
    pub loose_objects: usize,
    pub categories: usize,
    pub max_depth: usize,
}

/// Validation issue types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssueType {
    MissingFile,
    MissingFolder,
    CorruptedData,
    PermissionDenied,
}

/// Validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub issue_type: ValidationIssueType,
    pub path: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_service_creation() {
        let service = GitRepositoryService::new();
        let _ = service;
    }

    #[test]
    fn test_repository_stats_default() {
        let stats = RepositoryStats::default();
        assert_eq!(stats.total_objects, 0);
        assert_eq!(stats.files, 0);
        assert_eq!(stats.folders, 0);
    }

    #[test]
    fn test_validate_empty_repository() {
        let tree = vec![];
        let issues = GitRepositoryService::validate_repository_structure(&tree);
        assert!(issues.is_empty());
    }
}
