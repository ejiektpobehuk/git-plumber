use crate::git::loose_object::{LooseObject, LooseObjectError};
use crate::git::pack::{PackError, PackIndex};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Represents a group of pack-related files with the same base name
#[derive(Debug, Clone)]
pub struct PackGroup {
    pub base_name: String,
    pub pack_file: Option<PathBuf>,
    pub idx_file: Option<PathBuf>,
    pub rev_file: Option<PathBuf>,
    pub mtimes_file: Option<PathBuf>,
}

impl PackGroup {
    /// Creates a new PackGroup with the given base name
    pub fn new(base_name: &str) -> Self {
        Self {
            base_name: base_name.to_string(),
            pack_file: None,
            idx_file: None,
            rev_file: None,
            mtimes_file: None,
        }
    }

    /// Returns true if this group has at least a .pack file
    pub fn is_valid(&self) -> bool {
        self.pack_file.is_some()
    }

    /// Returns true if this group has both .pack and .idx files
    pub fn has_index(&self) -> bool {
        self.pack_file.is_some() && self.idx_file.is_some()
    }

    /// Returns all available file paths in this group
    pub fn get_all_files(&self) -> Vec<(&str, &PathBuf)> {
        let mut files = Vec::new();

        if let Some(ref path) = self.pack_file {
            files.push(("packfile", path));
        }
        if let Some(ref path) = self.idx_file {
            files.push(("index", path));
        }
        if let Some(ref path) = self.rev_file {
            files.push(("xedni", path)); // reversed index
        }
        if let Some(ref path) = self.mtimes_file {
            files.push(("mtime", path)); // mtimes
        }

        files
    }

    /// Load and parse the index file if available
    pub fn load_index(&self) -> Result<Option<PackIndex>, PackError> {
        if let Some(ref idx_path) = self.idx_file {
            match std::fs::read(idx_path) {
                Ok(data) => match PackIndex::parse(&data) {
                    Ok((_, index)) => Ok(Some(index)),
                    Err(e) => Err(PackError::ParseError(format!(
                        "Failed to parse index: {:?}",
                        e
                    ))),
                },
                Err(e) => Err(PackError::DecompressionError(e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Look up an object by SHA-1 hash using the index file
    /// Returns the byte offset in the pack file if found
    pub fn lookup_object_offset(&self, sha1: &[u8; 20]) -> Result<Option<u64>, PackError> {
        match self.load_index()? {
            Some(index) => Ok(index.lookup_object(sha1)),
            None => Ok(None),
        }
    }

    /// Get basic statistics about the pack group
    pub fn get_stats(&self) -> Result<PackGroupStats, PackError> {
        let mut stats = PackGroupStats {
            base_name: self.base_name.clone(),
            has_pack: self.pack_file.is_some(),
            has_index: self.idx_file.is_some(),
            has_rev: self.rev_file.is_some(),
            has_mtimes: self.mtimes_file.is_some(),
            object_count: None,
            pack_size: None,
            index_size: None,
        };

        // Get pack file size
        if let Some(ref pack_path) = self.pack_file
            && let Ok(metadata) = std::fs::metadata(pack_path)
        {
            stats.pack_size = Some(metadata.len());
        }

        // Get index file size and object count
        if let Some(ref idx_path) = self.idx_file {
            if let Ok(metadata) = std::fs::metadata(idx_path) {
                stats.index_size = Some(metadata.len());
            }

            // Load index to get object count
            if let Ok(Some(index)) = self.load_index() {
                stats.object_count = Some(index.object_count());
            }
        }

        Ok(stats)
    }
}

/// Statistics about a pack group
#[derive(Debug, Clone)]
pub struct PackGroupStats {
    pub base_name: String,
    pub has_pack: bool,
    pub has_index: bool,
    pub has_rev: bool,
    pub has_mtimes: bool,
    pub object_count: Option<usize>,
    pub pack_size: Option<u64>,
    pub index_size: Option<u64>,
}

impl std::fmt::Display for PackGroupStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Pack Group: {}", self.base_name)?;

        if let Some(count) = self.object_count {
            writeln!(f, "Objects: {}", count)?;
        }

        if let Some(size) = self.pack_size {
            writeln!(f, "Pack size: {} bytes", size)?;
        }

        if let Some(size) = self.index_size {
            writeln!(f, "Index size: {} bytes", size)?;
        }

        writeln!(f, "Files present:")?;
        writeln!(f, "  Pack: {}", if self.has_pack { "✓" } else { "✗" })?;
        writeln!(f, "  Index: {}", if self.has_index { "✓" } else { "✗" })?;
        writeln!(f, "  Rev: {}", if self.has_rev { "✓" } else { "✗" })?;
        writeln!(f, "  Mtimes: {}", if self.has_mtimes { "✓" } else { "✗" })?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Not a git repository: {0}")]
    NotGitRepository(String),

    #[error("Loose object error: {0}")]
    LooseObjectError(#[from] LooseObjectError),

    #[error("Pack error: {0}")]
    PackError(#[from] PackError),
}

/// Statistics about loose objects in the repository
#[derive(Debug, Clone, Default)]
pub struct LooseObjectStats {
    pub total_count: usize,
    pub total_size: usize,
    pub commit_count: usize,
    pub tree_count: usize,
    pub blob_count: usize,
    pub tag_count: usize,
}

impl LooseObjectStats {
    /// Get a formatted summary of the statistics
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Total: {} objects ({} bytes)\nCommits: {}, Trees: {}, Blobs: {}, Annotated Tags: {}",
            self.total_count,
            self.total_size,
            self.commit_count,
            self.tree_count,
            self.blob_count,
            self.tag_count
        )
    }
}

/// Represents a Git repository
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// Creates a new Repository instance from a path
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The provided path does not contain a .git directory
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
    #[must_use]
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Lists all pack files in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading the objects/pack directory
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

    /// Lists all pack-related files grouped by their base name (without extension)
    ///
    /// Returns a map where keys are base names (e.g., "pack-abc123") and values are
    /// structs containing paths to all related files (.pack, .idx, .rev, .mtimes)
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading the objects/pack directory
    pub fn list_pack_groups(&self) -> Result<HashMap<String, PackGroup>, RepositoryError> {
        let pack_dir = self.path.join(".git/objects/pack");

        if !pack_dir.exists() {
            return Ok(HashMap::new());
        }

        let mut pack_groups: HashMap<String, PackGroup> = HashMap::new();

        for entry in fs::read_dir(pack_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(extension) = path.extension().and_then(|ext| ext.to_str())
                && let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str())
            {
                let group = pack_groups
                    .entry(file_stem.to_string())
                    .or_insert_with(|| PackGroup::new(file_stem));

                match extension {
                    "pack" => group.pack_file = Some(path),
                    "idx" => group.idx_file = Some(path),
                    "rev" => group.rev_file = Some(path),
                    "mtimes" => group.mtimes_file = Some(path),
                    _ => {} // Ignore other extensions
                }
            }
        }

        Ok(pack_groups)
    }

    /// Lists all head refs (local branches) in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading the refs/heads directory
    pub fn list_head_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        Self::list_refs_in_dir(self.path.join(".git/refs/heads"))
    }

    /// Lists all remote refs grouped by remote name
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading the refs/remotes directory
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

                let remote_refs = Self::list_refs_in_dir(entry.path())?;
                remotes.push((remote_name, remote_refs));
            }
        }

        Ok(remotes)
    }

    /// Lists all tag refs in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading the refs/tags directory
    pub fn list_tag_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        Self::list_refs_in_dir(self.path.join(".git/refs/tags"))
    }

    /// Checks if stash ref exists
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when checking for stash refs
    pub fn has_stash_ref(&self) -> Result<bool, RepositoryError> {
        let stash_path = self.path.join(".git/refs/stash");
        Ok(stash_path.exists())
    }

    /// Helper method to list refs in a directory
    fn list_refs_in_dir(dir_path: PathBuf) -> Result<Vec<PathBuf>, RepositoryError> {
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
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading loose object directories
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

    /// Reads and parses a loose object from the given path
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file cannot be read
    /// - The object cannot be decompressed or parsed
    pub fn read_loose_object(&self, path: &Path) -> Result<LooseObject, RepositoryError> {
        Ok(LooseObject::read_from_path(path)?)
    }

    /// Reads and parses a loose object by its hash
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The object file cannot be found or read
    /// - The object cannot be decompressed or parsed
    pub fn read_loose_object_by_hash(&self, hash: &str) -> Result<LooseObject, RepositoryError> {
        if hash.len() != 40 {
            return Err(RepositoryError::LooseObjectError(
                LooseObjectError::InvalidFormat("Hash must be 40 characters".to_string()),
            ));
        }

        let (dir, file) = hash.split_at(2);
        let path = self.path.join(".git/objects").join(dir).join(file);

        self.read_loose_object(&path)
    }

    /// List parsed loose objects with a limit
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading loose object directories  
    /// - Objects cannot be parsed or decompressed
    pub fn list_parsed_loose_objects(
        &self,
        limit: usize,
    ) -> Result<Vec<LooseObject>, RepositoryError> {
        let loose_object_paths = self.list_loose_objects(limit)?;
        let mut parsed_objects = Vec::new();

        for path in loose_object_paths {
            match self.read_loose_object(&path) {
                Ok(object) => parsed_objects.push(object),
                Err(e) => {
                    // Log the error but continue processing other objects
                    eprintln!(
                        "Warning: Failed to parse loose object {}: {e}",
                        path.display()
                    );
                }
            }
        }

        Ok(parsed_objects)
    }

    /// Check if a loose object exists by its hash
    #[must_use]
    pub fn loose_object_exists(&self, hash: &str) -> bool {
        if hash.len() != 40 {
            return false;
        }

        let (dir, file) = hash.split_at(2);
        let path = self.path.join(".git/objects").join(dir).join(file);

        path.exists() && path.is_file()
    }

    /// Get statistics about all loose objects in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - File system operations fail when reading loose object directories
    /// - Objects cannot be parsed or analyzed
    pub fn get_loose_object_stats(&self) -> Result<LooseObjectStats, RepositoryError> {
        let objects_dir = self.path.join(".git/objects");
        if !objects_dir.exists() {
            return Ok(LooseObjectStats::default());
        }

        let mut stats = LooseObjectStats::default();

        for entry in fs::read_dir(&objects_dir)? {
            let entry = entry?;
            let dir_name = entry.file_name().to_string_lossy().to_string();

            // Skip info and pack directories
            if dir_name == "info" || dir_name == "pack" || !entry.path().is_dir() {
                continue;
            }

            if let Ok(subentries) = fs::read_dir(entry.path()) {
                for subentry in subentries.flatten() {
                    if let Ok(object) = self.read_loose_object(&subentry.path()) {
                        stats.total_count += 1;
                        stats.total_size += object.size;

                        match object.object_type {
                            crate::git::loose_object::LooseObjectType::Commit => {
                                stats.commit_count += 1;
                            }
                            crate::git::loose_object::LooseObjectType::Tree => {
                                stats.tree_count += 1;
                            }
                            crate::git::loose_object::LooseObjectType::Blob => {
                                stats.blob_count += 1;
                            }
                            crate::git::loose_object::LooseObjectType::Tag => stats.tag_count += 1,
                        }
                    }
                }
            }
        }

        Ok(stats)
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
