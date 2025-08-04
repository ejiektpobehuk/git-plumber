use crate::tui::model::GitObject;

/// Build "Packs" category using GitPlumber discovery
pub fn build_packs_category(plumber: &crate::GitPlumber) -> Result<GitObject, String> {
    let mut packs_category = GitObject::new_category("Packs");
    match plumber.list_pack_files() {
        Ok(pack_files) => {
            for pack_path in pack_files {
                packs_category.add_child(GitObject::new_pack(pack_path));
            }
            Ok(packs_category)
        }
        Err(e) => Err(format!("Error loading pack files: {e}")),
    }
}

/// Build "Refs" category with Heads, Remotes, Tags (and stash if present)
pub fn build_refs_category(plumber: &crate::GitPlumber) -> Result<GitObject, String> {
    let mut refs_category = GitObject::new_category("Refs");

    let mut heads_category = GitObject::new_category("Heads");
    let mut remotes_category = GitObject::new_category("Remotes");
    let mut tags_category = GitObject::new_category("Tags");

    // Heads
    match plumber.list_head_refs() {
        Ok(head_refs) => {
            for path in head_refs {
                heads_category.add_child(GitObject::new_ref(path));
            }
        }
        Err(e) => return Err(format!("Error loading head refs: {e}")),
    }

    // Remotes
    match plumber.list_remote_refs() {
        Ok(remote_refs) => {
            for (remote_name, refs) in remote_refs {
                let mut remote_category = GitObject::new_category(&remote_name);
                for path in refs {
                    remote_category.add_child(GitObject::new_ref(path));
                }
                remotes_category.add_child(remote_category);
            }
        }
        Err(e) => return Err(format!("Error loading remote refs: {e}")),
    }

    // Tags
    match plumber.list_tag_refs() {
        Ok(tag_refs) => {
            for path in tag_refs {
                tags_category.add_child(GitObject::new_ref(path));
            }
        }
        Err(e) => return Err(format!("Error loading tag refs: {e}")),
    }

    // Stash
    match plumber.has_stash_ref() {
        Ok(true) => refs_category.add_child(GitObject::new_ref(
            plumber.get_repo_path().join(".git/refs/stash"),
        )),
        Ok(false) => {}
        Err(e) => return Err(format!("Error checking stash ref: {e}")),
    }

    refs_category.add_child(heads_category);
    refs_category.add_child(remotes_category);
    refs_category.add_child(tags_category);

    Ok(refs_category)
}

/// Build "objects" category with parsed loose objects
pub fn build_loose_category(plumber: &crate::GitPlumber) -> Result<GitObject, String> {
    let mut loose_objects_category = GitObject::new_category("objects");

    match plumber.get_loose_object_stats() {
        Ok(stats) => match plumber.list_parsed_loose_objects(stats.total_count) {
            Ok(loose_objects) => {
                for parsed_obj in loose_objects {
                    loose_objects_category
                        .add_child(GitObject::new_parsed_loose_object(parsed_obj));
                }
                Ok(loose_objects_category)
            }
            Err(e) => Err(format!("Error loading loose objects: {e}")),
        },
        Err(e) => Err(format!("Error getting loose object stats: {e}")),
    }
}
