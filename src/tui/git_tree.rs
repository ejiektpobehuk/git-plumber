use std::fs;
use std::path::{Path, PathBuf};

use crate::tui::model::{GitObject, GitObjectType};

/// Recursively build a GitObject tree from a directory (e.g., .git)
pub fn build_git_tree_from_dir(dir_path: &Path) -> GitObject {
    let name = dir_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir_path.to_string_lossy().to_string());

    // Placeholder: attach educational info to known directories/files
    let educational = None;

    let mut node = GitObject::new_directory(&name, dir_path.to_path_buf(), educational);

    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry
                .file_name()
                .to_string_lossy()
                .to_string();
            if path.is_dir() {
                let child = build_git_tree_from_dir(&path);
                node.add_child(child);
            } else if path.is_file() {
                let size = fs::metadata(&path).ok().map(|m| m.len());
                // Placeholder: attach educational info to known files
                let educational = None;
                let child = GitObject::new_file(&file_name, path, size, educational);
                node.add_child(child);
            }
        }
    }
    node
} 