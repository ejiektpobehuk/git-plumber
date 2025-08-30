use crate::tui::model::{GitObject, GitObjectType};

/// Build the complete .git directory file tree structure
pub fn build_git_file_tree(plumber: &crate::GitPlumber) -> Result<Vec<GitObject>, String> {
    let git_path = plumber.get_repo_path().join(".git");
    let mut git_contents = Vec::new();

    // Build objects directory with pack and loose objects
    if let Ok(objects_folder) = build_objects_folder(plumber) {
        git_contents.push(objects_folder);
    }

    // Build refs directory
    if let Ok(refs_folder) = build_refs_folder(plumber) {
        git_contents.push(refs_folder);
    }

    // Scan for all other .git directories/files (not just the common ones)
    match std::fs::read_dir(&git_path) {
        Ok(entries) => {
            let mut items: Vec<(String, std::path::PathBuf, bool)> = Vec::new();

            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry_path.is_dir();

                // Skip objects and refs as we handle them specially
                if name != "objects" && name != "refs" {
                    items.push((name, entry_path, is_dir));
                }
            }

            // Sort: directories first, then files, both alphabetically
            items.sort_by(|a, b| {
                match (a.2, b.2) {
                    (true, false) => std::cmp::Ordering::Less, // dirs before files
                    (false, true) => std::cmp::Ordering::Greater, // files after dirs
                    _ => a.0.cmp(&b.0),                        // alphabetical within same type
                }
            });

            // Add all found items - use full tree loading for all filesystem folders
            for (_, entry_path, is_dir) in items {
                if is_dir {
                    git_contents.push(build_full_filesystem_folder(entry_path, false)?);
                } else {
                    git_contents.push(GitObject::new_filesystem_file(entry_path));
                }
            }
        }
        Err(e) => return Err(format!("Error reading .git directory: {e}")),
    }

    Ok(git_contents)
}

/// Build the objects directory with pack and loose object folders
fn build_objects_folder(plumber: &crate::GitPlumber) -> Result<GitObject, String> {
    let objects_path = plumber.get_repo_path().join(".git/objects");
    let mut objects_folder = GitObject::new_filesystem_folder(objects_path.clone(), true);

    // Make educational folders start expanded
    objects_folder.expanded = true;

    // Add pack folder with educational content
    let pack_path = objects_path.join("pack");
    if pack_path.exists() {
        let mut pack_folder = GitObject::new_filesystem_folder(pack_path, true);
        pack_folder.expanded = true; // Educational folders start expanded

        // Load pack groups (each pack as a folder with its associated files)
        match plumber.list_pack_groups() {
            Ok(pack_groups) => {
                for (_, pack_group) in pack_groups {
                    if pack_group.is_valid() {
                        pack_folder.add_child(GitObject::new_pack_folder(pack_group));
                    }
                }
            }
            Err(e) => return Err(format!("Error loading pack groups: {e}")),
        }
        // Mark pack folder as loaded since we populated it with pack files
        if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut pack_folder.obj_type {
            *is_loaded = true;
        }
        objects_folder.add_child(pack_folder);
    }

    // Add info folder if it exists (use full loading)
    let info_path = objects_path.join("info");
    if info_path.exists() {
        objects_folder.add_child(build_full_filesystem_folder(info_path, false)?);
    }

    // Create a special folder for loose objects with educational content
    // Always show "Loose Objects" folder for educational purposes, even when empty
    let mut loose_objects_folder = GitObject::new_category("Loose Objects");

    match plumber.get_loose_object_stats() {
        Ok(stats) => match plumber.list_parsed_loose_objects(stats.total_count) {
            Ok(loose_objects) => {
                // Set expansion state based on number of objects:
                // - Expand if 10 or fewer objects (easier to see small collections)
                // - Collapse if more than 10 objects (avoid overwhelming UI)
                let object_count = loose_objects.len();
                loose_objects_folder.expanded = object_count <= 10;

                // Add loose objects if they exist
                for parsed_obj in loose_objects {
                    loose_objects_folder.add_child(GitObject::new_parsed_loose_object(parsed_obj));
                }
            }
            Err(e) => return Err(format!("Error loading loose objects: {e}")),
        },
        Err(e) => return Err(format!("Error getting loose object stats: {e}")),
    }

    // Always add the loose objects folder to maintain static presence
    objects_folder.add_child(loose_objects_folder);

    // Mark objects folder as loaded since we populated it
    if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut objects_folder.obj_type {
        *is_loaded = true;
    }

    Ok(objects_folder)
}

/// Build the refs directory with all reference subcategories
fn build_refs_folder(plumber: &crate::GitPlumber) -> Result<GitObject, String> {
    let refs_path = plumber.get_repo_path().join(".git/refs");
    let mut refs_folder = GitObject::new_filesystem_folder(refs_path, true);

    // Make educational folders start expanded
    refs_folder.expanded = true;

    // Add heads folder with educational content
    let heads_path = plumber.get_repo_path().join(".git/refs/heads");
    if heads_path.exists() {
        let mut heads_folder = GitObject::new_filesystem_folder(heads_path, true);
        heads_folder.expanded = true; // Educational folders start expanded

        match plumber.list_head_refs() {
            Ok(head_refs) => {
                for path in head_refs {
                    heads_folder.add_child(GitObject::new_ref(path));
                }
            }
            Err(e) => return Err(format!("Error loading head refs: {e}")),
        }
        // Mark as loaded since we populated it
        if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut heads_folder.obj_type {
            *is_loaded = true;
        }
        refs_folder.add_child(heads_folder);
    }

    // Add remotes folder with educational content
    let remotes_path = plumber.get_repo_path().join(".git/refs/remotes");
    if remotes_path.exists() {
        let mut remotes_folder = GitObject::new_filesystem_folder(remotes_path, true);
        remotes_folder.expanded = true; // Educational folders start expanded

        match plumber.list_remote_refs() {
            Ok(remote_refs) => {
                for (remote_name, refs) in remote_refs {
                    let mut remote_category = GitObject::new_category(&remote_name);
                    for path in refs {
                        remote_category.add_child(GitObject::new_ref(path));
                    }
                    remotes_folder.add_child(remote_category);
                }
            }
            Err(e) => return Err(format!("Error loading remote refs: {e}")),
        }
        // Mark as loaded since we populated it
        if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut remotes_folder.obj_type {
            *is_loaded = true;
        }
        refs_folder.add_child(remotes_folder);
    }

    // Add tags folder with educational content
    let tags_path = plumber.get_repo_path().join(".git/refs/tags");
    if tags_path.exists() {
        let mut tags_folder = GitObject::new_filesystem_folder(tags_path, true);
        tags_folder.expanded = true; // Educational folders start expanded

        match plumber.list_tag_refs() {
            Ok(tag_refs) => {
                for path in tag_refs {
                    tags_folder.add_child(GitObject::new_ref(path));
                }
            }
            Err(e) => return Err(format!("Error loading tag refs: {e}")),
        }
        // Mark as loaded since we populated it
        if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut tags_folder.obj_type {
            *is_loaded = true;
        }
        refs_folder.add_child(tags_folder);
    }

    // Add stash file if it exists
    match plumber.has_stash_ref() {
        Ok(true) => {
            let stash_path = plumber.get_repo_path().join(".git/refs/stash");
            refs_folder.add_child(GitObject::new_ref(stash_path));
        }
        Ok(false) => {}
        Err(e) => return Err(format!("Error checking stash ref: {e}")),
    }

    // Mark refs folder as loaded since we populated it
    if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut refs_folder.obj_type {
        *is_loaded = true;
    }

    Ok(refs_folder)
}

/// Build a complete filesystem folder with all contents loaded recursively
/// This replaces the lazy loading approach with a full tree that includes all files and modification times
fn build_full_filesystem_folder(
    path: std::path::PathBuf,
    is_educational: bool,
) -> Result<GitObject, String> {
    let mut folder = GitObject::new_filesystem_folder(path, is_educational);

    // Always start collapsed for non-educational folders (user can expand as needed)
    folder.expanded = is_educational;

    // Load all contents immediately
    load_folder_contents_recursively(&mut folder)?;

    // Mark as loaded since we populated it
    if let GitObjectType::FileSystemFolder { is_loaded, .. } = &mut folder.obj_type {
        *is_loaded = true;
    }

    Ok(folder)
}

/// Recursively load all contents of a filesystem folder
fn load_folder_contents_recursively(folder: &mut GitObject) -> Result<(), String> {
    let path = match &folder.obj_type {
        GitObjectType::FileSystemFolder { path, .. } => path.clone(),
        _ => return Err("Not a filesystem folder".to_string()),
    };

    // Read directory contents
    match std::fs::read_dir(&path) {
        Ok(entries) => {
            let mut items: Vec<(String, std::path::PathBuf, bool)> = Vec::new();

            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry_path.is_dir();
                items.push((name, entry_path, is_dir));
            }

            // Sort: directories first, then files, both alphabetically
            items.sort_by(|a, b| {
                match (a.2, b.2) {
                    (true, false) => std::cmp::Ordering::Less, // dirs before files
                    (false, true) => std::cmp::Ordering::Greater, // files after dirs
                    _ => a.0.cmp(&b.0),                        // alphabetical within same type
                }
            });

            // Create child objects and recursively load subdirectories
            for (_, entry_path, is_dir) in items {
                if is_dir {
                    // Recursively build subdirectories with full contents
                    let mut subfolder = GitObject::new_filesystem_folder(entry_path, false);
                    subfolder.expanded = false; // Start collapsed

                    // Load subfolder contents
                    load_folder_contents_recursively(&mut subfolder)?;

                    // Mark as loaded
                    if let GitObjectType::FileSystemFolder { is_loaded, .. } =
                        &mut subfolder.obj_type
                    {
                        *is_loaded = true;
                    }

                    folder.children.push(subfolder);
                } else {
                    folder
                        .children
                        .push(GitObject::new_filesystem_file(entry_path));
                }
            }

            Ok(())
        }
        Err(e) => Err(format!("Error reading directory {}: {}", path.display(), e)),
    }
}
