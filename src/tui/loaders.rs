use crate::tui::main_view::PreviewState;
use crate::tui::message::Message;
use crate::tui::model::{AppState, AppView, GitObject, GitObjectType, PackObject};
use std::path::PathBuf;

use super::main_view::{MainViewState, PackPreViewState};
use rayon::prelude::*;
use sha1::{Digest, Sha1};

impl AppState {
    // Load Git objects from the repository
    pub fn load_git_objects(&mut self, plumber: &crate::GitPlumber) -> Message {
        match &mut self.view {
            AppView::Main { state } => {
                // Pre-build pipeline: snapshot old state for change detection
                let (old_positions, old_nodes) = if state.has_loaded_once {
                    state.snapshot_old_positions()
                } else {
                    (
                        std::collections::HashMap::new(),
                        std::collections::HashMap::new(),
                    )
                };
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

                // Load loose objects with parsed content
                match plumber.get_loose_object_stats() {
                    Ok(stats) => {
                        // Use total_count to load all loose objects
                        match plumber.list_parsed_loose_objects(stats.total_count) {
                            Ok(loose_objects) => {
                                for parsed_obj in loose_objects {
                                    loose_objects_category
                                        .add_child(GitObject::new_parsed_loose_object(parsed_obj));
                                }
                            }
                            Err(e) => {
                                return Message::LoadGitObjects(Err(format!(
                                    "Error loading loose objects: {e}"
                                )));
                            }
                        }
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error getting loose object stats: {e}"
                        )));
                    }
                }
                state.git_objects.list.push(loose_objects_category);

                // Sort lists for display consistency (natural sort for categories except "objects")
                MainViewState::sort_tree_for_display(&mut state.git_objects.list);

                // If this is not the first successful load, detect tree changes to drive UI effects
                if state.has_loaded_once {
                    let _ = state.detect_tree_changes(&old_positions, &old_nodes);
                }

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
                    let (_, obj, _) =
                        &state.git_objects.flat_view[state.git_objects.selected_index];
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
                            size,
                            object_id,
                            parsed_object,
                            ..
                    // tuple now contains (depth, object, status)

                        } => {
                            // Use cached loose object data if available, otherwise show basic info
                            let size_str = match size {
                                Some(size) => format!("{size} bytes"),
                                None => "Unknown size".to_string(),
                            };
                            let obj_id = object_id.as_deref().unwrap_or("Unknown object ID");

                            let detailed_info = if let Some(parsed_obj) = parsed_object {
                                format!(
                                    "Type: {} (Loose Object)\nObject ID: {}\nSize: {}\n\n{}",
                                    parsed_obj.object_type,
                                    obj_id,
                                    size_str,
                                    Self::format_parsed_object_details(parsed_obj)
                                )
                            } else {
                                format!("Type: Loose Object\nObject ID: {obj_id}\nSize: {size_str}")
                            };

                            Message::LoadGitObjectInfo(Ok(detailed_info))
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
                    let (_, obj, _) =
                        &state.git_objects.flat_view[state.git_objects.selected_index];
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
                        GitObjectType::LooseObject {
                            object_id,
                            parsed_object,
                            ..
                        } => {
                            // Generate custom preview showing the compressed file structure
                            if let Some(parsed_obj) = parsed_object {
                                let preview = Self::generate_loose_object_preview(parsed_obj);
                                Message::LoadEducationalContent(Ok(preview))
                            } else {
                                // Fallback to generic preview if no parsed object
                                let obj_id = object_id.as_deref().unwrap_or("Unknown");
                                let preview = self
                                    .educational_content_provider
                                    .get_loose_object_preview(obj_id);
                                Message::LoadEducationalContent(Ok(preview))
                            }
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
                        preview_state: PreviewState::Pack(PackPreViewState { pack_file_path, .. }),
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
                                return Message::LoadPackObjects {
                                    path: pack_path.clone(),
                                    result: Err(format!("Error parsing pack header: {e:?}")),
                                };
                            }
                        }

                        // Parallel SHA-1 calculation
                        let objects: Vec<PackObject> = parsed_objects
                            .into_par_iter()
                            .map(|(index, object, base_info)| {
                                let obj_type = object.header.obj_type();
                                let size = object.header.uncompressed_data_size();
                                let mut hasher = Sha1::new();
                                let header = format!("{obj_type} {size}\0");
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

                        Message::LoadPackObjects {
                            path: pack_path.clone(),
                            result: Ok(objects),
                        }
                    }
                    Err(e) => Message::LoadPackObjects {
                        path: pack_path.clone(),
                        result: Err(format!("Error reading pack file: {e}")),
                    },
                }
            }
            _ => Message::LoadPackObjects {
                path: pack_path.clone(),
                result: Err("Sent to wrong View".to_string()),
            },
        }
    }

    /// Format detailed information for parsed loose objects
    fn format_parsed_object_details(parsed_obj: &crate::git::loose_object::LooseObject) -> String {
        use crate::git::loose_object::ParsedContent;

        match parsed_obj.get_parsed_content() {
            Some(ParsedContent::Commit(commit)) => {
                format!(
                    "Commit Details:\n  Tree: {}\n  Parents: {}\n  Author: {}\n  Committer: {}\n  Message: \"{}\"",
                    commit.tree,
                    if commit.parents.is_empty() {
                        "none (root commit)".to_string()
                    } else {
                        commit.parents.join(", ")
                    },
                    commit.author,
                    commit.committer,
                    commit.message.lines().next().unwrap_or("").trim()
                )
            }
            Some(ParsedContent::Tree(tree)) => {
                let mut details = format!("Tree Details:\n  {} entries:\n", tree.entries.len());
                for (i, entry) in tree.entries.iter().enumerate() {
                    if i >= 5 {
                        // Limit to first 5 entries
                        details.push_str(&format!(
                            "  ... and {} more entries",
                            tree.entries.len() - 5
                        ));
                        break;
                    }
                    details.push_str(&format!(
                        "    {} {} {}\n",
                        entry.mode,
                        &entry.sha1[..8],
                        entry.name
                    ));
                }
                details
            }
            Some(ParsedContent::Blob(content)) => {
                let content_preview = if parsed_obj.is_binary() {
                    format!("Binary content ({} bytes)", content.len())
                } else {
                    let content_str = String::from_utf8_lossy(content);
                    let preview = if content_str.len() > 200 {
                        format!("{}...", &content_str[..200])
                    } else {
                        content_str.to_string()
                    };
                    format!("Text content preview:\n{preview}")
                };
                format!("Blob Details:\n  {content_preview}")
            }
            Some(ParsedContent::Tag(tag)) => {
                format!(
                    "Tag Details:\n  Tag: {}\n  Object: {} ({})\n  Tagger: {}\n  Message: \"{}\"",
                    tag.tag,
                    tag.object,
                    tag.object_type,
                    tag.tagger.as_deref().unwrap_or("unknown"),
                    tag.message.lines().next().unwrap_or("").trim()
                )
            }
            None => "Failed to parse object content".to_string(),
        }
    }

    /// Generate a preview showing the loose object's compressed file structure
    fn generate_loose_object_preview(
        parsed_obj: &crate::git::loose_object::LooseObject,
    ) -> ratatui::text::Text<'static> {
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Text};

        let mut lines = vec![
            // Explain the file structure
            Line::from("Git stores loose objects as compressed files with this structure:"),
            Line::from(""),
            Line::from("┌──────────────────────────────────┐"),
            Line::from("│ Compressed File (zlib format)    │"),
            Line::from("│ ┌──────────────────────────────┐ │"),
            Line::from("│ │ Header: \"<type> <size>\\0\"    │ │"),
            Line::from("│ │ Content: <actual object data>│ │"),
            Line::from("│ └──────────────────────────────┘ │"),
            Line::from("└──────────────────────────────────┘"),
            Line::from(""),
            // Show the actual header for this object
            Line::styled(
                "HEADER CONTENT",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Line::from("─".repeat(20)),
            Line::from(""),
        ];

        let header_text = format!("{} {}\\0", parsed_obj.object_type, parsed_obj.size);
        lines.push(Line::from(format!("Header: \"{header_text}\"")));
        lines.push(Line::from(format!("  - Type: {}", parsed_obj.object_type)));
        lines.push(Line::from(format!("  - Size: {} bytes", parsed_obj.size)));
        lines.push(Line::from("  - Null terminator: \\0"));
        lines.push(Line::from(""));

        // Show content preview
        lines.push(Line::styled(
            "CONTENT PREVIEW",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(20)));
        lines.push(Line::from(""));

        // Type-specific preview
        match parsed_obj.object_type {
            crate::git::loose_object::LooseObjectType::Commit => {
                lines.push(Line::from("Content type: Commit metadata"));
                if let Some(crate::git::loose_object::ParsedContent::Commit(commit)) =
                    parsed_obj.get_parsed_content()
                {
                    lines.push(Line::from(""));
                    lines.push(Line::from("Sample content:"));
                    lines.push(Line::from(format!("  tree {}", commit.tree)));
                    for parent in &commit.parents {
                        lines.push(Line::from(format!("  parent {parent}")));
                    }
                    lines.push(Line::from(format!("  author {}", commit.author)));
                    lines.push(Line::from(format!("  committer {}", commit.committer)));
                    lines.push(Line::from(""));
                    let message_preview = commit.message.lines().next().unwrap_or("").trim();
                    lines.push(Line::from(format!("  {message_preview}")));
                    if commit.message.lines().count() > 1 {
                        lines.push(Line::from("  ..."));
                    }
                }
            }
            crate::git::loose_object::LooseObjectType::Tree => {
                lines.push(Line::from("Content type: Directory listing"));
                if let Some(crate::git::loose_object::ParsedContent::Tree(tree)) =
                    parsed_obj.get_parsed_content()
                {
                    lines.push(Line::from(""));
                    lines.push(Line::from("Sample content (binary format):"));
                    lines.push(Line::from(format!(
                        "  {} entries total",
                        tree.entries.len()
                    )));
                    for (i, entry) in tree.entries.iter().enumerate() {
                        if i >= 3 {
                            // Show first 3 entries
                            lines.push(Line::from("  ..."));
                            break;
                        }
                        lines.push(Line::from(format!(
                            "  {} {} <20-byte-sha1>",
                            entry.mode, entry.name
                        )));
                    }
                }
            }
            crate::git::loose_object::LooseObjectType::Blob => {
                lines.push(Line::from("Content type: File content"));
                if let Some(crate::git::loose_object::ParsedContent::Blob(content)) =
                    parsed_obj.get_parsed_content()
                {
                    lines.push(Line::from(""));
                    if parsed_obj.is_binary() {
                        lines.push(Line::from("Binary content preview:"));
                        let preview_size = content.len().min(32);
                        let hex_preview: String = content[..preview_size]
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        lines.push(Line::from(format!(
                            "  {} {}",
                            hex_preview,
                            if content.len() > 32 { "..." } else { "" }
                        )));
                    } else {
                        lines.push(Line::from("Text content preview:"));
                        let content_str = String::from_utf8_lossy(content);
                        let preview = if content_str.len() > 100 {
                            format!("{}...", &content_str[..100])
                        } else {
                            content_str.to_string()
                        };
                        // Show first few lines
                        for (i, line) in preview.lines().enumerate() {
                            if i >= 5 {
                                // Show first 5 lines
                                lines.push(Line::from("  ..."));
                                break;
                            }
                            lines.push(Line::from(format!("  {line}")));
                        }
                    }
                }
            }
            crate::git::loose_object::LooseObjectType::Tag => {
                lines.push(Line::from("Content type: Tag metadata"));
                if let Some(crate::git::loose_object::ParsedContent::Tag(tag)) =
                    parsed_obj.get_parsed_content()
                {
                    lines.push(Line::from(""));
                    lines.push(Line::from("Sample content:"));
                    lines.push(Line::from(format!("  object {}", tag.object)));
                    lines.push(Line::from(format!("  type {}", tag.object_type)));
                    lines.push(Line::from(format!("  tag {}", tag.tag)));
                    if let Some(ref tagger) = tag.tagger {
                        lines.push(Line::from(format!("  tagger {tagger}")));
                    }
                    lines.push(Line::from(""));
                    let message_preview = tag.message.lines().next().unwrap_or("").trim();
                    lines.push(Line::from(format!("  {message_preview}")));
                    if tag.message.lines().count() > 1 {
                        lines.push(Line::from("  ..."));
                    }
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::styled(
            "COMPRESSION INFO",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(20)));
        lines.push(Line::from(""));
        lines.push(Line::from("• File is compressed using zlib (RFC 1950)"));
        lines.push(Line::from("• Decompressed size matches the size in header"));
        lines.push(Line::from("• Git uses this format for all loose objects"));
        lines.push(Line::from(
            "• Object ID is SHA-1 hash of the decompressed content",
        ));

        Text::from(lines)
    }
}
