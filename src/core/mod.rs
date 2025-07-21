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
    #[must_use]
    pub fn get_repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Get access to the repository if it exists
    #[must_use]
    pub const fn get_repository(&self) -> Option<&Repository> {
        self.repository.as_ref()
    }

    /// List all pack files in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading the objects/pack directory
    pub fn list_pack_files(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            Repository::list_pack_files,
        )
    }

    /// List all head refs (local branches) in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading the refs/heads directory
    pub fn list_head_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            Repository::list_head_refs,
        )
    }

    /// List all remote refs in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading the refs/remotes directory
    pub fn list_remote_refs(&self) -> Result<Vec<(String, Vec<PathBuf>)>, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            Repository::list_remote_refs,
        )
    }

    /// List all tag refs in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading the refs/tags directory
    pub fn list_tag_refs(&self) -> Result<Vec<PathBuf>, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            Repository::list_tag_refs,
        )
    }

    /// Check if the repository has stash refs
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when checking for stash refs
    pub fn has_stash_ref(&self) -> Result<bool, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            Repository::has_stash_ref,
        )
    }

    /// List loose objects in the repository with a limit
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading loose object directories
    pub fn list_loose_objects(&self, limit: usize) -> Result<Vec<PathBuf>, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            |repo| repo.list_loose_objects(limit),
        )
    }

    /// List parsed loose objects in the repository with a limit
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading loose object directories
    /// - Loose objects cannot be parsed or decompressed
    pub fn list_parsed_loose_objects(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::git::loose_object::LooseObject>, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            |repo| repo.list_parsed_loose_objects(limit),
        )
    }

    /// Get statistics about loose objects in the repository
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The path is not a valid git repository
    /// - File system operations fail when reading loose object directories
    /// - Loose objects cannot be parsed or analyzed
    pub fn get_loose_object_stats(
        &self,
    ) -> Result<crate::git::repository::LooseObjectStats, RepositoryError> {
        self.repository.as_ref().map_or_else(
            || {
                Err(RepositoryError::NotGitRepository(format!(
                    "{} is not a git repository",
                    self.repo_path.display()
                )))
            },
            Repository::get_loose_object_stats,
        )
    }

    /// Parse a pack file (basic analysis)
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The pack file cannot be read
    /// - The pack file format is invalid
    /// - Parsing operations fail
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

    /// Parse a pack file with detailed formatting for display
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The pack file cannot be read
    /// - The pack file format is invalid
    /// - Parsing or display formatting operations fail
    pub fn parse_pack_file_rich(&self, path: &Path) -> Result<(), String> {
        use crate::cli::formatters::CliPackFormatter;

        // Read the pack file
        let pack_data = std::fs::read(path).map_err(|e| format!("Error reading file: {e}"))?;

        // Parse the pack file header
        match crate::git::pack::Header::parse(&pack_data) {
            Ok((mut remaining_data, header)) => {
                let mut objects = Vec::new();

                // Parse all objects
                for _i in 0..header.object_count {
                    match crate::git::pack::Object::parse(remaining_data) {
                        Ok((new_remaining_data, object)) => {
                            objects.push(object);
                            remaining_data = new_remaining_data;
                        }
                        Err(e) => {
                            return Err(format!("Error parsing object: {e}"));
                        }
                    }
                }

                // Format and display the rich output
                let formatted_output = CliPackFormatter::format_pack_file(&header, &objects);
                print!("{formatted_output}");

                Ok(())
            }
            Err(e) => Err(format!("Error parsing pack file: {e}")),
        }
    }
}
