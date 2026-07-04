use crate::educational_content::EducationalContent;
use crate::tui::message::Message;
use crate::tui::model::{GitObject, GitObjectType, PackObject};
use crate::tui::widget::{PackIndexWidget, PackObjectWidget, PackReverseIndexWidget};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::animations::AnimationManager;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationType {
    /// File animation: shrinking background highlight
    #[default]
    FileShrink,
    /// Folder animation: blinking indicator
    FolderBlink,
}

#[derive(Debug, Clone, Default)]
pub struct HighlightInfo {
    pub color: Option<ratatui::style::Color>,
    pub expires_at: Option<std::time::Instant>,
    pub animation_type: AnimationType,
}

/// Coarse classification of a tree node for rendering and navigation.
///
/// Carries only what those decisions need; the full `GitObject` stays in
/// `TreeState::list` and is looked up by `key` when detail data is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Category,
    Folder { is_educational: bool },
    PackFolder,
    File,
    PackFile { is_pack: bool },
    Ref,
    LooseObject,
}

impl RowKind {
    #[must_use]
    pub const fn is_folder(self) -> bool {
        matches!(
            self,
            Self::Category | Self::Folder { .. } | Self::PackFolder
        )
    }

    // Whether selecting this row should show the pack preview (mirrors
    // `GitObjectType::is_pack`)
    #[must_use]
    pub const fn is_pack(self) -> bool {
        matches!(self, Self::PackFolder | Self::PackFile { is_pack: true })
    }
}

/// A single visible row of the tree: a lightweight view-model built during
/// flattening.
///
/// It deliberately does NOT own a `GitObject` — cloning nodes (with their
/// recursive children and parsed payloads) per row made every re-flatten
/// O(nodes × depth) deep clones.
#[derive(Debug, Clone)]
pub struct FlatTreeRow {
    pub depth: usize,
    /// Stable selection key of the underlying node (`MainViewState::selection_key`)
    pub key: String,
    pub name: String,
    pub kind: RowKind,
    pub expanded: bool,
    pub is_empty: bool,
    /// True when no later sibling exists at this depth; drives └ vs ├
    pub is_last_child: bool,
    /// Bit `d` set means indent level `d` needs a │ guide (the ancestor at
    /// depth `d + 1` has later siblings)
    pub guides: u64,
    pub render_status: RenderStatus,
    pub highlight: HighlightInfo,
}

impl FlatTreeRow {
    /// Build a row from a tree node. `is_last_child` and `guides` are filled
    /// afterwards by `TreeFlattener::compute_row_relationships` once the full
    /// row list (including ghost rows) is known.
    #[must_use]
    pub fn from_node(
        node: &GitObject,
        depth: usize,
        key: String,
        render_status: RenderStatus,
        highlight: HighlightInfo,
    ) -> Self {
        let kind = match &node.obj_type {
            GitObjectType::Category(_) => RowKind::Category,
            GitObjectType::FileSystemFolder { is_educational, .. } => RowKind::Folder {
                is_educational: *is_educational,
            },
            GitObjectType::PackFolder { .. } => RowKind::PackFolder,
            GitObjectType::FileSystemFile { .. } => RowKind::File,
            GitObjectType::PackFile { file_type, .. } => RowKind::PackFile {
                is_pack: file_type == "packfile" || file_type == "pack",
            },
            GitObjectType::Ref { .. } => RowKind::Ref,
            GitObjectType::LooseObject { .. } => RowKind::LooseObject,
        };
        Self {
            depth,
            key,
            name: node.name.clone(),
            kind,
            expanded: node.expanded,
            is_empty: node.is_empty(),
            is_last_child: false,
            guides: 0,
            render_status,
            highlight,
        }
    }
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
    pub pack_reverse_index_widget: Option<PackReverseIndexWidget>,
}

impl Default for RegularPreViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl RegularPreViewState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            focus: RegularFocus::GitObjects,
            preview_scroll_position: 0,
            pack_index_widget: None,
            pack_reverse_index_widget: None,
        }
    }

    #[must_use]
    pub fn new_with_pack_index(pack_index: crate::git::pack::PackIndex) -> Self {
        Self {
            focus: RegularFocus::GitObjects,
            preview_scroll_position: 0,
            pack_index_widget: Some(PackIndexWidget::new(pack_index)),
            pack_reverse_index_widget: None,
        }
    }

    #[must_use]
    pub fn new_with_pack_reverse_index(reverse_index: crate::git::pack::PackReverseIndex) -> Self {
        Self {
            focus: RegularFocus::GitObjects,
            preview_scroll_position: 0,
            pack_index_widget: None,
            pack_reverse_index_widget: Some(PackReverseIndexWidget::new(reverse_index)),
        }
    }
}

impl MainViewState {
    /// Cleanup timers; returns true when the flattened tree needs a rebuild
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
    #[must_use]
    pub fn new(ed_provider: &EducationalContent) -> Self {
        Self {
            tree: TreeState::new(),
            content: ContentState::new(ed_provider),
            session: SessionState::new(),
            preview_state: PreviewState::Regular(RegularPreViewState::new()),
            animations: AnimationManager::new(),
        }
    }

    // ===== Selection key (stable identity) =====

    #[must_use]
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

    #[must_use]
    pub fn current_selection_key(&self) -> Option<String> {
        self.tree
            .flat_view
            .get(self.tree.selected_index)
            .map(|row| row.key.clone())
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
        animation_duration_secs: u64,
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
                old_nodes.get(*key).is_some_and(|old_node| {
                    self.find_node_by_key(key)
                        .is_some_and(|new_node| Self::is_object_modified_static(old_node, new_node))
                })
            })
            .cloned()
            .collect();

        let now = Instant::now();

        // Additions - green
        self.animations
            .changed_keys
            .retain(|k, until| *until > now && new_keys.contains(k));
        for k in &added_keys {
            // If there was a ghost for this key (re-appeared), drop it
            self.animations.ghosts.remove(k);
            self.animations.changed_keys.insert(
                k.clone(),
                now + Duration::from_secs(animation_duration_secs),
            );
        }

        // Modifications - orange, 10s
        self.animations
            .modified_keys
            .retain(|k, until| *until > now && new_keys.contains(k));
        for k in &modified_keys {
            self.animations.modified_keys.insert(
                k.clone(),
                now + Duration::from_secs(animation_duration_secs),
            );
        }

        // Deletions -> ghosts overlay - red
        self.animations
            .ghosts
            .retain(|k, g| g.until > now && !new_keys.contains(k));
        let ghost_duration = Duration::from_secs(animation_duration_secs);
        for k in &deleted_keys {
            if self.animations.ghosts.contains_key(k) {
                continue;
            }
            if let Some(old_node) = old_nodes.get(k) {
                let (parent_key, sibling_index) = old_positions
                    .get(k)
                    .map_or((None, 0), |pos| (pos.parent_key.clone(), pos.sibling_index));
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

        // Use new pre-computed highlighting for perfect alignment
        self.tree.flat_view =
            super::services::TreeService::flatten_tree_with_precomputed_highlights(
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

    #[must_use]
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

    // Toggle expansion for categories, filesystem folders and pack folders
    pub fn toggle_expand(&mut self, visible_height: usize) -> Message {
        let Some(row) = self.tree.flat_view.get(self.tree.selected_index) else {
            return Message::LoadGitObjects(Ok(()));
        };
        if !row.kind.is_folder() {
            return Message::LoadGitObjects(Ok(())); // Other object types are not expandable
        }
        let key = row.key.clone();

        let Some(node) = super::services::TreeService::find_node_by_key_mut(
            &mut self.tree.list,
            &key,
            Self::selection_key,
        ) else {
            // Ghost rows have no backing node; nothing to toggle
            return Message::LoadGitObjects(Ok(()));
        };
        if !node.expanded
            && matches!(
                node.obj_type,
                GitObjectType::FileSystemFolder {
                    is_loaded: false,
                    ..
                }
            )
            && let Err(e) = node.load_folder_contents()
        {
            return Message::LoadGitObjects(Err(format!("Failed to expand folder: {e}")));
        }
        node.expanded = !node.expanded;

        // Remember the visual position of the toggled node before flattening
        let old_visual_position = self
            .tree
            .selected_index
            .saturating_sub(self.tree.scroll_position);

        self.flatten_tree();

        if self.tree.flat_view.is_empty() {
            self.tree.selected_index = 0;
            self.tree.scroll_position = 0;
        } else {
            // Keep the toggled node selected
            let new_index = self
                .tree
                .flat_view
                .iter()
                .position(|r| r.key == key)
                .unwrap_or(0);
            self.tree.selected_index = new_index.min(self.tree.flat_view.len() - 1);

            // Preserve visual position: try to keep the toggled item at the same visual offset
            let desired_scroll = new_index.saturating_sub(old_visual_position);
            self.tree.scroll_position = super::services::UIService::clamp_scroll_position(
                &self.tree.flat_view,
                desired_scroll,
                visible_height,
            );
        }
        Message::LoadGitObjects(Ok(()))
    }

    // ===== Modification detection helpers =====

    /// Find a node in the current tree by its selection key
    pub fn find_node_by_key(&self, key: &str) -> Option<&GitObject> {
        super::services::TreeService::find_node_by_key(&self.tree.list, key, Self::selection_key)
    }

    /// Resolve the currently selected row back to its tree node.
    /// Returns None for ghost rows (pending removal), which have no backing node.
    #[must_use]
    pub fn selected_node(&self) -> Option<&GitObject> {
        let row = self.tree.flat_view.get(self.tree.selected_index)?;
        self.find_node_by_key(&row.key)
    }

    /// Static version of `is_object_modified` for use in closures
    #[must_use]
    pub fn is_object_modified_static(old: &GitObject, new: &GitObject) -> bool {
        super::services::GitRepositoryService::is_object_modified_static(old, new)
    }
}
