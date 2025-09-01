use crate::tui::main_view::{Ghost, HighlightInfo};
use std::collections::HashMap;
use std::time::Instant;

/// Dynamic information about what file highlights are currently active inside a folder
#[derive(Debug, Clone)]
pub struct DynamicFolderHighlight {
    /// Color computed from active file highlights inside
    pub color: ratatui::style::Color,
    /// Earliest expiration time of any file highlight inside
    pub expires_at: Instant,
}

/// Animation and highlighting system for the main view
pub struct AnimationManager {
    /// Keys that were added (green highlighting)
    pub changed_keys: HashMap<String, Instant>,
    /// Keys that were modified (orange highlighting)
    pub modified_keys: HashMap<String, Instant>,
    /// Keys that were deleted (red ghost overlay)
    pub ghosts: HashMap<String, Ghost>,
    /// Current blink state for folder animations (true = visible, false = hidden)
    pub folder_blink_state: bool,
    /// Last time the blink state was toggled
    pub last_blink_toggle: Instant,
}

impl AnimationManager {
    /// Create a new animation manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            changed_keys: HashMap::new(),
            modified_keys: HashMap::new(),
            ghosts: HashMap::new(),
            folder_blink_state: true,
            last_blink_toggle: Instant::now(),
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

        // Update blink state every 500ms (twice per second)
        let blink_changed = if now.duration_since(self.last_blink_toggle).as_millis() >= 500 {
            self.folder_blink_state = !self.folder_blink_state;
            self.last_blink_toggle = now;
            true
        } else {
            false
        };

        ghosts_changed || changed_changed || modified_changed || blink_changed
    }

    /// Compute highlight information for a given key
    #[must_use]
    pub fn compute_highlight_info(&self, key: &str) -> HighlightInfo {
        let now = Instant::now();
        let is_folder = Self::is_folder_key(key);

        // Check for additions (green) - highest priority
        if let Some(until) = self.changed_keys.get(key).copied()
            && until > now
        {
            return HighlightInfo {
                color: Some(ratatui::style::Color::Green),
                expires_at: Some(until),
                animation_type: if is_folder {
                    crate::tui::main_view::model::AnimationType::FolderBlink
                } else {
                    crate::tui::main_view::model::AnimationType::FileShrink
                },
            };
        }

        // Check for modifications (orange) - second highest priority
        if let Some(until) = self.modified_keys.get(key).copied()
            && until > now
        {
            return HighlightInfo {
                color: Some(ratatui::style::Color::Rgb(255, 165, 0)), // Orange
                expires_at: Some(until),
                animation_type: if is_folder {
                    crate::tui::main_view::model::AnimationType::FolderBlink
                } else {
                    crate::tui::main_view::model::AnimationType::FileShrink
                },
            };
        }

        // Check for deletions/ghosts (red) - third priority
        if let Some(ghost) = self.ghosts.get(key)
            && ghost.until > now
        {
            return HighlightInfo {
                color: Some(ratatui::style::Color::Red),
                expires_at: Some(ghost.until),
                animation_type: if is_folder {
                    crate::tui::main_view::model::AnimationType::FolderBlink
                } else {
                    crate::tui::main_view::model::AnimationType::FileShrink
                },
            };
        }

        // No highlight for individual files
        HighlightInfo::default()
    }

    /// Determine if a selection key represents a folder
    fn is_folder_key(key: &str) -> bool {
        key.starts_with("folder:") || key.starts_with("category:")
    }

    /// Compute dynamic folder highlight based on files inside (called externally with tree context)
    #[must_use]
    pub fn compute_folder_highlight(
        &self,
        _folder_key: &str,
        files_inside: &[String],
    ) -> Option<DynamicFolderHighlight> {
        let now = Instant::now();
        let mut active_colors = Vec::new();
        let mut earliest_expiration = None;

        // Check each file inside the folder for active highlights
        for file_key in files_inside {
            // Check if this file has any active highlight
            if let Some(until) = self.changed_keys.get(file_key).copied()
                && until > now
            {
                active_colors.push(ratatui::style::Color::Green);
                earliest_expiration = Some(match earliest_expiration {
                    None => until,
                    Some(existing) => std::cmp::min(existing, until),
                });
            }

            if let Some(until) = self.modified_keys.get(file_key).copied()
                && until > now
            {
                active_colors.push(ratatui::style::Color::Rgb(255, 165, 0)); // Orange
                earliest_expiration = Some(match earliest_expiration {
                    None => until,
                    Some(existing) => std::cmp::min(existing, until),
                });
            }

            if let Some(ghost) = self.ghosts.get(file_key)
                && ghost.until > now
            {
                active_colors.push(ratatui::style::Color::Red);
                earliest_expiration = Some(match earliest_expiration {
                    None => ghost.until,
                    Some(existing) => std::cmp::min(existing, ghost.until),
                });
            }
        }

        // If no active highlights found, no folder highlight
        if active_colors.is_empty() {
            return None;
        }

        // Compute folder color based on active file highlights
        let folder_color = Self::compute_mixed_color(&active_colors);

        Some(DynamicFolderHighlight {
            color: folder_color,
            expires_at: earliest_expiration.unwrap(),
        })
    }

    /// Check if there are any active animations (must be called with tree context for folder highlights)
    #[must_use]
    pub fn has_active_animations(&self) -> bool {
        !self.changed_keys.is_empty() || !self.ghosts.is_empty() || !self.modified_keys.is_empty()
    }

    /// Check if there are any active animations including dynamic folder highlights
    pub fn has_active_animations_with_tree(
        &self,
        tree: &[crate::tui::model::GitObject],
        selection_key_fn: fn(&crate::tui::model::GitObject) -> String,
    ) -> bool {
        // Check regular animations first
        if self.has_active_animations() {
            return true;
        }

        // Check for dynamic folder highlights
        crate::tui::main_view::services::DynamicFolderService::has_active_folder_highlights(
            self,
            tree,
            selection_key_fn,
        )
    }

    /// Clear all animations
    pub fn clear_all(&mut self) {
        self.changed_keys.clear();
        self.modified_keys.clear();
        self.ghosts.clear();
    }

    /// Compute mixed color from a list of active highlight colors
    fn compute_mixed_color(colors: &[ratatui::style::Color]) -> ratatui::style::Color {
        if colors.is_empty() {
            return ratatui::style::Color::Gray;
        }

        // Remove duplicates and check what types we have
        let mut unique_colors = std::collections::HashSet::new();
        for color in colors {
            unique_colors.insert(*color);
        }

        // Convert to a sorted vector for consistent behavior
        let mut color_vec: Vec<_> = unique_colors.into_iter().collect();
        color_vec.sort_by_key(|color| match color {
            ratatui::style::Color::Green => 0,
            ratatui::style::Color::Rgb(255, 165, 0) => 1, // Orange
            ratatui::style::Color::Red => 2,
            _ => 3,
        });

        match color_vec.len() {
            1 => color_vec[0],                  // Single color type
            _ => ratatui::style::Color::Yellow, // Mixed colors
        }
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
