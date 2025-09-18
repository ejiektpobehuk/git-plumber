use crate::git::pack::PackReverseIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct HeaderFormatter<'a> {
    reverse_index: &'a PackReverseIndex,
}

impl<'a> HeaderFormatter<'a> {
    #[must_use]
    pub const fn new(reverse_index: &'a PackReverseIndex) -> Self {
        Self { reverse_index }
    }

    pub fn format_header(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "REVERSE INDEX HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(24)));
        lines.push(Line::from(""));

        let border_style = Style::default().fg(Color::Gray);
        let left_bit_style = Style::default().fg(Color::LightBlue);
        let right_bit_style = Style::default().fg(Color::LightGreen);

        // Signature section
        lines.push(Line::from(vec![
            Span::styled(
                "        Signature",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::from(" (magic number)"),
        ]));
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

        // Binary representation
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[0] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[0] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[1] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[1] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[2] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[2] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[3] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[3] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));

        // Hex representation
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[0] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[0] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[1] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[1] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[2] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[2] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[3] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[3] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));

        // UTF-8 representation
        lines.push(Line::from(vec![
            Span::from("utf8"),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.reverse_index.raw_data[0] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.reverse_index.raw_data[1] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.reverse_index.raw_data[2] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.reverse_index.raw_data[3] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ╰────────┴────────┴────────┴────────┴",
            border_style,
        )]));
        lines.push(Line::from(""));

        // Version section
        lines.push(Line::from(vec![
            Span::styled(
                "                Version: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::from(format!("{}", self.reverse_index.version)),
        ]));
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

        // Version binary representation
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[4] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[4] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[5] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[5] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[6] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[6] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[7] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[7] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));

        // Version hex representation
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[4] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[4] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[5] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[5] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[6] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[6] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[7] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[7] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ┴────────┴────────┴────────┴────────┴",
            border_style,
        )]));
        lines.push(Line::from(""));

        // Hash Function ID section
        lines.push(Line::from(vec![
            Span::styled(
                "          Hash Function ID: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::from(format!(
                "{} ({})",
                self.reverse_index.hash_function_id,
                self.reverse_index.hash_function_name()
            )),
        ]));
        lines.push(Line::styled(
            "byte 9        10       11       12",
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

        // Hash function binary representation
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[8] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[8] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[9] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[9] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[10] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[10] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[11] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.reverse_index.raw_data[11] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));

        // Hash function hex representation
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[8] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[8] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[9] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[9] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[10] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[10] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[11] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.reverse_index.raw_data[11] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ┴────────┴────────┴────────┴────────┴",
            border_style,
        )]));
    }
}

// Legacy function for backward compatibility
pub fn format_header(reverse_index: &PackReverseIndex) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let formatter = HeaderFormatter::new(reverse_index);
    formatter.format_header(&mut lines);
    lines
}
