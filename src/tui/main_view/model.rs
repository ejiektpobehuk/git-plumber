use crate::educational_content::EducationalContent;
use crate::tui::message::Message;
use crate::tui::model::{GitObject, GitObjectType, PackObject};
use crate::tui::widget::PackObjectWidget;
use ratatui::text::Text;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ===== Natural sort helpers (module scope) =====

#[derive(Debug, Clone, PartialEq, Eq)]
enum NatPart {
    Str(String),
    Num(u128),
}

impl Ord for NatPart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use NatPart::*;
        match (self, other) {
            (Str(a), Str(b)) => a.cmp(b),
            (Num(a), Num(b)) => a.cmp(b),
            (Str(_), Num(_)) => std::cmp::Ordering::Less,
            (Num(_), Str(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for NatPart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn natural_key(s: &str) -> Vec<NatPart> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut is_digit: Option<bool> = None;

    for ch in s.chars() {
        let d = ch.is_ascii_digit();
        match is_digit {
            None => {
                is_digit = Some(d);
                buf.push(ch.to_ascii_lowercase());
            }
            Some(prev) if prev == d => {
                buf.push(ch.to_ascii_lowercase());
            }
            Some(_) => {
                if let Some(true) = is_digit {
                    parts.push(NatPart::Num(buf.parse::<u128>().unwrap_or(0)));
                } else {
                    parts.push(NatPart::Str(buf.clone()));
                }
                buf.clear();
                is_digit = Some(d);
                buf.push(ch.to_ascii_lowercase());
            }
        }
    }
    if !buf.is_empty() {
        if let Some(true) = is_digit {
            parts.push(NatPart::Num(buf.parse::<u128>().unwrap_or(0)));
        } else {
            parts.push(NatPart::Str(buf));
        }
    }
    parts
}

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
    pub flat_view: Vec<(usize, GitObject, RenderStatus)>,
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

impl MainViewState {
    /// Cleanup timers; returns true if anything changed
    pub fn prune_timeouts(&mut self) -> bool {
        let now = Instant::now();
        let before_ghosts = self.ghosts.len();
        self.ghosts.retain(|_, g| g.until > now);
        let ghosts_changed = before_ghosts != self.ghosts.len();

        let before_changed = self.changed_keys.len();
        self.changed_keys.retain(|_, until| *until > now);
        let changed_changed = before_changed != self.changed_keys.len();

        let before_modified = self.modified_keys.len();
        self.modified_keys.retain(|_, until| *until > now);
        let modified_changed = before_modified != self.modified_keys.len();

        ghosts_changed || changed_changed || modified_changed
    }
}

pub struct MainViewState {
    pub git_objects: GitObjectsState,
    pub git_object_info: String,
    pub preview_state: PreviewState,
    pub educational_content: Text<'static>,
    // Live update persistence
    pub last_selection: Option<SelectionIdentity>,
    pub last_scroll_positions: Option<ScrollSnapshot>,
    // Additions highlighting: per-item timers
    pub changed_keys: HashMap<String, Instant>,
    // Modifications highlighting: per-item timers
    pub modified_keys: HashMap<String, Instant>,
    // Deleted item overlay (red background), not mutating the tree
    pub ghosts: HashMap<String, Ghost>,
    // First-load guard
    pub has_loaded_once: bool,
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
            git_objects: GitObjectsState::new(),
            git_object_info: String::new(),
            educational_content: ed_provider.get_default_content(),
            preview_state: PreviewState::Regular(RegularPreViewState::new()),
            last_selection: None,
            last_scroll_positions: None,
            changed_keys: HashMap::new(),
            modified_keys: HashMap::new(),
            ghosts: HashMap::new(),
            has_loaded_once: false,
        }
    }

    // ===== Selection key (stable identity) =====

    pub fn selection_key(obj: &GitObject) -> String {
        match &obj.obj_type {
            GitObjectType::Category(name) => format!("category:{name}"),
            GitObjectType::Pack { path, .. } => format!("pack:{}", path.display()),
            GitObjectType::Ref { path, .. } => format!("ref:{}", path.display()),
            GitObjectType::LooseObject { object_id, .. } => {
                format!("loose:{}", object_id.clone().unwrap_or_default())
            }
        }
    }

    pub fn current_selection_key(&self) -> Option<String> {
        if self.git_objects.selected_index < self.git_objects.flat_view.len() {
            let (_, obj, _) = &self.git_objects.flat_view[self.git_objects.selected_index];
            Some(Self::selection_key(obj))
        } else {
            None
        }
    }

    // ===== Change detection (snapshot/compare) =====

    pub fn snapshot_old_positions(
        &self,
    ) -> (HashMap<String, OldPosition>, HashMap<String, GitObject>) {
        fn walk(
            out_pos: &mut HashMap<String, OldPosition>,
            out_nodes: &mut HashMap<String, GitObject>,
            children: &[GitObject],
            parent_key: Option<String>,
        ) {
            for (idx, child) in children.iter().enumerate() {
                let key = MainViewState::selection_key(child);
                out_pos.insert(
                    key.clone(),
                    OldPosition {
                        parent_key: parent_key.clone(),
                        sibling_index: idx,
                    },
                );
                out_nodes.insert(key.clone(), child.clone());
                if let GitObjectType::Category(_) = child.obj_type {
                    walk(out_pos, out_nodes, &child.children, Some(key));
                }
            }
        }

        let mut positions = HashMap::new();
        let mut nodes = HashMap::new();
        walk(&mut positions, &mut nodes, &self.git_objects.list, None);
        (positions, nodes)
    }

    fn collect_all_keys(&self) -> HashSet<String> {
        fn walk_keys(children: &[GitObject], acc: &mut HashSet<String>) {
            for child in children {
                let key = MainViewState::selection_key(child);
                acc.insert(key);
                if let GitObjectType::Category(_) = child.obj_type {
                    walk_keys(&child.children, acc);
                }
            }
        }
        let mut keys = HashSet::new();
        walk_keys(&self.git_objects.list, &mut keys);
        keys
    }

    pub fn detect_tree_changes(
        &mut self,
        old_positions: &HashMap<String, OldPosition>,
        old_nodes: &HashMap<String, GitObject>,
    ) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
        let new_keys = self.collect_all_keys();
        let old_keys: HashSet<String> = old_positions.keys().cloned().collect();

        let added_keys: HashSet<String> = new_keys.difference(&old_keys).cloned().collect();
        let deleted_keys: HashSet<String> = old_keys.difference(&new_keys).cloned().collect();

        // Detect modifications: same key exists but content differs
        let modified_keys: HashSet<String> = new_keys
            .intersection(&old_keys)
            .filter(|key| {
                if let Some(old_node) = old_nodes.get(*key) {
                    if let Some(new_node) = self.find_node_by_key(key) {
                        self.is_object_modified(old_node, new_node)
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
        self.changed_keys
            .retain(|k, until| *until > now && new_keys.contains(k));
        for k in &added_keys {
            // If there was a ghost for this key (re-appeared), drop it
            self.ghosts.remove(k);
            self.changed_keys
                .insert(k.clone(), now + Duration::from_secs(5));
        }

        // Modifications (orange, 5s)
        self.modified_keys
            .retain(|k, until| *until > now && new_keys.contains(k));
        for k in &modified_keys {
            self.modified_keys
                .insert(k.clone(), now + Duration::from_secs(5));
        }

        // Deletions -> ghosts overlay (red, 5s)
        self.ghosts
            .retain(|k, g| g.until > now && !new_keys.contains(k));
        let ghost_duration = Duration::from_secs(5);
        for k in &deleted_keys {
            if self.ghosts.contains_key(k) {
                continue;
            }
            if let Some(old_node) = old_nodes.get(k) {
                let (parent_key, sibling_index) = if let Some(pos) = old_positions.get(k) {
                    (pos.parent_key.clone(), pos.sibling_index)
                } else {
                    (None, 0)
                };
                self.ghosts.insert(
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

    // ===== Sorting (natural sort for categories, except "objects") =====

    pub fn sort_tree_for_display(nodes: &mut [GitObject]) {
        for node in nodes.iter_mut() {
            if let GitObjectType::Category(ref name) = node.obj_type {
                if name != "objects" {
                    node.children
                        .sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
                }
                MainViewState::sort_tree_for_display(&mut node.children);
            }
        }
    }

    // ===== Flatten + overlay =====

    pub fn flatten_tree(&mut self) {
        self.git_objects.flat_view.clear();
        self.git_objects.flat_view.reserve(16);

        // Clone to avoid borrow issues while recursing
        let list_clone = self.git_objects.list.clone();
        for obj in &list_clone {
            self.flatten_node_recursive(obj, 0);
        }

        // Interleave ghosts overlay based on stored positions
        let now = Instant::now();
        self.ghosts.retain(|_, g| g.until > now);
        if !self.ghosts.is_empty() {
            // Group ghosts by parent_key
            let mut by_parent: HashMap<Option<String>, Vec<(usize, String)>> = HashMap::new();
            for (k, g) in &self.ghosts {
                by_parent
                    .entry(g.parent_key.clone())
                    .or_default()
                    .push((g.sibling_index, k.clone()));
            }
            for v in by_parent.values_mut() {
                v.sort_by_key(|(idx, _)| *idx);
            }

            let mut output: Vec<(usize, GitObject, RenderStatus)> =
                self.git_objects.flat_view.clone();

            // Top-level ghosts: precise mapping by sibling_index against top-level order
            if let Some(top_list) = by_parent.get(&None) {
                let top_keys: Vec<String> = self
                    .git_objects
                    .list
                    .iter()
                    .map(MainViewState::selection_key)
                    .collect();

                let find_top_flat_index =
                    |key: &str, flat: &[(usize, GitObject, RenderStatus)]| -> Option<usize> {
                        flat.iter()
                            .position(|(d, o, _)| *d == 0 && MainViewState::selection_key(o) == key)
                    };

                let end_of_top_level = {
                    let last_top_idx = output
                        .iter()
                        .enumerate()
                        .filter(|(_, (d, _, _))| *d == 0)
                        .map(|(i, _)| i)
                        .next_back();
                    match last_top_idx {
                        Some(i) => {
                            let mut j = i + 1;
                            while j < output.len() {
                                if output[j].0 == 0 {
                                    break;
                                }
                                j += 1;
                            }
                            j
                        }
                        None => 0,
                    }
                };

                for (sibling_index, ghost_key) in top_list.iter().rev() {
                    if let Some(g) = self.ghosts.get(ghost_key) {
                        let insert_at = if *sibling_index < top_keys.len() {
                            if let Some(idx) =
                                find_top_flat_index(&top_keys[*sibling_index], &output)
                            {
                                idx
                            } else {
                                end_of_top_level
                            }
                        } else {
                            end_of_top_level
                        };
                        output.insert(
                            insert_at,
                            (0, g.display.clone(), RenderStatus::PendingRemoval),
                        );
                    }
                }
            }

            // Nested ghosts: if parent expanded, place among visible children at sibling_index;
            // otherwise place right after parent row
            for (parent, list) in by_parent.into_iter() {
                if parent.is_none() {
                    continue;
                }
                if let Some(parent_key) = parent
                    && let Some(parent_row) = output
                        .iter()
                        .position(|(_, o, _)| Self::selection_key(o) == parent_key)
                {
                    let parent_depth = output[parent_row].0;
                    let parent_expanded =
                        if let GitObjectType::Category(_) = output[parent_row].1.obj_type {
                            output[parent_row].1.expanded
                        } else {
                            false
                        };

                    let mut child_rows: Vec<usize> = Vec::new();
                    if parent_expanded {
                        let mut i = parent_row + 1;
                        while i < output.len() {
                            let (d, _, _) = &output[i];
                            if *d <= parent_depth {
                                break;
                            }
                            if *d == parent_depth + 1 {
                                child_rows.push(i);
                            }
                            i += 1;
                        }
                    }

                    for (sibling_index, ghost_key) in list.into_iter().rev() {
                        if let Some(g) = self.ghosts.get(&ghost_key) {
                            let insert_at = if parent_expanded {
                                if sibling_index < child_rows.len() {
                                    child_rows[sibling_index]
                                } else {
                                    child_rows.last().map(|x| x + 1).unwrap_or(parent_row + 1)
                                }
                            } else {
                                parent_row + 1
                            };
                            output.insert(
                                insert_at,
                                (
                                    parent_depth + 1,
                                    g.display.clone(),
                                    RenderStatus::PendingRemoval,
                                ),
                            );
                            for r in &mut child_rows {
                                if *r >= insert_at {
                                    *r += 1;
                                }
                            }
                        }
                    }
                }
            }

            self.git_objects.flat_view = output;
        }

        // Cleanup expired timers
        let now = Instant::now();
        self.changed_keys.retain(|_, until| *until > now);
        self.modified_keys.retain(|_, until| *until > now);
    }

    // Recursive flattener
    fn flatten_node_recursive(&mut self, node: &GitObject, depth: usize) {
        self.git_objects
            .flat_view
            .push((depth, node.clone(), RenderStatus::Normal));

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

    // Toggle expansion for categories
    pub fn toggle_expand(&mut self) -> Message {
        if self.git_objects.selected_index < self.git_objects.flat_view.len() {
            let (_, selected_obj, _) =
                &self.git_objects.flat_view[self.git_objects.selected_index].clone();

            if let GitObjectType::Category(category_name) = &selected_obj.obj_type {
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

                for obj in &mut self.git_objects.list {
                    if find_and_toggle_category(obj, &name_to_find) {
                        break;
                    }
                }

                self.flatten_tree();

                // Keep the selected category visible
                let mut new_index = 0;
                for (i, (_, obj, _)) in self.git_objects.flat_view.iter().enumerate() {
                    if let GitObjectType::Category(name) = &obj.obj_type
                        && name == &name_to_find
                    {
                        new_index = i;
                        break;
                    }
                }
                if !self.git_objects.flat_view.is_empty() {
                    self.git_objects.selected_index =
                        new_index.min(self.git_objects.flat_view.len() - 1);
                } else {
                    self.git_objects.selected_index = 0;
                }
            }
        }
        Message::LoadGitObjects(Ok(()))
    }

    // ===== Modification detection helpers =====

    /// Find a node in the current tree by its selection key
    fn find_node_by_key(&self, key: &str) -> Option<&GitObject> {
        fn search_in_children<'a>(
            children: &'a [GitObject],
            target_key: &str,
        ) -> Option<&'a GitObject> {
            for child in children {
                let child_key = MainViewState::selection_key(child);
                if child_key == target_key {
                    return Some(child);
                }
                if let Some(found) = search_in_children(&child.children, target_key) {
                    return Some(found);
                }
            }
            None
        }
        search_in_children(&self.git_objects.list, key)
    }

    /// Check if an object has been modified by comparing modification times
    fn is_object_modified(&self, old: &GitObject, new: &GitObject) -> bool {
        match (&old.obj_type, &new.obj_type) {
            (
                GitObjectType::Pack { path: old_path, .. },
                GitObjectType::Pack { path: new_path, .. },
            ) => self.compare_file_mtime(old_path, new_path),
            (GitObjectType::LooseObject { .. }, GitObjectType::LooseObject { .. }) => {
                // Loose objects are content-addressable and immutable
                // Same object_id = same content, different object_id = different object
                // There's no concept of "modification" for loose objects
                false
            }
            (
                GitObjectType::Ref { path: old_path, .. },
                GitObjectType::Ref { path: new_path, .. },
            ) => self.compare_file_mtime(old_path, new_path),
            _ => false,
        }
    }

    /// Compare modification times of two files
    fn compare_file_mtime(&self, old_path: &Path, new_path: &Path) -> bool {
        if old_path != new_path {
            return false; // Different paths, not the same file
        }

        self.is_file_recently_modified(old_path)
    }

    /// Check if a file was modified recently (within last 2 seconds)
    fn is_file_recently_modified(&self, path: &Path) -> bool {
        if let Ok(meta) = fs::metadata(path)
            && let Ok(mtime) = meta.modified()
            && let Ok(elapsed) = mtime.elapsed()
        {
            return elapsed.as_secs() < 2;
        }
        false
    }
}
