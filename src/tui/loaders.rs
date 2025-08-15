use crate::tui::main_view::PreviewState;
use crate::tui::message::Message;
use crate::tui::model::{AppState, AppView, GitObjectType, PackObject};
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

                // Preserve old tree for state restoration
                let old_tree = state.git_objects.list.clone();

                // Clear existing objects
                state.git_objects.list.clear();

                // Use the new file tree structure
                match crate::tui::git_tree::build_git_file_tree(plumber) {
                    Ok(mut git_objects) => {
                        // Restore expansion and loading state from old tree if this isn't the first load
                        if state.has_loaded_once {
                            for new_obj in &mut git_objects {
                                new_obj.restore_state_from(&old_tree);
                            }
                        }

                        // Sort the new tree before comparison
                        MainViewState::sort_tree_for_display(&mut git_objects);

                        // Set the new tree
                        state.git_objects.list = git_objects;

                        // NOW detect changes after full restoration and before flattening
                        if state.has_loaded_once {
                            let _ = state.detect_tree_changes(&old_positions, &old_nodes);
                        }
                    }
                    Err(e) => {
                        return Message::LoadGitObjects(Err(format!(
                            "Error building git file tree: {e}"
                        )));
                    }
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
                        GitObjectType::FileSystemFolder { path, is_educational, .. } => {
                            // For filesystem folders, show path and contents
                            let folder_type = if *is_educational {
                                "Educational Folder"
                            } else {
                                "Directory"
                            };
                            let info = format!(
                                "Type: {}\nPath: {}\nContains: {} items",
                                folder_type,
                                path.display(),
                                obj.children.len()
                            );
                            Message::LoadGitObjectInfo(Ok(info))
                        }
                        GitObjectType::FileSystemFile { path, size, modified_time } => {
                            // For filesystem files, show file details
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
                                "Type: File\nPath: {}\nSize: {}\nLast modified: {}",
                                path.display(), size_str, modified
                            );
                            Message::LoadGitObjectInfo(Ok(info))
                        }
                        GitObjectType::PackFolder { base_name, pack_group } => {
                            let mut files = Vec::new();
                            for (file_type, _) in pack_group.get_all_files() {
                                files.push(file_type);
                            }
                            let info = format!(
                                "Type: Pack Group\nBase name: {}\nFiles: {}",
                                base_name, files.join(", ")
                            );
                            Message::LoadGitObjectInfo(Ok(info))
                        }
                        GitObjectType::PackFile { file_type, path, size, modified_time } => {
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
                                "Type: Pack {}\nPath: {}\nSize: {}\nLast modified: {}",
                                file_type, path.display(), size_str, modified
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
                        GitObjectType::FileSystemFolder {
                            path,
                            is_educational,
                            ..
                        } => {
                            if *is_educational {
                                // Map filesystem folders to educational content based on their path
                                let folder_name =
                                    path.file_name().unwrap_or_default().to_string_lossy();
                                let content_key = match folder_name.as_ref() {
                                    "pack" => "Packs",
                                    "refs" => "Refs",
                                    "heads" => "Heads",
                                    "remotes" => "Remotes",
                                    "tags" => "Tags",
                                    "objects" => "Loose Objects",
                                    _ => folder_name.as_ref(),
                                };
                                let content = self
                                    .educational_content_provider
                                    .get_category_content(content_key);
                                Message::LoadEducationalContent(Ok(content))
                            } else {
                                // For non-educational folders, show basic folder info
                                let content = ratatui::text::Text::from(format!(
                                    "Directory: {}\n\nThis is a Git directory containing various files and subdirectories.",
                                    path.display()
                                ));
                                Message::LoadEducationalContent(Ok(content))
                            }
                        }
                        GitObjectType::FileSystemFile { path, .. } => {
                            // For files, show basic file info
                            let content = ratatui::text::Text::from(format!(
                                "File: {}\n\nThis is a file in the Git repository. You can examine its contents using standard file tools.",
                                path.display()
                            ));
                            Message::LoadEducationalContent(Ok(content))
                        }
                        // For actual objects, show previews instead of educational content
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
                        GitObjectType::PackFolder { .. } => {
                            let content = self
                                .educational_content_provider
                                .get_category_content("Packs");
                            Message::LoadEducationalContent(Ok(content))
                        }
                        GitObjectType::PackFile {
                            file_type, path, ..
                        } => {
                            match file_type.as_str() {
                                "packfile" => {
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
                                                Err(e) => Message::LoadEducationalContent(Err(
                                                    format!("Error parsing pack header: {e:?}"),
                                                )),
                                            }
                                        }
                                        Err(e) => Message::LoadEducationalContent(Err(format!(
                                            "Error reading file: {e}"
                                        ))),
                                    }
                                }
                                "index" => {
                                    let content = self
                                        .educational_content_provider
                                        .get_category_content("Pack Index");
                                    Message::LoadEducationalContent(Ok(content))
                                }
                                "xedni" => {
                                    let content = self
                                        .educational_content_provider
                                        .get_category_content("Reverse Index");
                                    Message::LoadEducationalContent(Ok(content))
                                }
                                "mtime" => {
                                    let content = self
                                        .educational_content_provider
                                        .get_category_content("Pack Mtimes");
                                    Message::LoadEducationalContent(Ok(content))
                                }
                                _ => {
                                    let content = self
                                        .educational_content_provider
                                        .get_category_content("Packs");
                                    Message::LoadEducationalContent(Ok(content))
                                }
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
