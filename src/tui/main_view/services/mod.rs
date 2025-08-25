pub mod git_service;
pub mod service_container;
pub mod state_service;
pub mod tree_service;
pub mod ui_service;

// Re-export services for easy access
pub use git_service::GitRepositoryService;
pub use service_container::ServiceContainer;
pub use state_service::StateService;
pub use tree_service::TreeService;
pub use ui_service::UIService;
