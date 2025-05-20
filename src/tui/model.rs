use std::path::PathBuf;
use std::time::SystemTime;

// Import main view types from the main_view module
use crate::educational_content::EducationalContent;
use crate::tui::main_view::MainViewState;
use crate::tui::pack_details::PackViewState;

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
    Pack {
        path: PathBuf,
        size: Option<u64>,
        modified_time: Option<SystemTime>,
    },
    Ref {
        path: PathBuf,
        content: Option<String>,
    },
    LooseObject {
        path: PathBuf,
        size: Option<u64>,
        object_id: Option<String>,
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
    pub fn new_category(name: &str) -> Self {
        Self {
            name: name.to_string(),
            obj_type: GitObjectType::Category(name.to_string()),
            children: Vec::new(),
            expanded: true,
        }
    }

    pub fn new_pack(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Load pack file details
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
            obj_type: GitObjectType::Pack {
                path,
                size,
                modified_time,
            },
            children: Vec::new(),
            expanded: false,
        }
    }

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

    pub fn new_loose_object(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Load loose object details
        let (size, object_id) = match std::fs::metadata(&path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                let parent_name = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let obj_id = format!("{parent_name}{file_name}");
                (Some(file_size), Some(obj_id))
            }
            Err(_) => (None, None),
        };

        Self {
            name,
            obj_type: GitObjectType::LooseObject {
                path,
                size,
                object_id,
            },
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn add_child(&mut self, child: Self) {
        self.children.push(child);
    }

    // Utility method to format SystemTime as "time ago" string
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
    Main { state: MainViewState },
    PackObjectDetail { state: PackViewState },
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
    // Layout dimensions for accurate scrolling
    pub layout_dimensions: LayoutDimensions,
    pub educational_content_provider: EducationalContent,
}

impl AppState {
    // Initialize a new application state
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
            view: AppView::Main {
                state: MainViewState::new(&educational_content_provider),
            },
            layout_dimensions: LayoutDimensions::default(),
            educational_content_provider,
        }
    }

    // Update layout dimensions based on terminal size
    pub fn update_layout_dimensions(&mut self, terminal_size: ratatui::layout::Size) {
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

    pub fn is_wide_screen(&self) -> bool {
        self.layout_dimensions.terminal_width > 158
    }
}
