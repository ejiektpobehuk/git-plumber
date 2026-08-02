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

    /// Render the BTMP table: which slice of the multi-pack bitmap each pack owns
    ///
    /// Skipped entirely when the chunk is absent, which is the common case —
    /// BTMP is only written alongside a multi-pack bitmap.
    pub fn format_bitmapped_packs(&self, lines: &mut Vec<Line<'static>>) {
        let Some(bitmapped_packs) = &self.multi_pack_index.bitmapped_packs else {
            return;
        };

        lines.push(Line::styled(
            "BITMAPPED PACKFILES (BTMP)",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from("─".repeat(30)));
        lines.push(Line::from(""));

        let gray = Style::default().fg(Color::Gray);
        for note in [
            "  A multi-pack bitmap numbers objects in \"pseudo-pack\" order: one pack's",
            "  objects, then the next pack's, and so on. BTMP records where each pack's",
            "  run of bits starts and how long it is — one entry per pack, in PNAM",
            "  order, so the pack-int-id is the row number rather than a stored field.",
            "  This is what lets pack-objects reuse a byte range from any bitmapped",
            "  pack, not just the preferred one.",
            "",
            "  The preferred pack is placed first in pseudo-pack order, so the ranges",
            "  below need not ascend with the pack-int-id.",
        ] {
            lines.push(Line::from(vec![Span::styled(note, gray)]));
        }
        lines.push(Line::from(""));

        if bitmapped_packs.is_empty() {
            lines.push(Line::from("No bitmapped packs listed."));
            lines.push(Line::from(""));
            return;
        }

        lines.push(Line::from(vec![
            Span::styled(" Pack ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("│ ", gray),
            Span::styled(
                "Bit range   ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", gray),
            Span::styled("Objects", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", gray),
            Span::styled("Pack file", Style::default().add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::styled(
            "──────┼─────────────┼─────────┼───────────────────────────────────",
            gray,
        ));

        let shown = bitmapped_packs.len().min(MAX_PACKS_SHOWN);
        for pack in bitmapped_packs.iter().take(shown) {
            let name = self
                .multi_pack_index
                .pack_name_for(pack.pack_id)
                .unwrap_or("<unknown pack>");

            let (range, range_style) = if pack.is_empty() {
                // bitmap_pos is written as 0 for these; showing it would imply
                // they own bit 0
                ("—".to_string(), gray)
            } else {
                (
                    format!("{}..{}", pack.bitmap_pos, pack.bitmap_end()),
                    Style::default().fg(Color::Cyan),
                )
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" [{:2}] ", pack.pack_id),
                    Style::default().fg(Color::LightBlue),
                ),
                Span::styled("│ ", gray),
                Span::styled(format!("{range:<12}"), range_style),
                Span::styled("│ ", gray),
                Span::styled(
                    format!("{:>7} ", pack.bitmap_nr),
                    Style::default().fg(Color::LightGreen),
                ),
                Span::styled("│ ", gray),
                Span::styled(name.to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }
        if bitmapped_packs.len() > shown {
            lines.push(Line::from(vec![Span::styled(
                format!("  ... ({} more packs)", bitmapped_packs.len() - shown),
                gray,
            )]));
        }

        lines.push(Line::from(""));
        self.format_bitmap_coverage(lines, bitmapped_packs);
    }

    /// Compare the bits BTMP accounts for against the MIDX object count
    ///
    /// The runs should tile the bitmap exactly. They are checked in bit order,
    /// not pack order: the preferred pack takes bit 0 whatever its pack-int-id,
    /// so a valid table is routinely out of order by row.
    fn format_bitmap_coverage(
        &self,
        lines: &mut Vec<Line<'static>>,
        bitmapped_packs: &[crate::git::pack::BitmappedPack],
    ) {
        let total_bits = self.multi_pack_index.bitmapped_object_count().unwrap_or(0);
        let object_count = self.multi_pack_index.object_count() as u32;

        // Whichever pack claims bit 0 is the one placed first in pseudo-pack
        // order, i.e. the preferred pack
        if let Some(preferred) = self.multi_pack_index.pack_for_bitmap_pos(0) {
            let name = self
                .multi_pack_index
                .pack_name_for(preferred.pack_id)
                .unwrap_or("<unknown pack>");
            lines.push(Line::from(vec![
                Span::from("  First in pseudo-pack order (preferred pack): "),
                Span::styled(name.to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::from("  Bits accounted for: "),
            Span::styled(
                format!("{total_bits}"),
                Style::default().fg(Color::LightGreen),
            ),
            Span::from(" of "),
            Span::styled(
                format!("{object_count}"),
                Style::default().fg(Color::LightGreen),
            ),
            Span::from(" indexed objects"),
        ]));

        // Walk the non-empty runs in bit order; each must start where the
        // previous one ended, leaving no gap or overlap
        let mut runs: Vec<_> = bitmapped_packs
            .iter()
            .filter(|pack| !pack.is_empty())
            .collect();
        runs.sort_by_key(|pack| pack.bitmap_pos);

        let mut expected_pos = 0u32;
        let mut tiles = true;
        for pack in runs {
            if pack.bitmap_pos != expected_pos {
                tiles = false;
                break;
            }
            expected_pos = pack.bitmap_end();
        }

        if !tiles {
            lines.push(Line::from(vec![Span::styled(
                "  Warning: bit ranges leave a gap or overlap.",
                Style::default().fg(Color::Red),
            )]));
        } else if total_bits != object_count {
            lines.push(Line::from(vec![Span::styled(
                "  Warning: bit total does not match the indexed object count.",
                Style::default().fg(Color::Red),
            )]));
        }

        lines.push(Line::from(""));
    }
}
