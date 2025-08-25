use crate::educational_content::EducationalContent;
use crate::tui::message::Message;
use crate::tui::model::{GitObject, GitObjectType, PackObject};
use crate::tui::widget::{PackIndexWidget, PackObjectWidget};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::animations::AnimationManager;
use super::services::ServiceContainer;
use super::state_components::{ContentState, SessionState, TreeState};

// ===== Ghost overlay types =====

#[derive(Debug, Clone)]
pub struct Ghost {
    pub until: Instant,
    pub parent_key: Option<String>,
    pub sibling_index: usize,
    pub display: GitObject,
}

#[derive(Debug, Clone)]
pub struct OldPosition {
    pub parent_key: Option<String>,
    pub sibling_index: usize,
}

// ===== View state types =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStatus {
    Normal,
    PendingRemoval,
}

#[derive(Debug, Clone, Default)]
pub struct HighlightInfo {
    pub color: Option<ratatui::style::Color>,
    pub expires_at: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub struct FlatTreeRow {
    pub depth: usize,
    pub object: GitObject,
    pub render_status: RenderStatus,
    pub highlight: HighlightInfo,
}

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
    pub pack_object_widget_state: PackObjectWidget,
    pub educational_scroll_position: usize,
}

#[derive(Debug, Clone)]
pub struct RegularPreViewState {
    pub focus: RegularFocus,
    pub preview_scroll_position: usize,
    pub pack_index_widget: Option<PackIndexWidget>,
}

impl Default for RegularPreViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl RegularPreViewState {
    pub const fn new() -> Self {
        Self {
            focus: RegularFocus::GitObjects,
            preview_scroll_position: 0,
            pack_index_widget: None,
        }
    }

    pub fn new_with_pack_index(pack_index: crate::git::pack::PackIndex) -> Self {
        Self {
            focus: RegularFocus::GitObjects,
            preview_scroll_position: 0,
            pack_index_widget: Some(PackIndexWidget::new(pack_index)),
        }
    }
}

impl MainViewState {
    /// Cleanup timers; returns true if anything changed
    pub fn prune_timeouts(&mut self) -> bool {
        self.animations.prune_timeouts()
    }
}

pub struct MainViewState {
    // Core state components (clean architecture)
    pub tree: TreeState,
    pub content: ContentState,
    pub session: SessionState,

    // UI and interaction state
    pub preview_state: PreviewState,
    pub animations: AnimationManager,

    // Domain services
    pub services: ServiceContainer,
}

#[derive(Debug, Clone)]
pub struct SelectionIdentity {
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct ScrollSnapshot {
    pub git_list_scroll: usize,
    pub preview_scroll: usize,
    pub pack_list_scroll: usize,
}

impl MainViewState {
    pub fn new(ed_provider: &EducationalContent) -> Self {
        Self {
            tree: TreeState::new(),
            content: ContentState::new(ed_provider),
            session: SessionState::new(),
            preview_state: PreviewState::Regular(RegularPreViewState::new()),
            animations: AnimationManager::new(),
            services: ServiceContainer::new(),
        }
    }

    // ===== Selection key (stable identity) =====

    pub fn selection_key(obj: &GitObject) -> String {
        match &obj.obj_type {
            GitObjectType::Category(name) => format!("category:{name}"),
            GitObjectType::FileSystemFolder { path, .. } => format!("folder:{}", path.display()),
            GitObjectType::FileSystemFile { path, .. } => format!("file:{}", path.display()),

            GitObjectType::PackFolder { base_name, .. } => format!("pack_folder:{base_name}"),
            GitObjectType::PackFile {
                file_type, path, ..
            } => format!("pack_file:{}:{}", file_type, path.display()),
            GitObjectType::Ref { path, .. } => format!("ref:{}", path.display()),
            GitObjectType::LooseObject { object_id, .. } => {
                format!("loose:{}", object_id.clone().unwrap_or_default())
            }
        }
    }

    pub fn current_selection_key(&self) -> Option<String> {
        if self.tree.selected_index < self.tree.flat_view.len() {
            let row = &self.tree.flat_view[self.tree.selected_index];
            Some(Self::selection_key(&row.object))
        } else {
            None
        }
    }

    // ===== Change detection (snapshot/compare) =====

    pub fn snapshot_old_positions(
        &self,
    ) -> (HashMap<String, OldPosition>, HashMap<String, GitObject>) {
        let snapshot = super::change_detection::ChangeDetectionService::snapshot_tree_positions(
            &self.tree.list,
            Self::selection_key,
        );
        (snapshot.positions, snapshot.nodes)
    }

    pub fn detect_tree_changes(
        &mut self,
        old_positions: &HashMap<String, OldPosition>,
        old_nodes: &HashMap<String, GitObject>,
    ) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
        // Use our change detection service to get the basic changes
        let new_keys = super::change_detection::ChangeDetectionService::collect_all_keys(
            &self.tree.list,
            Self::selection_key,
        );
        let old_keys: HashSet<String> = old_positions.keys().cloned().collect();

        let added_keys: HashSet<String> = new_keys.difference(&old_keys).cloned().collect();
        let deleted_keys: HashSet<String> = old_keys.difference(&new_keys).cloned().collect();

        // Detect modifications: same key exists but content differs
        let modified_keys: HashSet<String> = new_keys
            .intersection(&old_keys)
            .filter(|key| {
                if let Some(old_node) = old_nodes.get(*key) {
                    if let Some(new_node) = self.find_node_by_key(key) {
                        Self::is_object_modified_static(old_node, new_node)
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        let now = Instant::now();

        // Additions (green, 5s)
        self.animations
            .changed_keys
            .retain(|k, until| *until > now && new_keys.contains(k));
        for k in &added_keys {
            // If there was a ghost for this key (re-appeared), drop it
            self.animations.ghosts.remove(k);
            self.animations
                .changed_keys
                .insert(k.clone(), now + Duration::from_secs(5));
        }

        // Modifications (orange, 5s)
        self.animations
            .modified_keys
            .retain(|k, until| *until > now && new_keys.contains(k));
        for k in &modified_keys {
            self.animations
                .modified_keys
                .insert(k.clone(), now + Duration::from_secs(5));
        }

        // Deletions -> ghosts overlay (red, 5s)
        self.animations
            .ghosts
            .retain(|k, g| g.until > now && !new_keys.contains(k));
        let ghost_duration = Duration::from_secs(5);
        for k in &deleted_keys {
            if self.animations.ghosts.contains_key(k) {
                continue;
            }
            if let Some(old_node) = old_nodes.get(k) {
                let (parent_key, sibling_index) = if let Some(pos) = old_positions.get(k) {
                    (pos.parent_key.clone(), pos.sibling_index)
                } else {
                    (None, 0)
                };
                self.animations.ghosts.insert(
                    k.clone(),
                    Ghost {
                        until: now + ghost_duration,
                        parent_key,
                        sibling_index,
                        display: old_node.clone(),
                    },
                );
            }
        }

        (added_keys, deleted_keys, modified_keys)
    }

    // ===== Flatten + overlay =====

    pub fn flatten_tree(&mut self) {
        // Clean up expired ghosts first
        let now = Instant::now();
        self.animations.ghosts.retain(|_, g| g.until > now);

        // Use TreeService to flatten the tree with animations
        self.tree.flat_view = super::services::TreeService::flatten_tree_with_animations(
            &self.tree.list,
            &self.animations,
            Self::selection_key,
        );

        // Clean up expired animation timers
        self.animations.changed_keys.retain(|_, until| *until > now);
        self.animations
            .modified_keys
            .retain(|_, until| *until > now);
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
        self.tree.scroll_position =
            super::services::UIService::update_git_objects_scroll_for_selection(
                &self.tree.flat_view,
                new_index,
                self.tree.scroll_position,
                visible_height,
            );
    }

    // Toggle expansion for categories and filesystem folders
    pub fn toggle_expand(&mut self) -> Message {
        if self.tree.selected_index < self.tree.flat_view.len() {
            let selected_obj = &self.tree.flat_view[self.tree.selected_index].object.clone();

            match &selected_obj.obj_type {
                GitObjectType::Category(category_name) => {
                    let name_to_find = category_name.clone();

                    fn find_and_toggle_category(obj: &mut GitObject, target_name: &str) -> bool {
                        if let GitObjectType::Category(name) = &obj.obj_type
                            && name == target_name
                        {
                            obj.expanded = !obj.expanded;
                            return true;
                        }
                        for child in &mut obj.children {
                            if find_and_toggle_category(child, target_name) {
                                return true;
                            }
                        }
                        false
                    }

                    for obj in &mut self.tree.list {
                        if find_and_toggle_category(obj, &name_to_find) {
                            break;
                        }
                    }

                    self.flatten_tree();

                    // Keep the selected category visible
                    let mut new_index = 0;
                    for (i, row) in self.tree.flat_view.iter().enumerate() {
                        if let GitObjectType::Category(name) = &row.object.obj_type
                            && *name == name_to_find
                        {
                            new_index = i;
                            break;
                        }
                    }
                    if self.tree.flat_view.is_empty() {
                        self.tree.selected_index = 0;
                    } else {
                        self.tree.selected_index = new_index.min(self.tree.flat_view.len() - 1);
                    }
                }
                GitObjectType::FileSystemFolder { path, .. } => {
                    let path_to_find = path.clone();

                    fn find_and_toggle_folder(
                        obj: &mut GitObject,
                        target_path: &std::path::Path,
                    ) -> Result<bool, String> {
                        if let GitObjectType::FileSystemFolder {
                            path, is_loaded, ..
                        } = &mut obj.obj_type
                            && path == target_path
                        {
                            if !obj.expanded && !*is_loaded {
                                // Load folder contents before expanding
                                obj.load_folder_contents()?;
                            }
                            obj.expanded = !obj.expanded;
                            return Ok(true);
                        }
                        for child in &mut obj.children {
                            if find_and_toggle_folder(child, target_path)? {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }

                    let mut error_msg = None;
                    for obj in &mut self.tree.list {
                        match find_and_toggle_folder(obj, &path_to_find) {
                            Ok(true) => break,
                            Ok(false) => continue,
                            Err(e) => {
                                error_msg = Some(e);
                                break;
                            }
                        }
                    }

                    if let Some(error) = error_msg {
                        return Message::LoadGitObjects(Err(format!(
                            "Failed to expand folder: {error}"
                        )));
                    }

                    self.flatten_tree();

                    // Keep the selected folder visible
                    let mut new_index = 0;
                    for (i, row) in self.tree.flat_view.iter().enumerate() {
                        if let GitObjectType::FileSystemFolder { path, .. } = &row.object.obj_type
                            && *path == path_to_find
                        {
                            new_index = i;
                            break;
                        }
                    }
                    if self.tree.flat_view.is_empty() {
                        self.tree.selected_index = 0;
                    } else {
                        self.tree.selected_index = new_index.min(self.tree.flat_view.len() - 1);
                    }
                }
                GitObjectType::PackFolder { base_name, .. } => {
                    let base_name_to_find = base_name.clone();

                    fn find_and_toggle_pack_folder(
                        obj: &mut GitObject,
                        target_base_name: &str,
                    ) -> bool {
                        if let GitObjectType::PackFolder { base_name, .. } = &obj.obj_type
                            && base_name == target_base_name
                        {
                            obj.expanded = !obj.expanded;
                            return true;
                        }
                        for child in &mut obj.children {
                            if find_and_toggle_pack_folder(child, target_base_name) {
                                return true;
                            }
                        }
                        false
                    }

                    for obj in &mut self.tree.list {
                        if find_and_toggle_pack_folder(obj, &base_name_to_find) {
                            break;
                        }
                    }

                    self.flatten_tree();

                    // Keep the selected pack folder visible
                    let mut new_index = 0;
                    for (i, row) in self.tree.flat_view.iter().enumerate() {
                        if let GitObjectType::PackFolder { base_name, .. } = &row.object.obj_type
                            && *base_name == base_name_to_find
                        {
                            new_index = i;
                            break;
                        }
                    }
                    if self.tree.flat_view.is_empty() {
                        self.tree.selected_index = 0;
                    } else {
                        self.tree.selected_index = new_index.min(self.tree.flat_view.len() - 1);
                    }
                }
                _ => {} // Other object types are not expandable
            }
        }
        Message::LoadGitObjects(Ok(()))
    }

    // ===== Modification detection helpers =====

    /// Find a node in the current tree by its selection key
    pub fn find_node_by_key(&self, key: &str) -> Option<&GitObject> {
        super::services::TreeService::find_node_by_key(&self.tree.list, key, Self::selection_key)
    }

    /// Check if an object has been modified by comparing modification times
    pub fn is_object_modified(&self, old: &GitObject, new: &GitObject) -> bool {
        super::services::GitRepositoryService::is_object_modified(old, new)
    }

    /// Static version of `is_object_modified` for use in closures
    pub fn is_object_modified_static(old: &GitObject, new: &GitObject) -> bool {
        super::services::GitRepositoryService::is_object_modified_static(old, new)
    }
}
