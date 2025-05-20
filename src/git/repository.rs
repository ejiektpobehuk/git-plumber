use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Not a git repository: {0}")]
    NotGitRepository(String),
}

/// Represents a Git repository
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// Creates a new Repository instance from a path
    pub fn new(path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        let path = path.as_ref().to_path_buf();
        let git_dir = path.join(".git");

        if !git_dir.exists() {
            return Err(RepositoryError::NotGitRepository(
                "No .git directory found".to_string(),
            ));
        }

        Ok(Self { path })
    }

    /// Returns the path to the repository
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Lists all pack files in the repository
    pub fn list_pack_files(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        let pack_dir = self.path.join(".git/objects/pack");

        if !pack_dir.exists() {
            return Ok(Vec::new()); // Return empty list if pack directory doesn't exist
        }

        let mut pack_files = Vec::new();
        for entry in fs::read_dir(pack_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pack") {
                pack_files.push(path);
            }
        }

        Ok(pack_files)
    }

    /// Lists all head refs (local branches) in the repository
    pub fn list_head_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        self.list_refs_in_dir(self.path.join(".git/refs/heads"))
    }

    /// Lists all remote refs in the repository
    pub fn list_remote_refs(&self) -> Result<Vec<(String, Vec<PathBuf>)>, RepositoryError> {
        let remotes_dir = self.path.join(".git/refs/remotes");
        if !remotes_dir.exists() {
            return Ok(Vec::new());
        }

        let mut remotes = Vec::new();
        for entry in fs::read_dir(remotes_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let remote_name = entry.file_name().to_string_lossy().to_string();

                let remote_refs = self.list_refs_in_dir(entry.path())?;
                remotes.push((remote_name, remote_refs));
            }
        }

        Ok(remotes)
    }

    /// Lists all tag refs in the repository
    pub fn list_tag_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        self.list_refs_in_dir(self.path.join(".git/refs/tags"))
    }

    /// Checks if stash ref exists
    pub fn has_stash_ref(&self) -> Result<bool, RepositoryError> {
        let stash_path = self.path.join(".git/refs/stash");
        Ok(stash_path.exists() && stash_path.is_file())
    }

    /// Helper method to list refs in a directory
    fn list_refs_in_dir(&self, dir_path: PathBuf) -> Result<Vec<PathBuf>, RepositoryError> {
        if !dir_path.exists() {
            return Ok(Vec::new());
        }

        let mut refs = Vec::new();
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                refs.push(path);
            }
        }

        Ok(refs)
    }

    /// Lists a sample of loose objects in the repository
    /// Limit parameter controls the maximum number of objects to return
    pub fn list_loose_objects(&self, limit: usize) -> Result<Vec<PathBuf>, RepositoryError> {
        let objects_dir = self.path.join(".git/objects");
        if !objects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut loose_objects = Vec::new();
        let mut count = 0;

        for entry in fs::read_dir(&objects_dir)? {
            let entry = entry?;
            let dir_name = entry.file_name().to_string_lossy().to_string();

            // Skip info and pack directories
            if dir_name == "info" || dir_name == "pack" || !entry.path().is_dir() {
                continue;
            }

            if let Ok(subentries) = fs::read_dir(entry.path()) {
                for subentry in subentries.flatten() {
                    if count < limit {
                        loose_objects.push(subentry.path());
                        count += 1;
                    } else {
                        return Ok(loose_objects);
                    }
                }
            }
        }

        Ok(loose_objects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_list_pack_files() {
        // Create a temporary directory structure
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git/objects/pack");
        fs::create_dir_all(&git_dir).unwrap();

        // Create some test pack files
        fs::write(git_dir.join("pack-1.pack"), b"").unwrap();
        fs::write(git_dir.join("pack-2.pack"), b"").unwrap();
        fs::write(git_dir.join("pack-1.idx"), b"").unwrap(); // Should be ignored

        let repo = Repository::new(temp_dir.path()).unwrap();
        let pack_files = repo.list_pack_files().unwrap();

        assert_eq!(pack_files.len(), 2);
        assert!(pack_files.iter().all(|p| p.extension().unwrap() == "pack"));
    }
}
