use crate::git::loose_object::CommitObject;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

pub struct CommitFormatter<'a> {
    commit: &'a CommitObject,
}

impl<'a> CommitFormatter<'a> {
    #[must_use]
    pub const fn new(commit: &'a CommitObject) -> Self {
        Self { commit }
    }

    pub fn format_commit(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "COMMIT DETAILS",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(40)));
        lines.push(Line::from(""));

        // Tree
        lines.push(Line::from(format!("Tree: {}", self.commit.tree)));
        lines.push(Line::from(""));

        // Parents
        if self.commit.parents.is_empty() {
            lines.push(Line::from("Parents: none (root commit)"));
        } else {
            lines.push(Line::from("Parents:"));
            for parent in &self.commit.parents {
                lines.push(Line::from(format!("  {parent}")));
            }
        }
        lines.push(Line::from(""));

        // Author
        lines.push(Line::styled(
            "Author Information:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(format!("  Name: {}", self.commit.author)));
        lines.push(Line::from(format!(
            "  Date: {}",
            self.format_timestamp(&self.commit.author_date)
        )));
        lines.push(Line::from(""));

        // Committer
        lines.push(Line::styled(
            "Committer Information:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(format!("  Name: {}", self.commit.committer)));
        lines.push(Line::from(format!(
            "  Date: {}",
            self.format_timestamp(&self.commit.committer_date)
        )));
        lines.push(Line::from(""));

        // Message
        lines.push(Line::styled(
            "Commit Message:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(20)));

        // Split message into lines and add them
        for line in self.commit.message.lines() {
            lines.push(Line::from(line.to_string()));
        }

        if self.commit.message.is_empty() {
            lines.push(Line::from("(no message)"));
        }
    }

    fn format_timestamp(&self, timestamp: &str) -> String {
        // For now, just return the raw timestamp
        // In the future, we could parse Unix timestamps manually
        timestamp.to_string()
    }
}
