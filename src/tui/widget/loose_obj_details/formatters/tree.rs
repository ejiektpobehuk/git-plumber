use crate::git::loose_object::{TreeEntryType, TreeObject};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

pub struct TreeFormatter<'a> {
    tree: &'a TreeObject,
}

impl<'a> TreeFormatter<'a> {
    pub fn new(tree: &'a TreeObject) -> Self {
        Self { tree }
    }

    pub fn format_tree(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "TREE DETAILS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(40)));
        lines.push(Line::from(""));

        lines.push(Line::from(format!(
            "Total entries: {}",
            self.tree.entries.len()
        )));
        lines.push(Line::from(""));

        // Group entries by type for summary
        let mut blob_count = 0;
        let mut tree_count = 0;
        let mut executable_count = 0;
        let mut symlink_count = 0;
        let mut submodule_count = 0;

        for entry in &self.tree.entries {
            match entry.object_type {
                TreeEntryType::Blob => blob_count += 1,
                TreeEntryType::Tree => tree_count += 1,
                TreeEntryType::Executable => executable_count += 1,
                TreeEntryType::Symlink => symlink_count += 1,
                TreeEntryType::Submodule => submodule_count += 1,
            }
        }

        // Summary
        lines.push(Line::styled(
            "Entry Summary:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        if blob_count > 0 {
            lines.push(Line::from(format!("  Files (blobs): {blob_count}")));
        }
        if tree_count > 0 {
            lines.push(Line::from(format!("  Directories (trees): {tree_count}")));
        }
        if executable_count > 0 {
            lines.push(Line::from(format!("  Executables: {executable_count}")));
        }
        if symlink_count > 0 {
            lines.push(Line::from(format!("  Symlinks: {symlink_count}")));
        }
        if submodule_count > 0 {
            lines.push(Line::from(format!("  Submodules: {submodule_count}")));
        }
        lines.push(Line::from(""));

        // Detailed entries
        lines.push(Line::styled(
            "Tree Entries:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(20)));

        for entry in &self.tree.entries {
            let type_icon = match entry.object_type {
                TreeEntryType::Blob => "📄",
                TreeEntryType::Tree => "📁",
                TreeEntryType::Executable => "🚀",
                TreeEntryType::Symlink => "🔗",
                TreeEntryType::Submodule => "📦",
            };

            let type_desc = match entry.object_type {
                TreeEntryType::Blob => "blob",
                TreeEntryType::Tree => "tree",
                TreeEntryType::Executable => "exec",
                TreeEntryType::Symlink => "link",
                TreeEntryType::Submodule => "subm",
            };

            lines.push(Line::from(format!(
                "{} {} {} {} {}",
                type_icon,
                entry.mode,
                type_desc,
                &entry.sha1[..8],
                entry.name
            )));
        }

        if self.tree.entries.is_empty() {
            lines.push(Line::from("(empty tree)"));
        }
    }
}
