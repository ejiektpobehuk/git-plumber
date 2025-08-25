use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout};

fn apply_git_tree_highlight_fx(
    buf: &mut Buffer,
    area: ratatui::layout::Rect,
    state: &crate::tui::main_view::MainViewState,
    reduced: bool,
    now: std::time::Instant,
) {
    let (hold_ms, shrink_ms) = if reduced {
        (5000_u64, 0_u64)
    } else {
        (1000_u64, 4000_u64)
    };
    let total = hold_ms + shrink_ms;

    // Use inner content area (exclude borders) so row indexing matches rendered list rows
    let start_row = area.y.saturating_add(1);
    let end_row = area.y.saturating_add(area.height.saturating_sub(1));
    let start_col = area.x.saturating_add(1);
    let width = area.width.saturating_sub(2);

    for (row_idx, y) in (start_row..end_row).enumerate() {
        let idx = state.tree.scroll_position + row_idx;
        if idx >= state.tree.flat_view.len() {
            continue;
        }
        let row = &state.tree.flat_view[idx];
        let _key = crate::tui::main_view::MainViewState::selection_key(&row.object);

        // Use highlight information from the flattened tree row
        let (color, start) = if let Some(highlight_color) = row.highlight.color {
            let expires_at = row.highlight.expires_at.unwrap_or(now);
            if expires_at > now {
                let start_time = expires_at
                    .checked_sub(std::time::Duration::from_millis(total))
                    .unwrap();
                (Some(highlight_color), Some(start_time))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let (bg, start_at) = match (color, start) {
            (Some(c), Some(s)) => (c, s),
            _ => continue,
        };

        let n_cols: u16 = if reduced {
            if now.duration_since(start_at).as_millis() as u64 <= hold_ms {
                width
            } else {
                0
            }
        } else {
            let elapsed = now.saturating_duration_since(start_at);
            if elapsed.as_millis() as u64 <= hold_ms {
                width
            } else {
                let after = elapsed - std::time::Duration::from_millis(hold_ms);
                if after.as_millis() as u64 >= shrink_ms {
                    0
                } else {
                    let p = after.as_secs_f32() / (shrink_ms as f32 / 1000.0);
                    (f32::from(width) * (1.0 - p)).ceil() as u16
                }
            }
        };

        if n_cols == 0 {
            continue;
        }

        let hi = n_cols.min(width);
        for dx in 0..hi {
            let x = start_col + dx;
            if let Some(cell) = buf.cell_mut((x, y)) {
                let s = cell.style().bg(bg);
                cell.set_style(s);
            }
        }
    }
}

use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};
use std::time::Instant;

use super::model::{MainViewState, PackFocus, RegularFocus};
use super::{PackPreViewState, PreviewState, RegularPreViewState};
use crate::tui::helpers::{render_list_with_scrollbar, render_styled_paragraph_with_scrollbar};
use crate::tui::model::{AppState, AppView, GitObjectType};

pub fn render(f: &mut ratatui::Frame, app: &mut AppState, area: ratatui::layout::Rect) {
    let project_name = app.project_name.clone();
    let reduced = app.reduced_motion;
    if let AppView::Main { state } = &mut app.view {
        // Split main content into two blocks
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(42), // 40 chars + 2 for borders
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(area);

        render_git_tree(f, state, project_name, content_chunks[0], reduced);
        // Apply cell-based highlight after rendering the tree
        apply_git_tree_highlight_fx(
            f.buffer_mut(),
            content_chunks[0],
            state,
            reduced,
            Instant::now(),
        );
        match &state.preview_state {
            PreviewState::Regular(_) => {
                render_regular_preview_layout(f, state, &app.error, content_chunks[1]);
            }
            PreviewState::Pack(_) => {
                render_pack_preview_layout(f, state, &app.error, content_chunks[1]);
            }
        }
    }
}

fn render_regular_preview_layout(
    f: &mut ratatui::Frame,
    main_view: &mut MainViewState,
    app_error: &Option<String>,
    area: ratatui::layout::Rect,
) {
    if let PreviewState::Regular(preview_state) = &mut main_view.preview_state {
        // Split area into two vertical sections for consistent layout
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(6), // Height for object details
                    Constraint::Min(0),    // Remaining space for educational content
                ]
                .as_ref(),
            )
            .split(area);

        // Top block - Object details
        let object_info = if app_error.is_some() {
            app_error.as_ref().unwrap()
        } else if main_view.content.git_object_info.is_empty()
            && !main_view.tree.flat_view.is_empty()
        {
            "Select an object to view details"
        } else if main_view.tree.flat_view.is_empty() {
            "Loading repository…"
        } else {
            &main_view.content.git_object_info
        };

        let details_widget = Paragraph::new(object_info).block(
            Block::default()
                .title("Object Details")
                .borders(Borders::ALL),
        );
        f.render_widget(details_widget, content_chunks[0]);

        // Bottom block - Educational/Preview content or Pack Index widget
        if let Some(pack_index_widget) = &mut preview_state.pack_index_widget {
            // Render pack index widget
            pack_index_widget.render(
                f,
                content_chunks[1],
                matches!(preview_state.focus, RegularFocus::Preview),
            );
        } else {
            // Render regular educational content
            let bottom_title = if !main_view.tree.flat_view.is_empty()
                && main_view.tree.selected_index < main_view.tree.flat_view.len()
            {
                let selected_object =
                    &main_view.tree.flat_view[main_view.tree.selected_index].object;
                match &selected_object.obj_type {
                    GitObjectType::Category(_) => "Educational Content",
                    GitObjectType::FileSystemFolder { is_educational, .. } => {
                        if *is_educational {
                            "Educational Content"
                        } else {
                            "Directory Info"
                        }
                    }
                    GitObjectType::FileSystemFile { .. } => "File Info",
                    GitObjectType::PackFolder { .. } => "Pack Preview",
                    _ => "Object Preview",
                }
            } else {
                "Content"
            };

            render_styled_paragraph_with_scrollbar(
                f,
                content_chunks[1],
                main_view.content.educational_content.clone(),
                preview_state.preview_scroll_position,
                bottom_title,
                matches!(preview_state.focus, RegularFocus::Preview),
            );
        }
    }
}

pub fn render_pack_preview_layout(
    f: &mut ratatui::Frame,
    main_view: &mut MainViewState,
    app_error: &Option<String>,
    area: ratatui::layout::Rect,
) {
    if let PreviewState::Pack(_) = &main_view.preview_state {
        if area.width > 116 {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(46), Constraint::Min(0)].as_ref())
                .split(area);

            let pack_file_details = horizontal_chunks[0];
            let object_details_area = horizontal_chunks[1];

            // Render main content in the left area
            render_pack_file_preview(f, main_view, app_error, pack_file_details, true);

            // Extract the data we need first
            if let PreviewState::Pack(pack_preview_state) = &mut main_view.preview_state {
                // Render pack detail in the right area only if pack_object_list is not empty
                if !pack_preview_state.pack_object_list.is_empty()
                    && pack_preview_state.selected_pack_object
                        < pack_preview_state.pack_object_list.len()
                {
                    pack_preview_state.pack_object_widget_state.render(
                        f,
                        object_details_area,
                        matches!(pack_preview_state.focus, PackFocus::PackObjectDetails),
                    );
                } else {
                    // Render empty state
                    let empty_widget = Paragraph::new("Loading pack objects...").block(
                        Block::default()
                            .title("Pack Object Detail")
                            .borders(Borders::ALL),
                    );
                    f.render_widget(empty_widget, object_details_area);
                }
            }
        } else {
            render_pack_file_preview(f, main_view, app_error, area, false);
        }
    }
}

fn render_pack_file_preview(
    f: &mut ratatui::Frame,
    main_view: &MainViewState,
    app_error: &Option<String>,
    area: ratatui::layout::Rect,
    is_widescreen: bool,
) {
    if let PreviewState::Pack(preview_state) = &main_view.preview_state {
        // Split area into three vertical sections for consistent layout
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(6),      // Height for object details
                    Constraint::Percentage(50), // Educational content
                    Constraint::Percentage(50), // Pack objects list
                ]
                .as_ref(),
            )
            .split(area);

        // Top block - Object details (same as PackPreview)
        let object_info = if app_error.is_some() {
            app_error.as_ref().unwrap()
        } else if main_view.content.git_object_info.is_empty()
            && !main_view.tree.flat_view.is_empty()
        {
            "Select an object to view details"
        } else if main_view.tree.flat_view.is_empty() {
            "Loading repository…"
        } else {
            &main_view.content.git_object_info
        };

        let details_widget = Paragraph::new(object_info).block(
            Block::default()
                .title("Object Details")
                .borders(Borders::ALL),
        );
        f.render_widget(details_widget, content_chunks[0]);

        // Middle block - Educational content with scrolling
        // Only highlight if in ObjectPreview mode and focus is Educational
        render_styled_paragraph_with_scrollbar(
            f,
            content_chunks[1],
            main_view.content.educational_content.clone(),
            preview_state.educational_scroll_position,
            "Pack File Header",
            matches!(preview_state.focus, PackFocus::Educational),
        );
        // Bottom block - Pack objects list
        // Only highlight if in ObjectPreview mode and focus is PackObjects
        if preview_state.pack_object_list.is_empty() {
            let loading = Paragraph::new("Loading pack objects...")
                .block(Block::default().title("Pack Objects").borders(Borders::ALL));
            f.render_widget(loading, content_chunks[2]);
        } else {
            let selected_index = Some(
                preview_state
                    .selected_pack_object
                    .min(preview_state.pack_object_list.len().saturating_sub(1)),
            );

            render_list_with_scrollbar(
                f,
                content_chunks[2],
                &preview_state.pack_object_list,
                selected_index,
                preview_state.pack_object_list_scroll_position,
                "Pack Objects",
                matches!(preview_state.focus, PackFocus::PackObjectsList),
                |_absolute_index, pack_obj, is_selected| {
                    let display_text = format!(
                        "{}: {} | {} bytes{}",
                        pack_obj.index,
                        pack_obj.obj_type,
                        pack_obj.size,
                        if let Some(ref hash) = pack_obj.sha1 {
                            format!(" | {hash}")
                        } else {
                            String::new()
                        }
                    );

                    ListItem::new(display_text).style(
                        if (is_widescreen
                            || matches!(preview_state.focus, PackFocus::PackObjectsList))
                            && is_selected
                        {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    )
                },
            );
        }
    }
}

pub fn navigation_hints(app: &AppState) -> Vec<Span<'_>> {
    let is_wide_screen = app.is_wide_screen();
    let mut hints = Vec::new();
    if let AppView::Main { state } = &app.view {
        let MainViewState { preview_state, .. } = &state;
        match &preview_state {
            PreviewState::Pack(PackPreViewState { focus, .. }) => match focus {
                PackFocus::GitObjects => {
                    hints.append(&mut vec![
                        Span::styled("←", Style::default().fg(Color::Green)),
                        Span::styled("↕→", Style::default().fg(Color::Blue)),
                    ]);
                }
                PackFocus::Educational => {
                    if is_wide_screen {
                        hints.push(Span::styled("←↕→", Style::default().fg(Color::Blue)));
                    } else {
                        hints.append(&mut vec![
                            Span::styled("←↕", Style::default().fg(Color::Blue)),
                            Span::styled("→", Style::default().fg(Color::Gray)),
                        ]);
                    }
                }
                PackFocus::PackObjectsList => {
                    if is_wide_screen {
                        hints.push(Span::styled("←↕→", Style::default().fg(Color::Blue)));
                    } else {
                        hints.append(&mut vec![
                            Span::styled("←↕", Style::default().fg(Color::Blue)),
                            Span::styled("→", Style::default().fg(Color::Green)),
                        ]);
                    }
                }
                PackFocus::PackObjectDetails => {
                    hints.append(&mut vec![
                        Span::styled("←↕", Style::default().fg(Color::Blue)),
                        Span::styled("→", Style::default().fg(Color::Gray)),
                    ]);
                }
            },
            PreviewState::Regular(RegularPreViewState { focus, .. }) => match focus {
                RegularFocus::GitObjects => {
                    if !state.tree.flat_view.is_empty()
                        && state.tree.selected_index < state.tree.flat_view.len()
                    {
                        match state.tree.flat_view[state.tree.selected_index]
                            .object
                            .obj_type
                        {
                            GitObjectType::Category(_) => {
                                hints.append(&mut vec![
                                    Span::styled("←", Style::default().fg(Color::Green)),
                                    Span::styled("↕→", Style::default().fg(Color::Blue)),
                                ]);
                            }
                            _ => {
                                hints.append(&mut vec![
                                    Span::styled("←", Style::default().fg(Color::Green)),
                                    Span::styled("↕→", Style::default().fg(Color::Blue)),
                                ]);
                            }
                        }
                    }
                }
                RegularFocus::Preview => {
                    hints.append(&mut vec![
                        Span::styled("←↕", Style::default().fg(Color::Blue)),
                        Span::styled("→", Style::default().fg(Color::Gray)),
                    ]);
                }
            },
        }
    }
    hints.append(&mut vec![
        Span::raw(" to navigate | "),
        Span::raw("("),
        Span::styled("Q", Style::default().fg(Color::Blue)),
        Span::raw(")uit"),
    ]);
    hints
}

fn render_git_tree(
    f: &mut ratatui::Frame,
    state: &MainViewState,
    project_name: String,
    area: ratatui::layout::Rect,
    reduced: bool,
) {
    render_list_with_scrollbar(
        f,
        area,
        &state.tree.flat_view,
        Some(state.tree.selected_index),
        state.tree.scroll_position,
        &format!("{project_name}/.git"),
        state.are_git_objects_focused(),
        |i, row, is_selected| {
            let depth = &row.depth;
            let obj = &row.object;
            let _ = reduced;
            // Create indentation based on depth
            let indent = if *depth > 0 {
                let mut indent = String::new();

                // For each level from 0 to depth-1, determine if we need a vertical line
                for d in 0..(*depth - 1) {
                    // We need a vertical line at depth d if there are more siblings
                    // at depth d that will come after the current branch
                    let needs_vertical_line = {
                        // Find the ancestor of the current item at depth d+1
                        let mut ancestor_index = None;
                        for k in (0..i).rev() {
                            let ancestor_row = &state.tree.flat_view[k];
                            if ancestor_row.depth == d + 1 {
                                ancestor_index = Some(k);
                                break;
                            } else if ancestor_row.depth <= d {
                                break;
                            }
                        }

                        // If we found an ancestor, check if it has siblings after it
                        if let Some(ancestor_idx) = ancestor_index {
                            let mut has_sibling = false;
                            for j in (ancestor_idx + 1)..state.tree.flat_view.len() {
                                let next_row = &state.tree.flat_view[j];
                                if next_row.depth == d + 1 {
                                    has_sibling = true;
                                    break;
                                } else if next_row.depth <= d {
                                    break;
                                }
                            }
                            has_sibling
                        } else {
                            false
                        }
                    };

                    indent.push_str(if needs_vertical_line { "│" } else { " " });
                }

                indent
            } else {
                String::new()
            };

            // Add expansion indicator for categories and folders
            let prefix = match &obj.obj_type {
                // Handle all folder types with unified empty-state detection
                GitObjectType::Category(_)
                | GitObjectType::FileSystemFolder { .. }
                | GitObjectType::PackFolder { .. } => {
                    // Helper function to determine if this is the last item at this depth
                    let is_last_at_depth = || {
                        let mut is_last = true;
                        for j in (i + 1)..state.tree.flat_view.len() {
                            let next_row = &state.tree.flat_view[j];
                            if next_row.depth == *depth {
                                is_last = false;
                                break;
                            } else if next_row.depth < *depth {
                                break;
                            }
                        }
                        is_last
                    };

                    // Determine symbols based on empty state
                    let (expanded_symbol, collapsed_symbol) = if obj.is_empty() {
                        // Empty folder symbols
                        ("▽", "▷")
                    } else {
                        // Non-empty folder symbols
                        ("▼", "▶")
                    };

                    if obj.expanded {
                        if *depth == 0 {
                            if expanded_symbol == "▽" {
                                "▽ "
                            } else {
                                "▼ "
                            }
                        } else {
                            let is_last = is_last_at_depth();
                            if expanded_symbol == "▽" {
                                if is_last { "└▽ " } else { "├▽ " }
                            } else if is_last {
                                "└▼ "
                            } else {
                                "├▼ "
                            }
                        }
                    } else if *depth == 0 {
                        if collapsed_symbol == "▷" {
                            "▷ "
                        } else {
                            "▶ "
                        }
                    } else {
                        let is_last = is_last_at_depth();
                        if collapsed_symbol == "▷" {
                            if is_last { "└▷ " } else { "├▷ " }
                        } else if is_last {
                            "└▶ "
                        } else {
                            "├▶ "
                        }
                    }
                }
                _ => {
                    // Find if this is the last item in its group
                    let is_last = if *depth > 0 {
                        // Look ahead to find the next item at the same depth
                        let mut is_last = true;
                        for j in (i + 1)..state.tree.flat_view.len() {
                            let next_row = &state.tree.flat_view[j];
                            if next_row.depth == *depth {
                                is_last = false;
                                break;
                            } else if next_row.depth < *depth {
                                break;
                            }
                        }
                        is_last
                    } else {
                        false
                    };

                    match *depth {
                        0 => "",
                        _ => {
                            if is_last {
                                "└─ "
                            } else {
                                "├─ "
                            }
                        }
                    }
                }
            };

            let display_text = format!("{}{}{}", indent, prefix, obj.name);
            let _key = MainViewState::selection_key(obj);

            // Simple item rendering; highlight is applied in a post-render cell pass
            ListItem::new(display_text).style({
                if is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }
            })
        },
    );

    // If there are no items yet, render a placeholder "Loading…"
    if state.tree.flat_view.is_empty() {
        use ratatui::widgets::Paragraph;
        let placeholder = Paragraph::new("Loading…").block(
            Block::default()
                .title(format!("{project_name}/.git"))
                .borders(Borders::ALL),
        );
        f.render_widget(placeholder, area);
    }
}
