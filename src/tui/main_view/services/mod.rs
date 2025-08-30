pub mod direct_highlight_service;
pub mod dynamic_folder_service;
pub mod git_service;
pub mod precomputed_highlight_service;
pub mod service_container;
pub mod state_service;
pub mod tree_service;
pub mod ui_service;

// Re-export services for easy access
pub use direct_highlight_service::DirectHighlightService;
pub use dynamic_folder_service::DynamicFolderService;
pub use git_service::GitRepositoryService;
pub use precomputed_highlight_service::{PrecomputedHighlightService, PrecomputedHighlights};
pub use service_container::ServiceContainer;
pub use state_service::StateService;
pub use tree_service::TreeService;
pub use ui_service::UIService;
