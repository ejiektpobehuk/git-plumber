use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
/// Educational content for different Git object types and categories
use std::collections::HashMap;

pub struct EducationalContent {
    content_map: HashMap<String, Text<'static>>,
}

impl EducationalContent {
    pub fn new() -> Self {
        let mut content_map = HashMap::new();

        // Pack files educational content
        content_map.insert(
            "Packs".to_string(),
            Text::from(
                "PACK FILES\n\nPack files are Git's way of efficiently storing objects.\n\
             Instead of storing each object separately, Git combines them\n\
             into a single file with delta compression for better efficiency.\n\
             This significantly reduces repository size and improves performance. \n\
             \n\
             \n\
             Pack file header is 12 bytes:\n\
             \n\
             │1 2 3 4│5 6 7 8│9 0 1 2│\n\
             ├───────┼───────┼───────┤\n\
             │  Sign │  Ver  │ Count │\n\
             ╰───────┴───────┴───────╯\n\
             \n\
             Sign. This is a file type signature, a.k.a. a magic number.\n\
             It helps to identify pack files even whithout `.pack` extension.\n\
             For pack files it is always 4 bytes - \"PACK\".\n\
             \n\
             Ver is the version number of the pack file format.\n\
             Currently versions 2 and 3 are supported. But only version 2 is set.\n\
             \n\
             Count is the number of objects in the pack file. Stored in network byte order.\n\
             \n\
             Theoretical maximum for both version and number of objects is 4G.",
            ),
        );

        // References educational content
        content_map.insert(
            "Refs".to_string(),
            Text::from(
                "REFERENCES\n\nGit references are pointers to specific commits.\n\
             They help track branches, tags, and other important positions.\n\
             References make it easy to find commits without using hashes.\n\
             Common ref types: heads (branches), tags, remotes, and stash.",
            ),
        );

        // Heads (branches) educational content
        content_map.insert(
            "Heads".to_string(),
            Text::from(
                "BRANCHES (HEADS)\n\nBranches in Git are just references to specific commits.\n\
             Each branch is stored as a file in .git/refs/heads/.\n\
             The file contains the SHA-1 hash of the commit it points to.\n\
             When you commit to a branch, this reference is updated.",
            ),
        );

        // Remotes educational content
        content_map.insert(
            "Remotes".to_string(),
            Text::from(
                "REMOTE REFERENCES\n\nRemote references track branches from remote repositories.\n\
             They're stored in .git/refs/remotes/<remote-name>/.\n\
             These are updated when you fetch or pull from a remote.\n\
             Unlike local branches, you can't commit directly to them.",
            ),
        );

        // Tags educational content
        content_map.insert(
            "Tags".to_string(),
            Text::from(
                "TAGS\n\nTags are references that point to specific points in Git history.\n\
             Unlike branches, tags don't move as you make new commits.\n\
             They're stored in .git/refs/tags/ directory.\n\
             Lightweight tags are just refs, annotated tags are Git objects.",
            ),
        );

        // Loose Objects educational content
        content_map.insert(
            "Loose Objects".to_string(),
            Text::from(
                "LOOSE OBJECTS\n\nLoose objects are individual Git objects not yet packed.\n\
             They're stored in .git/objects/ with the first 2 hash chars as directory.\n\
             These include blobs (file contents), trees (directories),\n\
             commits (snapshots), and tags (references to commits).",
            ),
        );

        Self { content_map }
    }

    /// Get educational content for a specific category
    pub fn get_category_content(&self, category_name: &str) -> Text<'static> {
        self.content_map
            .get(category_name)
            .cloned()
            .unwrap_or_else(|| {
                Text::from(format!(
                    "Category: {category_name}\n\nThis groups related Git objects together."
                ))
            })
    }

    /// Get preview content for a reference file
    pub fn get_ref_preview(&self, content: &str) -> Text<'static> {
        Text::from(format!(
            "Reference Content\n\n{}\n\nThis reference points to the commit hash shown above.",
            content.trim()
        ))
    }

    /// Get preview content for a loose object
    pub fn get_loose_object_preview(&self, object_id: &str) -> Text<'static> {
        Text::from(format!(
            "Loose Object Preview\n\nObject ID: {object_id}\n\nThis is a raw Git object stored as a single file.\nUse 'cat-file' command to examine its contents."
        ))
    }

    /// Get pack file preview with detailed header breakdown
    pub fn get_pack_preview(&self, header: &crate::git::pack::Header) -> Text<'static> {
        let mut lines: Vec<Line> = Vec::new();
        let border_style = Style::default().fg(Color::Gray);
        let left_bit_style = Style::default().fg(Color::LightBlue);
        let right_bit_style = Style::default().fg(Color::LightGreen);

        // Ensure we have the expected 12 bytes of raw data
        if header.raw_data.len() < 12 {
            lines.push(Line::from("Error: Invalid header data"));
            return Text::from(lines);
        }

        lines.push(Line::from("Signature (magic number)").centered());
        lines.push(Line::styled(
            "byte│1        2        3        4",
            border_style,
        ));
        lines.push(Line::styled(
            "bit │76543210 76543210 76543210 76543210",
            border_style,
        ));
        lines.push(Line::from(vec![Span::styled(
            "    ├────────┼────────┼────────┼────────┼",
            border_style,
        )]));
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[0] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[0] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[1] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[1] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[2] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[2] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[3] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[3] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[0] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[0] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[1] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[1] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[2] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[2] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[3] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[3] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("utf8"),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", header.raw_data[0] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", header.raw_data[1] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", header.raw_data[2] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", header.raw_data[3] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ╰────────┴────────┴────────┴────────┴",
            border_style,
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(format!("Version: {}", header.version)).centered());
        lines.push(Line::styled(
            "byte 5        6        7        8",
            border_style,
        ));
        lines.push(Line::styled(
            "bit  76543210 76543210 76543210 76543210",
            border_style,
        ));
        lines.push(Line::from(vec![Span::styled(
            "    ┼────────┼────────┼────────┼────────┼",
            border_style,
        )]));
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("┊", border_style),
            Span::styled(format!("{:04b}", header.raw_data[4] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[4] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[5] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[5] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[6] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[6] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[7] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[7] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[4] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[4] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[5] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01x}╯", header.raw_data[5] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[6] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[6] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[7] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[7] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ┴────────┴────────┴────────┴────────┴",
            border_style,
        )]));
        lines.push(Line::from(""));

        lines.push(Line::from(format!("Number of objects: {}", header.object_count)).centered());
        lines.push(Line::styled(
            "byte 9        10       11       12      │",
            border_style,
        ));
        lines.push(Line::styled(
            "bit  76543210 76543210 76543210 76543210│",
            border_style,
        ));
        lines.push(Line::from(vec![Span::styled(
            "    ┼────────┼────────┼────────┼────────┤",
            border_style,
        )]));
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("┊", border_style),
            Span::styled(format!("{:04b}", header.raw_data[8] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[8] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[9] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[9] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[10] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[10] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(format!("{:04b}", header.raw_data[11] >> 4), left_bit_style),
            Span::styled(
                format!("{:04b}", header.raw_data[11] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[8] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[8] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[9] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[9] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[10] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[10] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[11] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", header.raw_data[11] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ┴────────┴────────┴────────┴────────╯",
            border_style,
        )]));

        Text::from(lines)
    }

    /// Get default content when no object is selected
    pub fn get_default_content(&self) -> Text<'static> {
        Text::from("Select an object to view details")
    }
}

impl Default for EducationalContent {
    fn default() -> Self {
        Self::new()
    }
}
