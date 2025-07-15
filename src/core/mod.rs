use crate::git::repository::{Repository, RepositoryError};
use std::path::{Path, PathBuf};

/// Main application struct that handles shared logic
pub struct GitPlumber {
    repo_path: PathBuf,
    repository: Option<Repository>,
}

impl GitPlumber {
    /// Create a new `GitPlumber` instance
    pub fn new(repo_path: impl AsRef<Path>) -> Self {
        let repo_path = repo_path.as_ref().to_path_buf();
        let repository = Repository::new(&repo_path).ok();

        Self {
            repo_path,
            repository,
        }
    }

    /// Get the repository path
    pub fn get_repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Get access to the repository if it exists
    pub fn get_repository(&self) -> Option<&Repository> {
        self.repository.as_ref()
    }

    /// List all pack files in the repository
    pub fn list_pack_files(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.list_pack_files(),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// List all head refs (local branches) in the repository
    pub fn list_head_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.list_head_refs(),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// List all remote refs in the repository
    pub fn list_remote_refs(&self) -> Result<Vec<(String, Vec<PathBuf>)>, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.list_remote_refs(),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// List all tag refs in the repository
    pub fn list_tag_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.list_tag_refs(),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// Check if stash ref exists
    pub fn has_stash_ref(&self) -> Result<bool, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.has_stash_ref(),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// List a sample of loose objects in the repository
    pub fn list_loose_objects(&self, limit: usize) -> Result<Vec<PathBuf>, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.list_loose_objects(limit),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// List parsed loose objects in the repository
    pub fn list_parsed_loose_objects(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::git::loose_object::LooseObject>, RepositoryError> {
        match &self.repository {
            Some(repo) => repo.list_parsed_loose_objects(limit),
            None => Err(RepositoryError::NotGitRepository(format!(
                "{} is not a git repository",
                self.repo_path.display()
            ))),
        }
    }

    /// Parse and display a pack file
    pub fn parse_pack_file(&self, path: &Path) -> Result<(), String> {
        // Read the pack file
        let pack_data = std::fs::read(path).map_err(|e| format!("Error reading file: {e}"))?;

        // Parse the pack file
        match crate::git::pack::Header::parse(&pack_data) {
            Ok((objects_data, header)) => {
                println!("Pack version: {}", header.version);
                println!("Number of objects: {}", header.object_count);
                let mut remaining_data = objects_data;
                for i in 0..header.object_count {
                    match crate::git::pack::Object::parse(remaining_data) {
                        Ok((new_remaining_data, object)) => {
                            println!("{object}");
                            remaining_data = new_remaining_data;
                        }
                        Err(e) => {
                            return Err(format!("Error parsing object: {e}"));
                        }
                    }
                    if i < header.object_count - 1 {
                        println!("--------------------------------");
                    }
                }
                Ok(())
            }
            Err(e) => Err(format!("Error parsing pack file: {e}")),
        }
    }
}
