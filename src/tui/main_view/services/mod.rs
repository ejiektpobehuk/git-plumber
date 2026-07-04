pub mod dynamic_folder_service;
pub mod git_service;
pub mod precomputed_highlight_service;
pub mod tree_service;
pub mod ui_service;

// Re-export services for easy access
pub use dynamic_folder_service::DynamicFolderService;
pub use git_service::GitRepositoryService;
pub use precomputed_highlight_service::PrecomputedHighlightService;
pub use tree_service::TreeService;
pub use ui_service::UIService;
