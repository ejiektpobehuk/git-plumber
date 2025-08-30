use super::{GitRepositoryService, StateService, TreeService, UIService};

/// Container for all domain services with clear boundaries and dependency injection
pub struct ServiceContainer {
    pub tree_service: TreeService,
    pub state_service: StateService,
    pub ui_service: UIService,
    pub git_service: GitRepositoryService,
}

impl ServiceContainer {
    /// Create a new service container with all services initialized
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tree_service: TreeService::new(),
            state_service: StateService::new(),
            ui_service: UIService::new(),
            git_service: GitRepositoryService::new(),
        }
    }

    /// Get a reference to the tree service
    #[must_use]
    pub const fn tree(&self) -> &TreeService {
        &self.tree_service
    }

    /// Get a reference to the state service
    #[must_use]
    pub const fn state(&self) -> &StateService {
        &self.state_service
    }

    /// Get a reference to the UI service
    #[must_use]
    pub const fn ui(&self) -> &UIService {
        &self.ui_service
    }

    /// Get a reference to the git service
    #[must_use]
    pub const fn git(&self) -> &GitRepositoryService {
        &self.git_service
    }
}

impl Default for ServiceContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Service boundaries and responsibilities:
///
/// **TreeService**:
/// - Tree flattening and traversal
/// - Node finding and manipulation
/// - Tree statistics and validation
/// - Natural sorting
///
/// **StateService**:
/// - State persistence and restoration
/// - Synchronization between old/new structures
/// - Session management (selection, scroll positions)
/// - Content state management
///
/// **UIService**:
/// - UI state management (focus, navigation)
/// - Selection and scroll calculations
/// - User interaction handling
/// - Viewport management
///
/// **GitRepositoryService**:
/// - Git repository operations
/// - File system interaction
/// - Object modification detection
/// - Repository validation
///
/// **Service Dependencies**:
/// - Services are designed to be stateless and independent
/// - No circular dependencies between services
/// - Services can be injected and tested independently
/// - Clear separation of concerns with minimal coupling

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_container_creation() {
        let container = ServiceContainer::new();

        // Verify all services are accessible
        let _ = container.tree();
        let _ = container.state();
        let _ = container.ui();
        let _ = container.git();
    }

    #[test]
    fn test_service_container_default() {
        let container = ServiceContainer::default();
        let _ = container;
    }
}
