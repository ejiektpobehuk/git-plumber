use crate::git::pack::PackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub struct HeaderFormatter<'a> {
    pack_index: &'a PackIndex,
}

impl<'a> HeaderFormatter<'a> {
    #[must_use]
    pub const fn new(pack_index: &'a PackIndex) -> Self {
        Self { pack_index }
    }

    pub fn format_header(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "INDEX HEADER",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(20)));
        lines.push(Line::from(""));

        let border_style = Style::default().fg(Color::Gray);
        let left_bit_style = Style::default().fg(Color::LightBlue);
        let right_bit_style = Style::default().fg(Color::LightGreen);

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
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[0] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[0] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[1] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[1] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[2] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[2] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[3] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[3] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[0] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[0] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[1] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[1] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[2] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[2] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[3] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[3] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("utf8"),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.pack_index.raw_data[0] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.pack_index.raw_data[1] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.pack_index.raw_data[2] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("│", border_style),
            Span::styled("  ╰─", left_bit_style),
            Span::from(format!("{}", self.pack_index.raw_data[3] as char)),
            Span::styled("─╯ ", right_bit_style),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ╰────────┴────────┴────────┴────────┴",
            border_style,
        )]));
        lines.push(Line::from(""));

        lines.push(Line::styled(
            "Version 1 has no signature and starts with  fanout table.",
            Style::default().fg(Color::Gray),
        ));
        lines.push(Line::styled(
            "So signature is something that makes no sense for a table.",
            Style::default().fg(Color::Gray),
        ));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled(
                "                Version: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::from(format!("{}", self.pack_index.version)),
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
        lines.push(Line::from(vec![
            Span::from("bin "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[4] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[4] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[5] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[5] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[6] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[6] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[7] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("{:04b}", self.pack_index.raw_data[7] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![
            Span::from("hex "),
            Span::styled("┊", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[4] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[4] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[5] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01x}╯", self.pack_index.raw_data[5] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[6] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[6] & 0x0F),
                right_bit_style,
            ),
            Span::styled("│", border_style),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[7] >> 4),
                left_bit_style,
            ),
            Span::styled(
                format!("╰─{:01X}╯", self.pack_index.raw_data[7] & 0x0F),
                right_bit_style,
            ),
            Span::styled("┊", border_style),
        ]));
        lines.push(Line::from(vec![Span::styled(
            "    ┴────────┴────────┴────────┴────────┴",
            border_style,
        )]));
        lines.push(Line::from(""));
    }
}
