use crate::git::pack::MultiPackIndex;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Maximum number of pack names to list before truncating
const MAX_PACKS_SHOWN: usize = 20;

pub struct PacksFormatter<'a> {
    multi_pack_index: &'a MultiPackIndex,
}

impl<'a> PacksFormatter<'a> {
    #[must_use]
    pub const fn new(multi_pack_index: &'a MultiPackIndex) -> Self {
        Self { multi_pack_index }
    }

    pub fn format_pack_names(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::styled(
            "PACK FILE NAMES (PNAM)",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let gray = Style::default().fg(Color::Gray);
        lines.push(Line::from(vec![Span::styled(
            "  NUL-terminated pack file names. An object's pack is referenced by",
            gray,
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  its position in this list (the \"pack-int-id\").",
            gray,
        )]));
        lines.push(Line::from(""));

        let pack_count = self.multi_pack_index.pack_count();
        if pack_count == 0 {
            lines.push(Line::from("No pack files listed."));
            lines.push(Line::from(""));
            return;
        }

        let shown = pack_count.min(MAX_PACKS_SHOWN);
        for (pack_id, name) in self
            .multi_pack_index
            .pack_names
            .iter()
            .take(shown)
            .enumerate()
        {
            lines.push(Line::from(vec![
                Span::styled(format!(" [{pack_id:3}] "), Style::default().fg(Color::LightBlue)),
                Span::styled(name.clone(), Style::default().fg(Color::Yellow)),
            ]));
        }
        if pack_count > shown {
            lines.push(Line::from(vec![Span::styled(
                format!("  ... ({} more packs)", pack_count - shown),
                gray,
            )]));
        }

        lines.push(Line::from(""));
    }
}
