use crate::tui::model::{GitObject, GitObjectType};

/// Service responsible for Git repository operations and data loading
pub struct GitRepositoryService;

impl GitRepositoryService {
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
            (
                GitObjectType::Ref { path: old_path, .. },
                GitObjectType::Ref { path: new_path, .. },
            )
            | (
                GitObjectType::FileSystemFile { path: old_path, .. },
                GitObjectType::FileSystemFile { path: new_path, .. },
            ) => Self::compare_file_mtime_paths(old_path, new_path),
            (
                GitObjectType::FileSystemFolder { path: old_path, .. },
                GitObjectType::FileSystemFolder { path: new_path, .. },
            ) => {
                // For folders, we don't consider them "modified" in the traditional sense
                // Content changes (files added/removed) are handled by the change detection system
                old_path != new_path
            }
            // Loose objects are content-addressable and immutable: same object_id = same
            // content, so there's no concept of "modification" for them. Other type
            // combinations are never considered modified either.
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
}
