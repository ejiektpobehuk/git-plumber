use crate::educational_content::EducationalContent;
use crate::tui::message::Message;
use crate::tui::model::GitObject;
use crate::tui::model::GitObjectType;
use crate::tui::model::PackObject;
use ratatui::text::Text;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackFocus {
    GitObjects,
    Educational,
    PackObjectsList,
    PackObjectDetails,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackColumnPreviousFocus {
    Educational,
    PackObjectsList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegularFocus {
    GitObjects,
    Preview,
}

#[derive(Debug, Clone)]
pub enum PreviewState {
    Pack(PackPreViewState),
    Regular(RegularPreViewState),
}

#[derive(Debug, Clone)]
pub struct PackPreViewState {
    pub pack_file_path: PathBuf,
    pub pack_object_list: Vec<PackObject>,
    pub selected_pack_object: usize,
    pub pack_object_list_scroll_position: usize,
    pub focus: PackFocus,
    pub previous_focus: Option<PackColumnPreviousFocus>,
    pub educational_scroll_position: usize,
    pub pack_object_preview_scroll_position: usize,
    pub pack_object_detail_max_scroll: usize,
    pub pack_object_text_cache: Option<(ratatui::text::Text<'static>, usize)>, // (content, line_count)
}

#[derive(Debug, Clone)]
pub struct RegularPreViewState {
    pub focus: RegularFocus,
    pub preview_scroll_position: usize,
}

impl RegularPreViewState {
    fn new() -> Self {
        Self {
            focus: RegularFocus::GitObjects,
            preview_scroll_position: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitObjectsState {
    pub list: Vec<GitObject>,
    pub flat_view: Vec<(usize, GitObject)>,
    pub scroll_position: usize,
    pub selected_index: usize,
}

impl GitObjectsState {
    fn new() -> Self {
        Self {
            list: Vec::new(),
            flat_view: Vec::new(),
            scroll_position: 0,
            selected_index: 0,
        }
    }
}

pub struct MainViewState {
    pub git_objects: GitObjectsState,
    pub git_object_info: String,
    pub preview_state: PreviewState,
    pub educational_content: Text<'static>,
}

impl MainViewState {
    pub fn new(ed_provider: &EducationalContent) -> Self {
        Self {
            git_objects: GitObjectsState::new(),
            git_object_info: String::new(),
            educational_content: ed_provider.get_default_content(),
            preview_state: PreviewState::Regular(RegularPreViewState::new()),
        }
    }

    // Flatten the tree for display
    pub fn flatten_tree(&mut self) {
        self.git_objects.flat_view.clear();

        // Clone the objects first to avoid borrowing issues
        let list_clone = self.git_objects.list.clone();

        // Add each top-level object
        for obj in &list_clone {
            self.flatten_node_recursive(obj, 0);
        }
    }

    // Recursive helper to flatten a node and its children
    fn flatten_node_recursive(&mut self, node: &GitObject, depth: usize) {
        self.git_objects.flat_view.push((depth, node.clone()));

        if node.expanded {
            for child in &node.children {
                self.flatten_node_recursive(child, depth + 1);
            }
        }
    }

    pub fn are_git_objects_focused(&self) -> bool {
        match &self.preview_state {
            PreviewState::Pack(state) => state.focus == PackFocus::GitObjects,
            PreviewState::Regular(state) => state.focus == RegularFocus::GitObjects,
        }
    }

    pub fn update_git_objects_scroll_for_selection(
        &mut self,
        visible_height: usize,
        new_index: usize,
    ) {
        if new_index >= self.git_objects.scroll_position + visible_height {
            self.git_objects.scroll_position = new_index.saturating_sub(visible_height - 1);
        } else if new_index < self.git_objects.scroll_position {
            self.git_objects.scroll_position = new_index;
        }
    }

    // Toggle expansion state of the selected node
    pub fn toggle_expand(&mut self) -> Message {
        if self.git_objects.selected_index < self.git_objects.flat_view.len() {
            let (_, selected_obj) =
                &self.git_objects.flat_view[self.git_objects.selected_index].clone();

            // Only toggle expand for Category type objects
            if let GitObjectType::Category(category_name) = &selected_obj.obj_type {
                // Store the name for re-selection later
                let name_to_find = category_name.clone();

                // Find and toggle a category
                fn find_and_toggle_category(obj: &mut GitObject, target_name: &str) -> bool {
                    if let GitObjectType::Category(name) = &obj.obj_type {
                        if name == target_name {
                            obj.expanded = !obj.expanded;
                            return true;
                        }
                    }

                    for child in &mut obj.children {
                        if find_and_toggle_category(child, target_name) {
                            return true;
                        }
                    }
                    false
                }

                // Search through all top-level objects
                for obj in &mut self.git_objects.list {
                    if find_and_toggle_category(obj, &name_to_find) {
                        break;
                    }
                }

                // Rebuild the flattened view
                self.flatten_tree();

                // Try to keep the selected item visible
                let mut new_index = 0;
                for (i, (_, obj)) in self.git_objects.flat_view.iter().enumerate() {
                    if let GitObjectType::Category(name) = &obj.obj_type {
                        if name == &name_to_find {
                            new_index = i;
                            break;
                        }
                    }
                }
                self.git_objects.selected_index =
                    new_index.min(self.git_objects.flat_view.len() - 1);
            }
        }
        Message::LoadGitObjects(Ok(()))
    }
}
