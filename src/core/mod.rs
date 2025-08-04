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
                crate::cli::safe_println(&format!("Pack version: {}", header.version))?;
                crate::cli::safe_println(&format!("Number of objects: {}", header.object_count))?;
                let mut remaining_data = objects_data;
                for i in 0..header.object_count {
                    match crate::git::pack::Object::parse(remaining_data) {
                        Ok((new_remaining_data, object)) => {
                            crate::cli::safe_println(&format!("{object}"))?;
                            remaining_data = new_remaining_data;
                        }
                        Err(e) => {
                            return Err(format!("Error parsing object: {e}"));
                        }
                    }
                    if i < header.object_count - 1 {
                        crate::cli::safe_println("--------------------------------")?;
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
                crate::cli::safe_print(&formatted_output)?;

                Ok(())
            }
            Err(e) => Err(format!("Error parsing pack file: {e}")),
        }
    }

    /// View a file as a loose object with rich formatting
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file cannot be read or parsed as a loose object
    /// - The formatting operations fail
    pub fn view_file_as_object(&self, path: &Path) -> Result<(), String> {
        use crate::cli::formatters::CliLooseFormatter;

        self.repository.as_ref().map_or_else(
            || {
                Err(format!(
                    "Not a git repository: {}",
                    self.repo_path.display()
                ))
            },
            |repo| match repo.read_loose_object(path) {
                Ok(loose_obj) => {
                    let formatted_output = CliLooseFormatter::format_loose_object(&loose_obj);
                    crate::cli::safe_print(&formatted_output)?;
                    Ok(())
                }
                Err(e) => Err(format!("Error reading loose object: {e}")),
            },
        )
    }

    /// View an object by hash with rich formatting
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The object cannot be found by hash
    /// - Multiple objects match a partial hash (disambiguation needed)
    /// - The formatting operations fail
    pub fn view_object_by_hash(&self, hash: &str) -> Result<(), String> {
        use crate::cli::formatters::{CliLooseFormatter, CliPackFormatter};
        use std::fmt::Write;

        match self.repository.as_ref() {
            Some(_repo) => {
                // First try as loose object
                if let Ok(loose_obj) = self.find_loose_object_by_partial_hash(hash) {
                    let formatted_output = CliLooseFormatter::format_loose_object(&loose_obj);
                    crate::cli::safe_print(&formatted_output)?;
                    return Ok(());
                }

                // If not found in loose objects, search pack files
                if let Ok(pack_obj) = self.find_pack_object_by_partial_hash(hash) {
                    // Format pack object using existing formatter - create single object "pack file"
                    if let Some(ref object_data) = pack_obj.object_data {
                        let mut output = String::new();
                        writeln!(&mut output, "\x1b[1mPACK OBJECT (found by hash)\x1b[0m").unwrap();
                        writeln!(&mut output, "{}", "─".repeat(50)).unwrap();
                        writeln!(&mut output).unwrap();

                        // Create a PackObject from the found object and format it
                        let formatted_pack_obj = crate::tui::model::PackObject {
                            index: pack_obj.index,
                            obj_type: pack_obj.obj_type.clone(),
                            size: pack_obj.size,
                            sha1: pack_obj.sha1.clone(),
                            base_info: pack_obj.base_info.clone(),
                            object_data: Some(object_data.clone()),
                        };

                        let mut widget =
                            crate::tui::widget::pack_obj_details::PackObjectWidget::new(
                                formatted_pack_obj,
                            );
                        let formatted_text = widget.text();

                        // Convert ratatui Text to ANSI colored string (reuse formatter logic)
                        let colored_text = CliPackFormatter::text_to_ansi_string(&formatted_text);
                        output.push_str(&colored_text);

                        crate::cli::safe_print(&output)?;
                    } else {
                        // Fallback to basic info if no object data
                        crate::cli::safe_println("Pack Object (found by hash):")?;
                        crate::cli::safe_println(&format!(
                            "SHA1: {}",
                            pack_obj.sha1.as_deref().unwrap_or("unknown")
                        ))?;
                        crate::cli::safe_println(&format!("Type: {}", pack_obj.obj_type))?;
                        crate::cli::safe_println(&format!("Size: {} bytes", pack_obj.size))?;
                    }
                    return Ok(());
                }

                Err(format!("Object not found: {hash}"))
            }
            None => Err(format!(
                "Not a git repository: {}",
                self.repo_path.display()
            )),
        }
    }

    /// Find a loose object by partial hash (4-40 characters)
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - No matching objects are found
    /// - Multiple objects match the partial hash
    fn find_loose_object_by_partial_hash(
        &self,
        partial_hash: &str,
    ) -> Result<crate::git::loose_object::LooseObject, String> {
        // For full hash (40 chars), use direct lookup
        if partial_hash.len() == 40 {
            return self
                .repository
                .as_ref()
                .expect("Repository should be available for hash lookup")
                .read_loose_object_by_hash(partial_hash)
                .map_err(|e| format!("Object not found: {e}"));
        }

        // For partial hash, we need to search all loose objects
        match self.list_parsed_loose_objects(10000) {
            // Large limit for comprehensive search
            Ok(objects) => {
                let matches: Vec<_> = objects
                    .into_iter()
                    .filter(|obj| obj.object_id.starts_with(partial_hash))
                    .collect();

                match matches.len() {
                    0 => Err(format!("No loose objects found matching: {partial_hash}")),
                    1 => Ok(matches
                        .into_iter()
                        .next()
                        .expect("Should have exactly one match")),
                    _ => {
                        let mut error_msg = format!("Multiple objects match '{partial_hash}':\n");
                        for obj in matches {
                            use std::fmt::Write;
                            writeln!(&mut error_msg, "  {} ({})", obj.object_id, obj.object_type)
                                .expect("Writing to string should not fail");
                        }
                        Err(error_msg)
                    }
                }
            }
            Err(e) => Err(format!("Error searching loose objects: {e}")),
        }
    }

    /// Find a pack object by partial hash (4-40 characters)
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - No matching objects are found
    /// - Multiple objects match the partial hash
    /// - Pack files cannot be read or parsed
    fn find_pack_object_by_partial_hash(
        &self,
        partial_hash: &str,
    ) -> Result<crate::tui::model::PackObject, String> {
        use crate::git::pack::{Header, Object};
        use sha1::Digest;

        // Get all pack files
        let pack_files = self
            .list_pack_files()
            .map_err(|e| format!("Error listing pack files: {e}"))?;

        let mut matches = Vec::new();

        // Search through each pack file
        for pack_path in pack_files {
            let pack_data =
                std::fs::read(&pack_path).map_err(|e| format!("Error reading pack file: {e}"))?;

            if let Ok((mut remaining_data, header)) = Header::parse(&pack_data) {
                // Parse all objects in this pack file
                for index in 0..header.object_count {
                    if let Ok((new_remaining_data, object)) = Object::parse(remaining_data) {
                        // Calculate SHA-1 hash for this object
                        let obj_type = object.header.obj_type();
                        let size = object.header.uncompressed_data_size();
                        let mut hasher = sha1::Sha1::new();
                        let header_str = format!("{obj_type} {size}\0");
                        hasher.update(header_str.as_bytes());
                        hasher.update(&object.uncompressed_data);
                        let sha1 = format!("{:x}", hasher.finalize());

                        // Check if this hash matches our partial hash
                        if sha1.starts_with(partial_hash) {
                            let pack_obj = crate::tui::model::PackObject {
                                index: index as usize + 1,
                                obj_type: obj_type.to_string(),
                                size: u32::try_from(size).unwrap_or(u32::MAX),
                                sha1: Some(sha1),
                                base_info: None, // TODO: Add delta info if needed
                                object_data: Some(object),
                            };
                            matches.push(pack_obj);
                        }

                        remaining_data = new_remaining_data;
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(format!("No pack objects found matching: {partial_hash}")),
            1 => Ok(matches
                .into_iter()
                .next()
                .expect("Should have exactly one match")),
            _ => {
                let mut error_msg = format!("Multiple pack objects match '{partial_hash}':\n");
                for obj in matches {
                    use std::fmt::Write;
                    writeln!(
                        &mut error_msg,
                        "  {} ({})",
                        obj.sha1.as_deref().unwrap_or("unknown"),
                        obj.obj_type
                    )
                    .expect("Writing to string should not fail");
                }
                Err(error_msg)
            }
        }
    }
}
