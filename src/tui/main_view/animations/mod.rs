use crate::tui::main_view::{Ghost, HighlightInfo};
use std::collections::HashMap;
use std::time::Instant;

/// Animation and highlighting system for the main view
pub struct AnimationManager {
    /// Keys that were added (green highlighting)
    pub changed_keys: HashMap<String, Instant>,
    /// Keys that were modified (orange highlighting)
    pub modified_keys: HashMap<String, Instant>,
    /// Keys that were deleted (red ghost overlay)
    pub ghosts: HashMap<String, Ghost>,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new() -> Self {
        Self {
            changed_keys: HashMap::new(),
            modified_keys: HashMap::new(),
            ghosts: HashMap::new(),
        }
    }

    /// Cleanup expired timers and return true if anything changed
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

    /// Compute highlight information for a given key
    pub fn compute_highlight_info(&self, key: &str) -> HighlightInfo {
        let now = Instant::now();

        // Check for additions (green) - highest priority
        if let Some(until) = self.changed_keys.get(key).copied()
            && until > now
        {
            return HighlightInfo {
                color: Some(ratatui::style::Color::Green),
                expires_at: Some(until),
            };
        }

        // Check for modifications (orange) - medium priority
        if let Some(until) = self.modified_keys.get(key).copied()
            && until > now
        {
            return HighlightInfo {
                color: Some(ratatui::style::Color::Rgb(255, 165, 0)), // Orange
                expires_at: Some(until),
            };
        }

        // Check for deletions/ghosts (red) - handled separately in ghost overlay
        if let Some(ghost) = self.ghosts.get(key)
            && ghost.until > now
        {
            return HighlightInfo {
                color: Some(ratatui::style::Color::Red),
                expires_at: Some(ghost.until),
            };
        }

        // No highlight
        HighlightInfo::default()
    }

    /// Check if there are any active animations
    pub fn has_active_animations(&self) -> bool {
        !self.changed_keys.is_empty() || !self.ghosts.is_empty() || !self.modified_keys.is_empty()
    }

    /// Clear all animations
    pub fn clear_all(&mut self) {
        self.changed_keys.clear();
        self.modified_keys.clear();
        self.ghosts.clear();
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_animation_manager_creation() {
        let manager = AnimationManager::new();
        assert!(!manager.has_active_animations());
    }

    #[test]
    fn test_highlight_computation() {
        let mut manager = AnimationManager::new();
        let now = Instant::now();

        // Add a changed key
        manager
            .changed_keys
            .insert("test_key".to_string(), now + Duration::from_secs(5));

        let highlight = manager.compute_highlight_info("test_key");
        assert!(highlight.color.is_some());
        assert!(manager.has_active_animations());
    }

    #[test]
    fn test_timeout_pruning() {
        let mut manager = AnimationManager::new();
        let past_time = Instant::now() - Duration::from_secs(10);

        // Add expired entries
        manager
            .changed_keys
            .insert("expired".to_string(), past_time);
        manager
            .modified_keys
            .insert("expired".to_string(), past_time);

        let changed = manager.prune_timeouts();
        assert!(changed);
        assert!(!manager.has_active_animations());
    }
}
