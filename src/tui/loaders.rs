use crate::tui::main_view::PreviewState;
use crate::tui::message::Message;
use crate::tui::model::{AppState, AppView, GitObject, GitObjectType, PackObject};
use std::path::PathBuf;

use super::main_view::{MainViewState, PackPreViewState};
use super::pack_details::PackViewState;
use rayon::prelude::*;
use sha1::{Digest, Sha1};

impl AppState {
    // Load Git objects from the repository
    pub fn load_git_objects(&mut self, plumber: &crate::GitPlumber) -> Message {
        match &mut self.view {
            AppView::Main { state } => {
                // Clear existing objects
                state.git_objects.list.clear();

                // Add Pack Files category
                let mut packs_category = GitObject::new_category("Packs");
                match plumber.list_pack_files() {
                    Ok(pack_files) => {
                        for pack_path in pack_files {
                            packs_category.add_child(GitObject::new_pack(pack_path));
                        }
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error loading pack files: {e}"
                        )));
                    }
                }
                state.git_objects.list.push(packs_category);

                // Add Refs category
                let mut refs_category = GitObject::new_category("Refs");

                // Refs subcategories
                let mut heads_category = GitObject::new_category("Heads");
                let mut remotes_category = GitObject::new_category("Remotes");
                let mut tags_category = GitObject::new_category("Tags");

                // Load heads (branches)
                match plumber.list_head_refs() {
                    Ok(head_refs) => {
                        for path in head_refs {
                            heads_category.add_child(GitObject::new_ref(path));
                        }
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error loading head refs: {e}"
                        )));
                    }
                }

                // Load remotes
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
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error loading remote refs: {e}"
                        )));
                    }
                }
                // Load tags
                match plumber.list_tag_refs() {
                    Ok(tag_refs) => {
                        for path in tag_refs {
                            tags_category.add_child(GitObject::new_ref(path));
                        }
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error loading tag refs: {e}"
                        )));
                    }
                }

                // Add stash if it exists
                match plumber.has_stash_ref() {
                    Ok(true) => {
                        refs_category
                            .add_child(GitObject::new_ref(self.repo_path.join(".git/refs/stash")));
                    }
                    Ok(false) => {
                        // No stash ref exists, nothing to add
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error checking stash ref: {e}"
                        )));
                    }
                }

                refs_category.add_child(heads_category);
                refs_category.add_child(remotes_category);
                refs_category.add_child(tags_category);
                state.git_objects.list.push(refs_category);

                // Add Loose Objects category
                let mut loose_objects_category = GitObject::new_category("objects");

                // Load loose objects (just sample a few for demonstration)
                match plumber.list_loose_objects(10) {
                    Ok(loose_objects) => {
                        for path in loose_objects {
                            loose_objects_category.add_child(GitObject::new_loose_object(path));
                        }
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error loading loose objects: {e}"
                        )));
                    }
                }
                state.git_objects.list.push(loose_objects_category);

                // Flatten the tree for display
                state.flatten_tree();

                // Reset selection
                if !state.git_objects.flat_view.is_empty() {
                    state.git_objects.selected_index = 0;
                }

                Message::LoadGitObjects(Ok(()))
            }
            _ => Message::LoadGitObjects(Err("Called for the wrong View or State".to_string())),
        }
    }

    // Load details for the currently selected object
    pub fn load_git_object_details(&self, _plumber: &crate::GitPlumber) -> Message {
        match &self.view {
            AppView::Main { state } => {
                if !state.git_objects.flat_view.is_empty() {
                    let (_, obj) = &state.git_objects.flat_view[state.git_objects.selected_index];
                    match &obj.obj_type {
                        GitObjectType::Pack {
                            size,
                            modified_time,
                            ..
                        } => {
                            // Use cached data for pack file
                            let size_str = match size {
                                Some(size) => {
                                    if *size < 1024 {
                                        format!("{size} bytes")
                                    } else if *size < 1024 * 1024 {
                                        format!("{:.2} KB", *size as f64 / 1024.0)
                                    } else {
                                        format!("{:.2} MB", *size as f64 / (1024.0 * 1024.0))
                                    }
                                }
                                None => "Unknown size".to_string(),
                            };

                            let modified = modified_time
                                .as_ref()
                                .map(crate::tui::model::GitObject::format_time_ago)
                                .unwrap_or_else(|| "Unknown time".to_string());

                            let info = format!(
                                "Type: Pack File\nName: {}\nSize: {}\nLast modified: {}",
                                obj.name, size_str, modified
                            );

                            Message::LoadGitObjectInfo(Ok(info))
                        }
                        GitObjectType::Ref { content, .. } => {
                            // Use cached reference content
                            let ref_content =
                                content.as_deref().unwrap_or("Unable to read reference");
                            let info = format!(
                                "Type: Git Reference\nName: {}\nPoints to: {}",
                                obj.name, ref_content
                            );
                            Message::LoadGitObjectInfo(Ok(info))
                        }
                        GitObjectType::LooseObject {
                            size, object_id, ..
                        } => {
                            // Use cached loose object data
                            let size_str = match size {
                                Some(size) => format!("{size} bytes"),
                                None => "Unknown size".to_string(),
                            };
                            let obj_id = object_id.as_deref().unwrap_or("Unknown object ID");

                            let info = format!(
                                "Type: Loose Object\nObject ID: {}\nSize: {}",
                                obj_id, size_str
                            );
                            Message::LoadGitObjectInfo(Ok(info))
                        }
                        GitObjectType::Category(name) => {
                            // For categories, show number of children
                            let info = format!(
                                "Type: Category\nName: {}\nContains: {} items",
                                name,
                                obj.children.len()
                            );
                            Message::LoadGitObjectInfo(Ok(info))
                        }
                    }
                } else {
                    Message::LoadGitObjectInfo(Ok("No object selected".to_string()))
                }
            }
            _ => Message::LoadGitObjectInfo(Err("Sent to the wrong View".to_string())),
        }
    }

    // Load educational content for the currently selected object
    pub fn load_educational_content(&self, _plumber: &crate::GitPlumber) -> Message {
        match &self.view {
            AppView::Main { state } => {
                if !state.git_objects.flat_view.is_empty() {
                    let (_, obj) = &state.git_objects.flat_view[state.git_objects.selected_index];
                    match &obj.obj_type {
                        GitObjectType::Category(name) => {
                            let content =
                                self.educational_content_provider.get_category_content(name);
                            Message::LoadEducationalContent(Ok(content))
                        }
                        // For actual objects, show previews instead of educational content
                        GitObjectType::Pack { path, .. } => {
                            // Try to parse pack file header for preview
                            match std::fs::read(path) {
                                Ok(pack_data) => {
                                    match crate::git::pack::Header::parse(&pack_data) {
                                        Ok((_, header)) => {
                                            let preview = self
                                                .educational_content_provider
                                                .get_pack_preview(&header);
                                            Message::LoadEducationalContent(Ok(preview))
                                        }
                                        Err(e) => Message::LoadEducationalContent(Err(format!(
                                            "Error parsing pack header: {e:?}"
                                        ))),
                                    }
                                }
                                Err(e) => Message::LoadEducationalContent(Err(format!(
                                    "Error reading file: {e}"
                                ))),
                            }
                        }
                        GitObjectType::Ref { content, .. } => {
                            // Use cached reference content for preview
                            let ref_content = content.as_deref().unwrap_or("");
                            let preview = self
                                .educational_content_provider
                                .get_ref_preview(ref_content);
                            Message::LoadEducationalContent(Ok(preview))
                        }
                        GitObjectType::LooseObject { object_id, .. } => {
                            // Use cached object ID for preview
                            let obj_id = object_id.as_deref().unwrap_or("Unknown");
                            let preview = self
                                .educational_content_provider
                                .get_loose_object_preview(obj_id);
                            Message::LoadEducationalContent(Ok(preview))
                        }
                    }
                } else {
                    let content = self.educational_content_provider.get_default_content();
                    Message::LoadEducationalContent(Ok(content))
                }
            }
            _ => Message::LoadEducationalContent(Err("Sent for the wrong View".to_string())),
        }
    }

    // Load pack objects from a pack file
    pub fn load_pack_objects(&mut self, pack_path: &PathBuf) -> Message {
        match &mut self.view {
            AppView::Main {
                state:
                    MainViewState {
                        preview_state:
                            PreviewState::Pack(PackPreViewState {
                                pack_file_path,
                                pack_object_list,
                                ..
                            }),
                        ..
                    },
            }
            | AppView::PackObjectDetail {
                state:
                    PackViewState {
                        pack_file_path,
                        pack_object_list,
                        ..
                    },
            } => {
                *pack_file_path = pack_path.clone();
                match std::fs::read(pack_path) {
                    Ok(pack_data) => {
                        let mut parsed_objects = Vec::new();

                        match crate::git::pack::Header::parse(&pack_data) {
                            Ok((mut data, _header)) => {
                                let mut object_count = 0;
                                while !data.is_empty() {
                                    match crate::git::pack::Object::parse(data) {
                                        Ok((new_data, object)) => {
                                            let base_info = match &object.header {
                                                crate::git::pack::ObjectHeader::OfsDelta {
                                                    base_offset,
                                                    ..
                                                } => Some(format!("Base offset: {base_offset}")),
                                                crate::git::pack::ObjectHeader::RefDelta {
                                                    base_ref,
                                                    ..
                                                } => Some(format!(
                                                    "Base ref: {}",
                                                    hex::encode(base_ref)
                                                )),
                                                _ => None,
                                            };
                                            object_count += 1;
                                            parsed_objects.push((object_count, object, base_info));
                                            data = new_data;
                                        }
                                        Err(_) => break,
                                    }
                                }
                            }
                            Err(e) => {
                                return Message::LoadPackObjects(Err(format!(
                                    "Error parsing pack header: {e:?}"
                                )));
                            }
                        }

                        // Parallel SHA-1 calculation
                        let objects: Vec<PackObject> = parsed_objects
                            .into_par_iter()
                            .map(|(index, object, base_info)| {
                                let obj_type = object.header.obj_type();
                                let size = object.header.uncompressed_data_size();
                                let mut hasher = Sha1::new();
                                let header = format!("{} {}\0", obj_type, size);
                                hasher.update(header.as_bytes());
                                hasher.update(&object.uncompressed_data);
                                let sha1 = Some(format!("{:x}", hasher.finalize()));
                                PackObject {
                                    index,
                                    obj_type: obj_type.to_string(),
                                    size: size as u32,
                                    sha1,
                                    base_info,
                                    object_data: Some(object),
                                }
                            })
                            .collect();

                        Message::LoadPackObjects(Ok(objects))
                    }
                    Err(e) => {
                        Message::LoadPackObjects(Err(format!("Error reading pack file: {e}")))
                    }
                }
            }
            _ => Message::LoadPackObjects(Err("Sent to wrong View".to_string())),
        }
    }
}
