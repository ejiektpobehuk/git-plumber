use std::path::PathBuf;
impl AppState {
    pub fn set_tx(&mut self, tx: Sender<crate::tui::message::Message>) {
        self.tx = Some(tx);
    }
}

use crossbeam_channel::Sender;

use std::time::SystemTime;

// Import main view types from the main_view module
use crate::educational_content::EducationalContent;
use crate::git::loose_object::LooseObject;
use crate::tui::loose_details::LooseObjectViewState;
use crate::tui::main_view::MainViewState;
use crate::tui::pack_details::PackViewState;

// Minimum terminal dimensions required for the app to function properly
pub const MIN_TERMINAL_WIDTH: u16 = 45; // Left panel (42) + minimal right panel
pub const MIN_TERMINAL_HEIGHT: u16 = 8; // Header + footer + minimal content

// Define a structure for individual pack objects
#[derive(Debug, Clone)]
pub struct PackObject {
    pub index: usize,
    pub obj_type: String,
    pub size: u32,
    pub sha1: Option<String>,      // SHA-1 hash of the object
    pub base_info: Option<String>, // For delta objects
    pub object_data: Option<crate::git::pack::Object>, // The actual parsed object
}

// Define a tree structure for Git objects
#[derive(Debug, Clone)]
pub enum GitObjectType {
    Category(String),
    FileSystemFolder {
        path: PathBuf,
        is_educational: bool, // True if this folder should show educational content
        is_loaded: bool,      // True if folder contents have been loaded
        is_empty_cached: Option<bool>, // Cached empty state to avoid filesystem I/O during rendering
    },
    FileSystemFile {
        path: PathBuf,
        size: Option<u64>,
        modified_time: Option<SystemTime>,
    },

    PackFolder {
        base_name: String,
        pack_group: crate::git::repository::PackGroup,
    },
    PackFile {
        file_type: String, // "packfile", "index", "xedni", "mtime"
        path: PathBuf,
        size: Option<u64>,
        modified_time: Option<SystemTime>,
    },
    Ref {
        path: PathBuf,
        content: Option<String>,
    },
    LooseObject {
        size: Option<u64>,
        object_id: Option<String>,
        parsed_object: Option<LooseObject>,
    },
}

#[derive(Debug, Clone)]
pub struct GitObject {
    pub name: String,
    pub obj_type: GitObjectType,
    pub children: Vec<GitObject>,
    pub expanded: bool,
}

impl GitObject {
    /// Check if this folder is empty (has no children)
    /// Note: This method should avoid filesystem I/O as it's called during rendering
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match &self.obj_type {
            GitObjectType::Category(_) => self.children.is_empty(),
            GitObjectType::FileSystemFolder {
                is_loaded,
                is_empty_cached,
                ..
            } => {
                if *is_loaded {
                    // If loaded, check children (most accurate)
                    self.children.is_empty()
                } else if let Some(cached) = is_empty_cached {
                    // Use cached value to avoid filesystem I/O during rendering
                    *cached
                } else {
                    // No cache available, assume not empty to avoid filesystem I/O during rendering
                    // This means new folders will show as "has content" until expanded or cache is populated
                    false
                }
            }
            GitObjectType::PackFolder { pack_group, .. } => {
                // Pack folder is empty if it has no valid files
                pack_group.get_all_files().is_empty()
            }
            _ => false, // Non-folder types are never "empty" in this context
        }
    }

    /// Populate the empty state cache for a filesystem folder if not already cached
    pub fn ensure_empty_cache_populated(&mut self) {
        if let GitObjectType::FileSystemFolder {
            path,
            is_loaded,
            is_empty_cached,
            ..
        } = &mut self.obj_type
        {
            // Only compute cache if not loaded and not already cached
            if !*is_loaded && is_empty_cached.is_none() {
                *is_empty_cached = match std::fs::read_dir(path) {
                    Ok(mut entries) => Some(entries.next().is_none()),
                    Err(_) => Some(false), // If we can't read it, assume not empty
                };
            }
        }
    }

    /// Force refresh the empty state cache for a filesystem folder, even if already cached
    /// This is used to detect content changes in collapsed folders
    pub fn refresh_empty_cache(&mut self) {
        if let GitObjectType::FileSystemFolder {
            path,
            is_loaded,
            is_empty_cached,
            ..
        } = &mut self.obj_type
        {
            // For collapsed folders (not loaded), always refresh the cache
            if !*is_loaded {
                *is_empty_cached = match std::fs::read_dir(path) {
                    Ok(mut entries) => Some(entries.next().is_none()),
                    Err(_) => Some(false), // If we can't read it, assume not empty
                };
            }
        }
    }

    /// Recursively refresh empty state caches for all collapsed filesystem folders in the tree
    pub fn refresh_empty_caches_for_collapsed(&mut self) {
        self.refresh_empty_cache();
        for child in &mut self.children {
            child.refresh_empty_caches_for_collapsed();
        }
    }

    /// Recursively populate empty state caches for all filesystem folders in the tree
    pub fn populate_empty_caches_recursive(&mut self) {
        self.ensure_empty_cache_populated();
        for child in &mut self.children {
            child.populate_empty_caches_recursive();
        }
    }

    #[must_use]
    pub fn new_category(name: &str) -> Self {
        Self {
            name: name.to_string(),
            obj_type: GitObjectType::Category(name.to_string()),
            children: Vec::new(),
            expanded: true,
        }
    }

    #[must_use]
    pub fn new_filesystem_folder(path: PathBuf, is_educational: bool) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Self {
            name,
            obj_type: GitObjectType::FileSystemFolder {
                path,
                is_educational,
                is_loaded: false,
                is_empty_cached: None, // Will be computed lazily when needed
            },
            children: Vec::new(),
            expanded: false, // Start collapsed for on-demand loading
        }
    }

    #[must_use]
    pub fn new_filesystem_file(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Load file metadata
        let (size, modified_time) = match std::fs::metadata(&path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                let mod_time = metadata.modified().ok();
                (Some(file_size), mod_time)
            }
            Err(_) => (None, None),
        };

        Self {
            name,
            obj_type: GitObjectType::FileSystemFile {
                path,
                size,
                modified_time,
            },
            children: Vec::new(),
            expanded: false,
        }
    }

    #[must_use]
    pub fn new_pack_folder(pack_group: crate::git::repository::PackGroup) -> Self {
        let mut pack_folder = Self {
            name: pack_group.base_name.clone(),
            obj_type: GitObjectType::PackFolder {
                base_name: pack_group.base_name.clone(),
                pack_group: pack_group.clone(),
            },
            children: Vec::new(),
            expanded: false,
        };

        // Add children for each available file type
        // Add "pack" first (renamed from "packfile" to distinguish from the folder)
        for (file_type, path) in pack_group.get_all_files() {
            let child_file_type = match file_type {
                "packfile" => "pack", // Rename packfile to pack for the child
                other => other,
            };
            pack_folder.add_child(Self::new_pack_file(
                child_file_type.to_string(),
                path.clone(),
            ));
        }

        pack_folder
    }

    #[must_use]
    pub fn new_pack_file(file_type: String, path: PathBuf) -> Self {
        let name = match file_type.as_str() {
            "packfile" => "packfile",
            "pack" => "pack",
            "index" => "index",
            "xedni" => "xedni",
            "mtime" => "mtime",
            _ => "unknown",
        }
        .to_string();

        // Load file details
        let (size, modified_time) = match std::fs::metadata(&path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                let mod_time = metadata.modified().ok();
                (Some(file_size), mod_time)
            }
            Err(_) => (None, None),
        };

        Self {
            name,
            obj_type: GitObjectType::PackFile {
                file_type,
                path,
                size,
                modified_time,
            },
            children: Vec::new(),
            expanded: false,
        }
    }

    #[must_use]
    pub fn new_ref(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Load reference content
        let ref_content = std::fs::read_to_string(&path)
            .ok()
            .map(|content| content.trim().to_string());

        Self {
            name,
            obj_type: GitObjectType::Ref {
                path,
                content: ref_content,
            },
            children: Vec::new(),
            expanded: false,
        }
    }

    #[must_use]
    pub fn new_parsed_loose_object(parsed_object: LooseObject) -> Self {
        // Create a display name that includes the object type
        let short_id = if parsed_object.object_id.len() >= 8 {
            &parsed_object.object_id[..8]
        } else {
            &parsed_object.object_id
        };
        let name = format!("{} {}", parsed_object.object_type, short_id);

        Self {
            name,
            obj_type: GitObjectType::LooseObject {
                size: Some(parsed_object.size as u64),
                object_id: Some(parsed_object.object_id.clone()),
                parsed_object: Some(parsed_object),
            },
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn add_child(&mut self, child: Self) {
        self.children.push(child);
    }

    /// Restore expansion and loading state from another `GitObject` tree
    pub fn restore_state_from(&mut self, old_tree: &[Self]) {
        fn find_matching_object<'a>(
            children: &'a [GitObject],
            target_key: &str,
        ) -> Option<&'a GitObject> {
            for child in children {
                let key = crate::tui::main_view::MainViewState::selection_key(child);
                if key == target_key {
                    return Some(child);
                }
                if let Some(found) = find_matching_object(&child.children, target_key) {
                    return Some(found);
                }
            }
            None
        }

        let my_key = crate::tui::main_view::MainViewState::selection_key(self);
        if let Some(old_obj) = find_matching_object(old_tree, &my_key) {
            // Always restore expansion state for all object types
            self.expanded = old_obj.expanded;

            match (&mut self.obj_type, &old_obj.obj_type) {
                // FileSystemFolder: restore loading state and children if loaded
                (
                    GitObjectType::FileSystemFolder {
                        is_loaded,
                        is_educational,
                        is_empty_cached,
                        ..
                    },
                    GitObjectType::FileSystemFolder {
                        is_loaded: old_is_loaded,
                        is_empty_cached: old_is_empty_cached,
                        ..
                    },
                ) => {
                    // For educational folders, keep is_loaded state (they're pre-populated)
                    // For regular folders, always reload to detect file changes
                    if *is_educational {
                        *is_loaded = *old_is_loaded;
                        *is_empty_cached = *old_is_empty_cached; // Keep cache for educational folders
                    } else {
                        // Regular folders: with full tree loading, they should already be loaded
                        // Check if contents are already loaded by the new full tree system
                        if *is_loaded && !self.children.is_empty() {
                            // Already loaded by full tree system, keep the state
                            *is_empty_cached = *old_is_empty_cached;
                        } else if old_obj.expanded {
                            // Old behavior for backwards compatibility (shouldn't happen with full tree)
                            *is_loaded = false; // Mark as not loaded to trigger reload
                            *is_empty_cached = *old_is_empty_cached;
                            let _ = self.load_folder_contents(); // Load fresh contents immediately
                        } else {
                            *is_loaded = false; // Will load on-demand when expanded
                            *is_empty_cached = *old_is_empty_cached;
                        }
                    }

                    // Never restore children - always use fresh children from tree rebuild or reload
                    // This ensures new/modified/deleted files are detected properly
                }

                // Category: only restore expansion state, NOT children
                // Categories like "Loose Objects" should always use fresh children from tree rebuild
                (GitObjectType::Category(_), GitObjectType::Category(_)) => {
                    // expansion state already restored above, don't restore children
                }

                // Pack, Ref, LooseObject: these are leaf nodes, just restore expansion state
                // (expansion state already restored above)
                _ => {}
            }
        }

        // Recursively restore state for children
        for child in &mut self.children {
            child.restore_state_from(old_tree);
        }
    }

    /// Load the contents of a filesystem folder on demand
    pub fn load_folder_contents(&mut self) -> Result<(), String> {
        match &mut self.obj_type {
            GitObjectType::FileSystemFolder {
                path,
                is_loaded,
                is_empty_cached,
                ..
            } => {
                if *is_loaded {
                    return Ok(()); // Already loaded
                }

                // Read directory contents
                match std::fs::read_dir(path) {
                    Ok(entries) => {
                        let mut items: Vec<(String, PathBuf, bool)> = Vec::new();

                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            let name = entry.file_name().to_string_lossy().to_string();
                            let is_dir = entry_path.is_dir();
                            items.push((name, entry_path, is_dir));
                        }

                        // Sort: directories first, then files, both alphabetically
                        items.sort_by(|a, b| {
                            match (a.2, b.2) {
                                (true, false) => std::cmp::Ordering::Less, // dirs before files
                                (false, true) => std::cmp::Ordering::Greater, // files after dirs
                                _ => a.0.cmp(&b.0), // alphabetical within same type
                            }
                        });

                        // Create child objects
                        for (_, entry_path, is_dir) in items {
                            if is_dir {
                                self.children
                                    .push(Self::new_filesystem_folder(entry_path, false));
                            } else {
                                self.children.push(Self::new_filesystem_file(entry_path));
                            }
                        }

                        // Update the empty state cache now that we have loaded the contents
                        *is_empty_cached = Some(self.children.is_empty());
                        *is_loaded = true;
                        Ok(())
                    }
                    Err(e) => Err(format!("Failed to read directory: {e}")),
                }
            }
            _ => Err("Cannot load contents of non-folder object".to_string()),
        }
    }

    // Utility method to format SystemTime as "time ago" string
    #[must_use]
    pub fn format_time_ago(time: &SystemTime) -> String {
        match time.elapsed() {
            Ok(elapsed) => {
                let seconds = elapsed.as_secs();
                if seconds < 60 {
                    format!("{seconds} seconds ago")
                } else if seconds < 3600 {
                    format!("{} minutes ago", seconds / 60)
                } else if seconds < 86400 {
                    format!("{} hours ago", seconds / 3600)
                } else {
                    format!("{} days ago", seconds / 86400)
                }
            }
            Err(_) => "Unknown time".to_string(),
        }
    }
}

// Define the application view modes (simplified)
pub enum AppView {
    Main {
        state: MainViewState,
    },
    PackObjectDetail {
        state: PackViewState,
    },
    LooseObjectDetail {
        state: LooseObjectViewState,
    },
    TerminalTooSmall {
        width: u16,
        height: u16,
        min_width: u16,
        min_height: u16,
    },
}

// Store layout dimensions for accurate scrolling
#[derive(Debug, Clone)]
pub struct LayoutDimensions {
    pub educational_content_height: usize,
    pub pack_objects_height: usize,
    pub git_objects_height: usize,
    pub object_details_height: usize,
    pub terminal_width: usize,
}

impl Default for LayoutDimensions {
    fn default() -> Self {
        Self {
            educational_content_height: 8, // Conservative default
            pack_objects_height: 8,        // Conservative default
            git_objects_height: 25,        // Conservative default
            object_details_height: 6,      // Fixed height from layout
            terminal_width: 0,             // Default value, actual width should be set
        }
    }
}

// Define the application state (Model)
pub struct AppState {
    // Repository data
    pub repo_path: PathBuf,
    pub project_name: String,
    // Error state
    pub error: Option<String>,
    // Current view
    pub view: AppView,
    // View stack for navigation history
    pub view_stack: Vec<AppView>,
    // Layout dimensions for accurate scrolling
    pub layout_dimensions: LayoutDimensions,
    pub educational_content_provider: EducationalContent,
    // Background message sender (for spawning jobs)
    pub tx: Option<Sender<crate::tui::message::Message>>,
    // Effects produced by update to be executed by the runner
    pub effects: Vec<crate::tui::message::Command>,
    // Keep FS watcher alive for the lifetime of the app
    pub fs_watcher: Option<notify::RecommendedWatcher>,
    // Preferences
    pub reduced_motion: bool,
    pub animation_duration_secs: u64,
    // Rendering optimization
    pub last_terminal_size: Option<ratatui::layout::Size>,
    // Flag to indicate we need to reload selection-dependent content after view restoration
    pub needs_selection_reload: bool,
}

impl AppState {
    // Initialize a new application state
    #[must_use]
    pub fn new(repo_path: PathBuf) -> Self {
        let educational_content_provider = EducationalContent::new();

        // Compute project name from repo path
        let project_name = if repo_path == PathBuf::from(".") {
            // For current directory, get the name from the current working directory
            std::env::current_dir()
                .ok()
                .and_then(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str().map(String::from))
                })
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            // For other paths, use the provided path
            repo_path
                .file_name()
                .and_then(|name| name.to_str().map(String::from))
                .unwrap_or_else(|| "unknown".to_string())
        };

        Self {
            repo_path,
            project_name,
            error: None,
            fs_watcher: None,
            view: AppView::Main {
                state: MainViewState::new(&educational_content_provider),
            },
            view_stack: Vec::new(),
            layout_dimensions: LayoutDimensions::default(),
            educational_content_provider,
            tx: None,
            effects: Vec::new(),
            // Overwritten by run_tui
            // TODO: use default values from config
            reduced_motion: false,
            animation_duration_secs: 10,
            // Rendering optimization
            last_terminal_size: None,
            needs_selection_reload: false,
        }
    }

    // Push current view onto stack and set new view
    pub fn push_view(&mut self, new_view: AppView) {
        let current_view = std::mem::replace(&mut self.view, new_view);
        self.view_stack.push(current_view);
    }

    // Pop previous view from stack and restore it
    pub fn pop_view(&mut self) -> bool {
        if let Some(previous_view) = self.view_stack.pop() {
            self.view = previous_view;
            true
        } else {
            false
        }
    }

    // Update layout dimensions based on terminal size
    pub const fn update_layout_dimensions(&mut self, terminal_size: ratatui::layout::Size) {
        // Calculate the main content area (subtract header and footer)
        let main_content_height = terminal_size.height.saturating_sub(2) as usize;

        // Left panel is fixed width (42), right panel gets the rest
        let _right_panel_width = terminal_size.width.saturating_sub(42) as usize;

        // Git objects get the full left panel height
        self.layout_dimensions.git_objects_height = main_content_height.saturating_sub(2); // Account for borders

        // For pack preview layout: object details (6) + educational (50%) + pack objects (50%)
        let object_details_height = 6;
        let remaining_height = main_content_height.saturating_sub(object_details_height);

        // Educational content and pack objects split the remaining space
        self.layout_dimensions.educational_content_height =
            (remaining_height / 2).saturating_sub(2); // Account for borders
        self.layout_dimensions.pack_objects_height = (remaining_height / 2).saturating_sub(2); // Account for borders
        self.layout_dimensions.object_details_height = object_details_height;

        // Store the full terminal width for wide screen detection
        self.layout_dimensions.terminal_width = terminal_size.width as usize;
    }

    #[must_use]
    pub const fn is_wide_screen(&self) -> bool {
        self.layout_dimensions.terminal_width > 158
    }

    // Check if terminal size meets minimum requirements
    pub fn is_terminal_too_small(size: ratatui::layout::Size) -> bool {
        size.width < MIN_TERMINAL_WIDTH || size.height < MIN_TERMINAL_HEIGHT
    }

    // Rendering optimization methods
    pub fn check_terminal_resize(&mut self, current_size: ratatui::layout::Size) -> bool {
        if self.last_terminal_size == Some(current_size) {
            false
        } else {
            // Always update layout dimensions first to ensure proper initialization
            self.update_layout_dimensions(current_size);

            // Check if terminal is too small and switch to appropriate view
            if Self::is_terminal_too_small(current_size) {
                // Store the previous view if we're not already in TerminalTooSmall view
                if !matches!(self.view, AppView::TerminalTooSmall { .. }) {
                    // Push current view to stack to restore later
                    let current_view = std::mem::replace(
                        &mut self.view,
                        AppView::TerminalTooSmall {
                            width: current_size.width,
                            height: current_size.height,
                            min_width: MIN_TERMINAL_WIDTH,
                            min_height: MIN_TERMINAL_HEIGHT,
                        },
                    );
                    self.view_stack.push(current_view);
                } else {
                    // Already in TerminalTooSmall view, just update the dimensions
                    if let AppView::TerminalTooSmall { width, height, .. } = &mut self.view {
                        *width = current_size.width;
                        *height = current_size.height;
                    }
                }
            } else {
                // Terminal is large enough, check if we need to restore from TerminalTooSmall view
                if matches!(self.view, AppView::TerminalTooSmall { .. }) {
                    // Restore the previous view
                    if let Some(previous_view) = self.view_stack.pop() {
                        self.view = previous_view;
                    } else {
                        // Fallback: create a new main view if no previous view exists
                        let main_view_state =
                            MainViewState::new(&self.educational_content_provider);
                        self.view = AppView::Main {
                            state: main_view_state,
                        };
                    }
                    // Mark that we need to reload selection-dependent content after restoration
                    self.needs_selection_reload = true;
                }
            }

            self.last_terminal_size = Some(current_size);
            true
        }
    }

    // Check if animations are currently active (including folder highlights)
    pub fn has_active_animations(&self) -> bool {
        if let AppView::Main { state } = &self.view {
            state.animations.has_active_animations_with_tree(
                &state.tree.list,
                crate::tui::main_view::MainViewState::selection_key,
            )
        } else {
            false
        }
    }
}
