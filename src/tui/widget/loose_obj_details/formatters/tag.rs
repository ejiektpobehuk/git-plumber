use crate::git::loose_object::TagObject;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

pub struct TagFormatter<'a> {
    tag: &'a TagObject,
}

impl<'a> TagFormatter<'a> {
    pub const fn new(tag: &'a TagObject) -> Self {
        Self { tag }
    }

    pub fn format_tag(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "TAG DETAILS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(40)));
        lines.push(Line::from(""));

        // Basic tag information
        lines.push(Line::from(format!("Tag: {}", self.tag.tag)));
        lines.push(Line::from(format!("Object: {}", self.tag.object)));
        lines.push(Line::from(format!("Type: {}", self.tag.object_type)));
        lines.push(Line::from(""));

        // Tagger information
        if let Some(ref tagger) = self.tag.tagger {
            lines.push(Line::styled(
                "Tagger Information:",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(format!("  Name: {tagger}")));

            if let Some(ref tagger_date) = self.tag.tagger_date {
                lines.push(Line::from(format!(
                    "  Date: {}",
                    self.format_timestamp(tagger_date)
                )));
            }
        } else {
            lines.push(Line::from("Tagger: (none)"));
        }
        lines.push(Line::from(""));

        // Tag message
        lines.push(Line::styled(
            "Tag Message:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(15)));

        if self.tag.message.is_empty() {
            lines.push(Line::from("(no message)"));
        } else {
            // Split message into lines and add them
            for line in self.tag.message.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }

        lines.push(Line::from(""));

        // Additional information
        lines.push(Line::styled(
            "Additional Information:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(25)));

        let tag_type = match self.tag.object_type.as_str() {
            "commit" => "This tag points to a commit object",
            "tree" => "This tag points to a tree object",
            "blob" => "This tag points to a blob object",
            "tag" => "This tag points to another tag object",
            _ => "This tag points to an unknown object type",
        };

        lines.push(Line::from(tag_type));
        lines.push(Line::from(""));

        if self.tag.object_type == "commit" {
            lines.push(Line::from(
                "Use git log or git show to see the commit details.",
            ));
        } else if self.tag.object_type == "tree" {
            lines.push(Line::from("Use git ls-tree to see the tree contents."));
        } else if self.tag.object_type == "blob" {
            lines.push(Line::from("Use git cat-file to see the blob contents."));
        }
    }

    fn format_timestamp(&self, timestamp: &str) -> String {
        // For now, just return the raw timestamp
        // In the future, we could parse Unix timestamps manually
        timestamp.to_string()
    }
}
