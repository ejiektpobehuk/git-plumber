use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};
use std::env;
use std::path::PathBuf;

use super::model::{MainViewState, PackFocus, RegularFocus};
use super::{PackPreViewState, PreviewState};
use crate::tui::helpers::{
    render_list_with_scrollbar, render_paragraph_with_scrollbar,
    render_styled_paragraph_with_scrollbar,
};
use crate::tui::model::{AppState, AppView, GitObjectType};
use crate::tui::pack_details::render_pack_object_detail_view_with_cache;

pub fn render(f: &mut ratatui::Frame, app: &AppState, area: ratatui::layout::Rect) {
    if let AppView::Main { state } = &app.view {
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

        render_list_with_scrollbar(
            f,
            content_chunks[0],
            &state.git_objects.flat_view,
            Some(state.git_objects.selected_index),
            state.git_objects.scroll_position,
            &format!("{}/.git", app.project_name),
            state.are_git_objects_focused(),
            |i, (depth, obj), is_selected| {
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
                                let (ancestor_depth, _) = &state.git_objects.flat_view[k];
                                if *ancestor_depth == d + 1 {
                                    ancestor_index = Some(k);
                                    break;
                                } else if *ancestor_depth <= d {
                                    break;
                                }
                            }

                            // If we found an ancestor, check if it has siblings after it
                            if let Some(ancestor_idx) = ancestor_index {
                                let mut has_sibling = false;
                                for j in (ancestor_idx + 1)..state.git_objects.flat_view.len() {
                                    let (next_depth, _) = &state.git_objects.flat_view[j];
                                    if *next_depth == d + 1 {
                                        has_sibling = true;
                                        break;
                                    } else if *next_depth <= d {
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

                // Add expansion indicator for categories
                let prefix = match &obj.obj_type {
                    GitObjectType::Category(_) if !obj.children.is_empty() => {
                        if obj.expanded {
                            if *depth == 0 {
                                "▼ "
                            } else {
                                // Find if this is the last category at this depth
                                let is_last = {
                                    let mut is_last = true;
                                    for j in (i + 1)..state.git_objects.flat_view.len() {
                                        let (next_depth, _) = &state.git_objects.flat_view[j];
                                        if *next_depth == *depth {
                                            is_last = false;
                                            break;
                                        } else if *next_depth < *depth {
                                            break;
                                        }
                                    }
                                    is_last
                                };
                                if is_last { "└▼ " } else { "├▼ " }
                            }
                        } else {
                            if *depth == 0 {
                                "▶ "
                            } else {
                                // Find if this is the last category at this depth
                                let is_last = {
                                    let mut is_last = true;
                                    for j in (i + 1)..state.git_objects.flat_view.len() {
                                        let (next_depth, _) = &state.git_objects.flat_view[j];
                                        if *next_depth == *depth {
                                            is_last = false;
                                            break;
                                        } else if *next_depth < *depth {
                                            break;
                                        }
                                    }
                                    is_last
                                };
                                if is_last { "└▶ " } else { "├▶ " }
                            }
                        }
                    }
                    GitObjectType::Category(_) => {
                        if *depth == 0 {
                            "  "
                        } else {
                            // Find if this is the last category at this depth
                            let is_last = {
                                let mut is_last = true;
                                for j in (i + 1)..state.git_objects.flat_view.len() {
                                    let (next_depth, _) = &state.git_objects.flat_view[j];
                                    if *next_depth == *depth {
                                        is_last = false;
                                        break;
                                    } else if *next_depth < *depth {
                                        break;
                                    }
                                }
                                is_last
                            };
                            if is_last { "└─ " } else { "├─ " }
                        }
                    }
                    _ => {
                        // Find if this is the last item in its group
                        let is_last = if *depth > 0 {
                            // Look ahead to find the next item at the same depth
                            let mut is_last = true;
                            for j in (i + 1)..state.git_objects.flat_view.len() {
                                let (next_depth, _) = &state.git_objects.flat_view[j];
                                if *next_depth == *depth {
                                    is_last = false;
                                    break;
                                } else if *next_depth < *depth {
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

                ListItem::new(display_text).style(if is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
            },
        );
        match &state.preview_state {
            PreviewState::Regular(_) => render_regular_preview_layout(f, app, content_chunks[1]),
            PreviewState::Pack(_) => render_pack_preview_layout(f, app, content_chunks[1]),
        };
    }
}

fn render_regular_preview_layout(
    f: &mut ratatui::Frame,
    app: &AppState,
    area: ratatui::layout::Rect,
) {
    if let AppView::Main { state } = &app.view {
        if let PreviewState::Regular(preview_state) = &state.preview_state {
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
            let object_info = if app.error.is_some() {
                app.error.as_ref().unwrap()
            } else if state.git_object_info.is_empty() && !state.git_objects.flat_view.is_empty() {
                "Select an object to view details"
            } else if state.git_objects.flat_view.is_empty() {
                "No Git objects found"
            } else {
                &state.git_object_info
            };

            let details_widget = Paragraph::new(object_info).block(
                Block::default()
                    .title("Object Details")
                    .borders(Borders::ALL),
            );
            f.render_widget(details_widget, content_chunks[0]);

            // Bottom block - Educational/Preview content
            let bottom_title = if !state.git_objects.flat_view.is_empty()
                && state.git_objects.selected_index < state.git_objects.flat_view.len()
            {
                let selected_object =
                    &state.git_objects.flat_view[state.git_objects.selected_index].1;
                match selected_object.obj_type {
                    GitObjectType::Category(_) => "Educational Content",
                    _ => "Object Preview",
                }
            } else {
                "Content"
            };

            render_styled_paragraph_with_scrollbar(
                f,
                content_chunks[1],
                state.educational_content.clone(),
                preview_state.preview_scroll_position,
                bottom_title,
                matches!(preview_state.focus, RegularFocus::Preview),
            );
        }
    }
}

pub fn render_pack_preview_layout(
    f: &mut ratatui::Frame,
    app: &AppState,
    area: ratatui::layout::Rect,
) {
    if let AppView::Main {
        state:
            MainViewState {
                preview_state:
                    PreviewState::Pack(PackPreViewState {
                        pack_file_path,
                        pack_object_list,
                        selected_pack_object,
                        focus,
                        pack_object_preview_scroll_position,
                        pack_object_text_cache,
                        ..
                    }),
                ..
            },
    } = &app.view
    {
        if app.is_wide_screen() {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(46), Constraint::Min(0)].as_ref())
                .split(area);

            let pack_file_details = horizontal_chunks[0];
            let object_inside_the_pack_preview = horizontal_chunks[1];

            // Render main content in the left area
            render_pack_preview_2_panels(f, app, pack_file_details);

            // Render pack detail in the right area only if pack_object_list is not empty
            if !pack_object_list.is_empty() && *selected_pack_object < pack_object_list.len() {
                render_pack_object_detail_view_with_cache(
                    f,
                    object_inside_the_pack_preview,
                    &pack_object_list[*selected_pack_object],
                    *pack_object_preview_scroll_position,
                    "Pack Object Detail",
                    matches!(focus, PackFocus::PackObjectDetails),
                    Some(pack_object_text_cache),
                );
            } else {
                // Render empty state
                let empty_widget = Paragraph::new("Loading pack objects...").block(
                    Block::default()
                        .title("Pack Object Detail")
                        .borders(Borders::ALL),
                );
                f.render_widget(empty_widget, object_inside_the_pack_preview);
            }
        } else {
            render_pack_preview_2_panels(f, app, area);
        }
    }
}

fn render_pack_preview_2_panels(
    f: &mut ratatui::Frame,
    app: &AppState,
    area: ratatui::layout::Rect,
) {
    if let AppView::Main { state } = &app.view {
        if let PreviewState::Pack(preview_state) = &state.preview_state {
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
            let object_info = if app.error.is_some() {
                app.error.as_ref().unwrap()
            } else if state.git_object_info.is_empty() && !state.git_objects.flat_view.is_empty() {
                "Select an object to view details"
            } else if state.git_objects.flat_view.is_empty() {
                "No Git objects found"
            } else {
                &state.git_object_info
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
                state.educational_content.clone(),
                preview_state.educational_scroll_position,
                "Pack File Header",
                matches!(preview_state.focus, PackFocus::Educational),
            );
            // Bottom block - Pack objects list
            // Only highlight if in ObjectPreview mode and focus is PackObjects
            let selected_index = if preview_state.pack_object_list.is_empty() {
                None
            } else {
                Some(
                    preview_state
                        .selected_pack_object
                        .min(preview_state.pack_object_list.len().saturating_sub(1)),
                )
            };

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

                    ListItem::new(display_text).style(if is_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    })
                },
            );
        }
    }
}
