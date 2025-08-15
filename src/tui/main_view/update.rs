use super::model::{MainViewState, PackColumnPreviousFocus, PackFocus, PreviewState};
use super::{PackPreViewState, RegularFocus, RegularPreViewState};
use crate::tui::loose_details::LooseObjectViewState;
use crate::tui::message::{MainNavigation, Message};
use crate::tui::model::{AppState, AppView, GitObjectType};
use crate::tui::pack_details::PackViewState;
use crate::tui::widget::{PackObjectWidget, loose_obj_details::LooseObjectWidget};

impl AppState {
    // Handle git object selection with all associated updates
    fn handle_git_object_selection(
        &mut self,
        new_index: usize,
        is_pack_preview: bool,
        plumber: &crate::GitPlumber,
    ) {
        // Handle preview state transition based on object type
        self.handle_preview_state_transition(new_index, is_pack_preview);

        // Update scroll position to keep selected item visible
        self.update_git_objects_scroll_for_selection(new_index);

        // Load details for the newly selected object
        let details_msg = self.load_git_object_details(plumber);
        self.update(details_msg, plumber);

        // Load educational content for the newly selected object
        let content_msg = self.load_educational_content(plumber);
        self.update(content_msg, plumber);

        // Load pack objects if this is a pack file and update pack object details
        if is_pack_preview {
            self.load_pack_objects_if_needed(plumber);
            self.update_pack_object_widget_if_needed();
        }
    }

    // Update pack object widget to show details of the selected pack object
    fn update_pack_object_widget_if_needed(&mut self) {
        if let AppView::Main {
            state:
                MainViewState {
                    preview_state: PreviewState::Pack(pack_preview_state),
                    ..
                },
        } = &mut self.view
        {
            // If we have pack objects and the widget is uninitialized or showing a different object
            if !pack_preview_state.pack_object_list.is_empty() {
                let selected_pack_object = pack_preview_state.selected_pack_object;
                if selected_pack_object < pack_preview_state.pack_object_list.len() {
                    // Update the widget to show the selected pack object's details
                    pack_preview_state.pack_object_widget_state = PackObjectWidget::new(
                        pack_preview_state.pack_object_list[selected_pack_object].clone(),
                    );
                }
            }
        }
    }

    // Handle view mode transition messages for main view
    pub fn handle_main_view_mode_message(
        &mut self,
        msg: Message,
        plumber: &crate::GitPlumber,
    ) -> bool {
        match msg {
            Message::MainNavigation(msg) => match msg {
                MainNavigation::FocusGitObjects => {
                    if let AppView::Main { state } = &mut self.view {
                        match &mut state.preview_state {
                            PreviewState::Regular(preview_state) => {
                                preview_state.focus = RegularFocus::GitObjects
                            }
                            PreviewState::Pack(preview_state) => {
                                preview_state.focus = PackFocus::GitObjects
                            }
                        };
                    };
                }

                MainNavigation::SelectNextGitObject => {
                    let (should_update, new_index, is_pack_preview) = match &mut self.view {
                        AppView::Main {
                            state: MainViewState { git_objects, .. },
                            ..
                        } => {
                            if !git_objects.flat_view.is_empty() {
                                let new_index =
                                    (git_objects.selected_index + 1) % git_objects.flat_view.len();
                                git_objects.selected_index = new_index;

                                let is_pack = git_objects
                                    .flat_view
                                    .get(new_index)
                                    .map(|(_, obj, _)| {
                                        matches!(obj.obj_type, GitObjectType::Pack { .. })
                                    })
                                    .unwrap_or(false);
                                (true, new_index, is_pack)
                            } else {
                                (false, 0, false)
                            }
                        }
                        _ => (false, 0, false),
                    };

                    if should_update {
                        self.handle_git_object_selection(new_index, is_pack_preview, plumber);
                    }
                }

                MainNavigation::SelectPreviouwGitObject => {
                    let (should_update, new_index, is_pack_preview) = match &mut self.view {
                        AppView::Main {
                            state: MainViewState { git_objects, .. },
                        } => {
                            if !git_objects.flat_view.is_empty() {
                                let new_index = if git_objects.selected_index > 0 {
                                    git_objects.selected_index - 1
                                } else {
                                    git_objects.flat_view.len() - 1
                                };
                                git_objects.selected_index = new_index;

                                let is_pack = git_objects
                                    .flat_view
                                    .get(new_index)
                                    .map(|(_, obj, _)| {
                                        matches!(obj.obj_type, GitObjectType::Pack { .. })
                                    })
                                    .unwrap_or(false);
                                (true, new_index, is_pack)
                            } else {
                                (false, 0, false)
                            }
                        }
                        _ => (false, 0, false),
                    };

                    if should_update {
                        self.handle_git_object_selection(new_index, is_pack_preview, plumber);
                    }
                }

                MainNavigation::SelectFirstGitObject => {
                    let (should_update, new_index, is_pack_preview) = match &mut self.view {
                        AppView::Main {
                            state: MainViewState { git_objects, .. },
                        } => {
                            if !git_objects.flat_view.is_empty() {
                                git_objects.selected_index = 0;

                                let is_pack = git_objects
                                    .flat_view
                                    .first()
                                    .map(|(_, obj, _)| {
                                        matches!(obj.obj_type, GitObjectType::Pack { .. })
                                    })
                                    .unwrap_or(false);
                                (true, 0, is_pack)
                            } else {
                                (false, 0, false)
                            }
                        }
                        _ => (false, 0, false),
                    };

                    if should_update {
                        self.handle_git_object_selection(new_index, is_pack_preview, plumber);
                    }
                }

                MainNavigation::SelectLastGitObject => {
                    let (should_update, new_index, is_pack_preview) = match &mut self.view {
                        AppView::Main {
                            state: MainViewState { git_objects, .. },
                        } => {
                            if !git_objects.flat_view.is_empty() {
                                let new_index = git_objects.flat_view.len() - 1;
                                git_objects.selected_index = new_index;

                                let is_pack = git_objects
                                    .flat_view
                                    .get(new_index)
                                    .map(|(_, obj, _)| {
                                        matches!(obj.obj_type, GitObjectType::Pack { .. })
                                    })
                                    .unwrap_or(false);
                                (true, new_index, is_pack)
                            } else {
                                (false, 0, false)
                            }
                        }
                        _ => (false, 0, false),
                    };

                    if should_update {
                        self.handle_git_object_selection(new_index, is_pack_preview, plumber);
                    }
                }

                MainNavigation::ToggleExpand => {
                    let (toggle_msg, has_items, selected_index, is_pack) =
                        if let AppView::Main { state } = &mut self.view {
                            let toggle_msg = state.toggle_expand();
                            // Extract the information we need before calling update to avoid borrow conflicts
                            let has_items = !state.git_objects.flat_view.is_empty();
                            let selected_index = state.git_objects.selected_index;
                            let is_pack = if let Some((_, git_object, _)) =
                                state.git_objects.flat_view.get(selected_index)
                            {
                                matches!(git_object.obj_type, GitObjectType::Pack { .. })
                            } else {
                                false
                            };
                            (toggle_msg, has_items, selected_index, is_pack)
                        } else {
                            return true; // Not in main view
                        };

                    self.update(toggle_msg, plumber);

                    // If we still have items, load details and educational content
                    if has_items {
                        self.handle_git_object_selection(selected_index, is_pack, plumber);
                    }
                }

                MainNavigation::JumpToParentCategory => {
                    if let AppView::Main { state } = &mut self.view
                        && !state.git_objects.flat_view.is_empty()
                        && state.git_objects.selected_index < state.git_objects.flat_view.len()
                    {
                        let (current_depth, _, _) =
                            &state.git_objects.flat_view[state.git_objects.selected_index];

                        if *current_depth > 0 {
                            // Find parent category by looking backwards for an object at depth - 1
                            for i in (0..state.git_objects.selected_index).rev() {
                                let (parent_depth, parent_obj, _) = &state.git_objects.flat_view[i];
                                if *parent_depth == current_depth - 1
                                    && let GitObjectType::Category(_) = &parent_obj.obj_type
                                {
                                    // Jump to this parent category
                                    state.git_objects.selected_index = i;

                                    // Update scroll position to keep selected item visible
                                    self.update_git_objects_scroll_for_selection(i);

                                    // Load details for the newly selected object
                                    self.handle_git_object_selection(i, false, plumber);
                                    break;
                                }
                            }
                        }
                    }
                }

                MainNavigation::FocusEducationalOrList => {
                    if let AppView::Main { state } = &mut self.view {
                        match &mut state.preview_state {
                            PreviewState::Regular(regular_preview_state) => {
                                regular_preview_state.focus = RegularFocus::Preview;
                            }
                            PreviewState::Pack(pack_preview_state) => match &mut pack_preview_state
                                .focus
                            {
                                PackFocus::GitObjects | PackFocus::PackObjectDetails => {
                                    if let Some(previous_focus) = &pack_preview_state.previous_focus
                                    {
                                        match previous_focus {
                                            PackColumnPreviousFocus::Educational => {
                                                pack_preview_state.focus = PackFocus::Educational;
                                            }
                                            PackColumnPreviousFocus::PackObjectsList => {
                                                pack_preview_state.focus =
                                                    PackFocus::PackObjectsList;
                                            }
                                        }
                                    } else {
                                        pack_preview_state.focus = PackFocus::Educational;
                                        pack_preview_state.previous_focus =
                                            Some(PackColumnPreviousFocus::Educational);
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                }

                MainNavigation::FocusPackObjectDetails => {
                    if let AppView::Main { state } = &mut self.view
                        && let PreviewState::Pack(pack_preview_state) = &mut state.preview_state
                    {
                        pack_preview_state.focus = PackFocus::PackObjectDetails;
                    }
                }

                MainNavigation::FocusToggle => {
                    let is_wide_screen = self.is_wide_screen();
                    if let AppView::Main { state } = &mut self.view {
                        match &mut state.preview_state {
                            PreviewState::Regular(preview_state) => match preview_state.focus {
                                RegularFocus::GitObjects => {
                                    preview_state.focus = RegularFocus::Preview
                                }
                                RegularFocus::Preview => {
                                    preview_state.focus = RegularFocus::GitObjects
                                }
                            },
                            PreviewState::Pack(preview_state) => match preview_state.focus {
                                PackFocus::GitObjects => {
                                    preview_state.focus = PackFocus::Educational;
                                    preview_state.previous_focus =
                                        Some(PackColumnPreviousFocus::Educational);
                                }
                                PackFocus::Educational => {
                                    preview_state.focus = PackFocus::PackObjectsList;
                                    preview_state.previous_focus =
                                        Some(PackColumnPreviousFocus::PackObjectsList);
                                }
                                PackFocus::PackObjectsList => {
                                    if is_wide_screen {
                                        preview_state.focus = PackFocus::PackObjectDetails
                                    } else {
                                        preview_state.focus = PackFocus::GitObjects
                                    }
                                }
                                PackFocus::PackObjectDetails => {
                                    preview_state.focus = PackFocus::GitObjects
                                }
                            },
                        }
                    }
                }
                MainNavigation::SelectNextPackObject => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state:
                                    PreviewState::Pack(PackPreViewState {
                                        pack_object_list,
                                        pack_object_list_scroll_position,
                                        selected_pack_object,
                                        pack_object_widget_state: pack_widget_state,
                                        ..
                                    }),
                                ..
                            },
                        ..
                    } = &mut self.view
                        && !pack_object_list.is_empty()
                    {
                        if *selected_pack_object < pack_object_list.len() - 1 {
                            *selected_pack_object += 1;
                            *pack_widget_state = PackObjectWidget::new(
                                pack_object_list[*selected_pack_object].clone(),
                            );

                            // Update scroll position to keep selected item visible
                            let visible_height = self.layout_dimensions.pack_objects_height;
                            if *selected_pack_object
                                >= *pack_object_list_scroll_position + visible_height
                            {
                                *pack_object_list_scroll_position =
                                    *selected_pack_object - visible_height + 1;
                            }
                        } else {
                            *selected_pack_object = 0;
                            *pack_object_list_scroll_position = 0;
                            *pack_widget_state = PackObjectWidget::new(pack_object_list[0].clone());
                        }
                    }
                }
                MainNavigation::SelectPreviousPackObject => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state:
                                    PreviewState::Pack(PackPreViewState {
                                        pack_object_list,
                                        pack_object_list_scroll_position,
                                        selected_pack_object,
                                        focus,
                                        previous_focus,
                                        pack_object_widget_state: pack_widget_state,
                                        ..
                                    }),
                                ..
                            },
                        ..
                    } = &mut self.view
                        && !pack_object_list.is_empty()
                    {
                        if *selected_pack_object > 0 {
                            *selected_pack_object -= 1;
                            *pack_widget_state = PackObjectWidget::new(
                                pack_object_list[*selected_pack_object].clone(),
                            );

                            // Update scroll position to keep selected item visible
                            if *selected_pack_object < *pack_object_list_scroll_position {
                                *pack_object_list_scroll_position = *selected_pack_object;
                            }
                        } else {
                            // At the top of pack objects, switch focus to educational content
                            *focus = PackFocus::Educational;
                            *previous_focus = Some(PackColumnPreviousFocus::Educational);
                        }
                    }
                }
                MainNavigation::SelectFirstPackObject => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state:
                                    PreviewState::Pack(PackPreViewState {
                                        pack_object_list,
                                        pack_object_list_scroll_position,
                                        selected_pack_object,
                                        ..
                                    }),
                                ..
                            },
                        ..
                    } = &mut self.view
                        && !pack_object_list.is_empty()
                    {
                        *selected_pack_object = 0;
                        *pack_object_list_scroll_position = 0;
                    }
                }
                MainNavigation::SelectLastPackObject => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state:
                                    PreviewState::Pack(PackPreViewState {
                                        pack_object_list,
                                        pack_object_list_scroll_position,
                                        selected_pack_object,
                                        ..
                                    }),
                                ..
                            },
                        ..
                    } = &mut self.view
                        && !pack_object_list.is_empty()
                    {
                        *selected_pack_object = pack_object_list.len() - 1;
                        // Update scroll position to show the last item
                        let visible_height = self.layout_dimensions.pack_objects_height;
                        if *selected_pack_object >= visible_height {
                            *pack_object_list_scroll_position =
                                *selected_pack_object - visible_height + 1;
                        } else {
                            *pack_object_list_scroll_position = 0;
                        }
                    }
                }
                MainNavigation::ScrollEducationalUp => {
                    if let AppView::Main {
                        state: MainViewState { preview_state, .. },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(_) => {}
                            PreviewState::Pack(PackPreViewState {
                                educational_scroll_position,
                                ..
                            }) => {
                                if *educational_scroll_position > 0 {
                                    *educational_scroll_position -= 1;
                                }
                            }
                        }
                    }
                }
                MainNavigation::ScrollEducationalDown => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state,
                                educational_content,
                                ..
                            },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(_) => {}
                            PreviewState::Pack(PackPreViewState {
                                educational_scroll_position,
                                focus,
                                previous_focus,
                                ..
                            }) => {
                                // Calculate maximum scroll position based on content
                                let content_lines = educational_content.lines.len();
                                let visible_height =
                                    self.layout_dimensions.educational_content_height;
                                let max_scroll = content_lines.saturating_sub(visible_height);

                                if *educational_scroll_position < max_scroll {
                                    *educational_scroll_position += 1;
                                } else {
                                    *focus = PackFocus::PackObjectsList;
                                    *previous_focus =
                                        Some(PackColumnPreviousFocus::PackObjectsList);
                                }
                            }
                        }
                    }
                }
                MainNavigation::ScrollEducationalToTop => {
                    if let AppView::Main {
                        state: MainViewState { preview_state, .. },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(_) => {}
                            PreviewState::Pack(PackPreViewState {
                                educational_scroll_position,
                                ..
                            }) => {
                                *educational_scroll_position = 0;
                            }
                        }
                    }
                }
                MainNavigation::ScrollEducationalToBottom => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                preview_state,
                                educational_content,
                                ..
                            },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(_) => {}
                            PreviewState::Pack(PackPreViewState {
                                educational_scroll_position,
                                ..
                            }) => {
                                let content_lines = educational_content.lines.len();
                                let visible_height =
                                    self.layout_dimensions.educational_content_height;
                                *educational_scroll_position =
                                    content_lines.saturating_sub(visible_height);
                            }
                        }
                    }
                }
                MainNavigation::ScrollPreviewUp => {
                    if let AppView::Main {
                        state: MainViewState { preview_state, .. },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(RegularPreViewState {
                                preview_scroll_position,
                                ..
                            }) => {
                                if *preview_scroll_position > 0 {
                                    *preview_scroll_position -= 1;
                                }
                            }
                            PreviewState::Pack(PackPreViewState {
                                pack_object_widget_state,
                                ..
                            }) => {
                                pack_object_widget_state.scroll_up();
                            }
                        }
                    }
                }
                MainNavigation::ScrollPreviewDown => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                educational_content,
                                preview_state,
                                ..
                            },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(RegularPreViewState {
                                preview_scroll_position,
                                ..
                            }) => {
                                let content_lines = educational_content.lines.len();
                                let visible_height =
                                    self.layout_dimensions.educational_content_height;
                                let max_scroll = content_lines.saturating_sub(visible_height);

                                if *preview_scroll_position < max_scroll {
                                    *preview_scroll_position += 1;
                                }
                            }
                            PreviewState::Pack(PackPreViewState {
                                pack_object_list,
                                selected_pack_object,
                                pack_object_widget_state,
                                ..
                            }) => {
                                // For pack object detail scrolling, we need to calculate max scroll based on the content
                                if !pack_object_list.is_empty()
                                    && *selected_pack_object < pack_object_list.len()
                                {
                                    pack_object_widget_state.scroll_down();
                                }
                            }
                        }
                    }
                }
                MainNavigation::ScrollPreviewToTop => {
                    if let AppView::Main {
                        state: MainViewState { preview_state, .. },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(RegularPreViewState {
                                preview_scroll_position,
                                ..
                            }) => {
                                *preview_scroll_position = 0;
                            }
                            PreviewState::Pack(PackPreViewState {
                                pack_object_widget_state,
                                ..
                            }) => {
                                pack_object_widget_state.scroll_to_top();
                            }
                        }
                    }
                }
                MainNavigation::ScrollPreviewToBottom => {
                    if let AppView::Main {
                        state:
                            MainViewState {
                                educational_content,
                                preview_state,
                                ..
                            },
                        ..
                    } = &mut self.view
                    {
                        match preview_state {
                            PreviewState::Regular(RegularPreViewState {
                                preview_scroll_position,
                                ..
                            }) => {
                                let content_lines = educational_content.lines.len();
                                let visible_height =
                                    self.layout_dimensions.educational_content_height;
                                let max_scroll = content_lines.saturating_sub(visible_height);
                                *preview_scroll_position = max_scroll;
                            }
                            PreviewState::Pack(PackPreViewState {
                                pack_object_widget_state,
                                ..
                            }) => {
                                pack_object_widget_state.scroll_to_bottom();
                            }
                        }
                    }
                }
            },

            Message::OpenPackView => {
                if let AppView::Main {
                    state:
                        MainViewState {
                            preview_state:
                                PreviewState::Pack(PackPreViewState {
                                    pack_object_widget_state: pack_widget_state,
                                    ..
                                }),
                            ..
                        },
                } = &self.view
                {
                    // Create the new pack view
                    let pack_view = AppView::PackObjectDetail {
                        state: PackViewState {
                            pack_widget: pack_widget_state.clone(),
                        },
                    };

                    // Push current view onto stack and transition to pack view
                    self.push_view(pack_view);
                }
            }
            Message::OpenLooseObjectView => {
                if let AppView::Main {
                    state: MainViewState { git_objects, .. },
                } = &self.view
                {
                    // Get the currently selected loose object
                    if let Some((_, git_object, _)) =
                        git_objects.flat_view.get(git_objects.selected_index)
                        && let GitObjectType::LooseObject {
                            parsed_object: Some(loose_obj),
                            ..
                        } = &git_object.obj_type
                    {
                        // Create the new loose object view
                        let loose_view = AppView::LooseObjectDetail {
                            state: LooseObjectViewState {
                                loose_widget: LooseObjectWidget::new(loose_obj.clone()),
                            },
                        };

                        // Push current view onto stack and transition to loose object view
                        self.push_view(loose_view);
                    }
                }
            }
            Message::OpenMainView => {
                // Pop the previous view from the stack to restore state
                if !self.pop_view() {
                    // Fallback: if no previous view in stack, create a new main view
                    // This should rarely happen, but provides a safety net
                    let main_view_state = MainViewState::new(&self.educational_content_provider);
                    self.view = AppView::Main {
                        state: main_view_state,
                    };

                    // Reload git objects to restore basic functionality
                    let objects_msg = self.load_git_objects(plumber);
                    self.update(objects_msg, plumber);
                }
            }
            _ => {
                unreachable!("handle_main_view_mode_message called with non-main-view message")
            }
        }
        true
    }

    // Handle transition between PreviewState::Regular and PreviewState::Pack
    fn handle_preview_state_transition(&mut self, selected_index: usize, is_pack: bool) {
        if let AppView::Main { state } = &mut self.view {
            if is_pack {
                // Ensure we have a Pack preview state
                if let Some((_, git_object, _)) = state.git_objects.flat_view.get(selected_index)
                    && let GitObjectType::Pack { path, .. } = &git_object.obj_type
                {
                    match &state.preview_state {
                        PreviewState::Pack(pack_state) if pack_state.pack_file_path == *path => {
                            // Same pack file, keep existing state
                        }
                        _ => {
                            // Different pack file or not a pack state - create new pack state
                            let new_pack_state = PackPreViewState {
                                pack_file_path: path.clone(),
                                pack_object_list: Vec::new(),
                                selected_pack_object: 0,
                                pack_object_list_scroll_position: 0,
                                focus: PackFocus::GitObjects,
                                previous_focus: None,
                                educational_scroll_position: 0,
                                pack_object_widget_state: PackObjectWidget::Uninitiolized,
                            };
                            state.preview_state = PreviewState::Pack(new_pack_state);
                        }
                    }
                }
            } else {
                // Ensure we have a Regular preview state
                if !matches!(state.preview_state, PreviewState::Regular(_)) {
                    let new_regular_state = RegularPreViewState {
                        focus: RegularFocus::GitObjects,
                        preview_scroll_position: 0,
                    };
                    state.preview_state = PreviewState::Regular(new_regular_state);
                }
            }
        }
    }
}
