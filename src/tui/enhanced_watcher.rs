use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Enhanced watcher that provides specific file change information
/// instead of just generic refresh messages
pub fn spawn_enhanced_git_watcher(
    repo_path: PathBuf,
    tx_ui: crossbeam_channel::Sender<crate::tui::message::Message>,
) -> Result<RecommendedWatcher, notify::Error> {
    // Std mpsc channel for notify callback; we'll bridge to crossbeam
    let (tx_fs, rx_fs) = std_mpsc::channel::<notify::Result<Event>>();

    // Recommended watcher with closure callback
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx_fs.send(res);
    })?;

    // Watch the .git directory recursively if it exists
    let git_dir = repo_path.join(".git");
    if git_dir.exists() {
        watcher.watch(&git_dir, RecursiveMode::Recursive)?;
    }

    // Enhanced debouncing with change accumulation
    std::thread::spawn(move || {
        let quiet = Duration::from_millis(200);
        let mut pending = false;
        let mut change_accumulator = ChangeAccumulator::new();

        loop {
            // Choose blocking or timeout receive based on pending state
            let recv_result = if pending {
                rx_fs.recv_timeout(quiet)
            } else {
                match rx_fs.recv() {
                    Ok(ev) => Ok(ev),
                    Err(_) => Err(std_mpsc::RecvTimeoutError::Disconnected),
                }
            };

            match recv_result {
                Ok(Ok(event)) => {
                    if is_meaningful(&event) {
                        change_accumulator.process_event(event);
                        pending = true;
                    }
                }
                Ok(Err(_e)) => {
                    // Treat watcher errors as meaningful: fall back to generic refresh
                    pending = true;
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => {
                    if pending {
                        // Send accumulated changes or generic refresh
                        let message = change_accumulator.take_changes()
                            .unwrap_or(crate::tui::message::Message::Refresh);
                        let _ = tx_ui.send(message);
                        pending = false;
                    }
                }
                Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(watcher)
}

/// Accumulates file system changes over the debounce period
struct ChangeAccumulator {
    added_files: HashSet<PathBuf>,
    modified_files: HashSet<PathBuf>,
    deleted_files: HashSet<PathBuf>,
}

impl ChangeAccumulator {
    fn new() -> Self {
        Self {
            added_files: HashSet::new(),
            modified_files: HashSet::new(),
            deleted_files: HashSet::new(),
        }
    }

    /// Process a file system event and accumulate changes
    fn process_event(&mut self, event: Event) {
        for path in event.paths {
            // Only process git-related paths
            if !is_git_related_path(&path) {
                continue;
            }

            match event.kind {
                EventKind::Create(_) => {
                    // Remove from deleted (in case of move/rename)
                    self.deleted_files.remove(&path);
                    self.added_files.insert(path);
                }
                EventKind::Modify(_) => {
                    // Only mark as modified if not already added in this batch
                    if !self.added_files.contains(&path) {
                        self.modified_files.insert(path);
                    }
                }
                EventKind::Remove(_) => {
                    // Remove from added/modified and mark as deleted
                    self.added_files.remove(&path);
                    self.modified_files.remove(&path);
                    self.deleted_files.insert(path);
                }
                EventKind::Any => {
                    // Generic change - treat as modification
                    if !self.added_files.contains(&path) {
                        self.modified_files.insert(path);
                    }
                }
                _ => {} // Ignore other event types
            }
        }
    }

    /// Take accumulated changes and return a message
    fn take_changes(&mut self) -> Option<crate::tui::message::Message> {
        if self.added_files.is_empty() 
            && self.modified_files.is_empty() 
            && self.deleted_files.is_empty() 
        {
            return None;
        }

        let message = crate::tui::message::Message::DirectFileChanges {
            added_files: std::mem::take(&mut self.added_files),
            modified_files: std::mem::take(&mut self.modified_files),
            deleted_files: std::mem::take(&mut self.deleted_files),
        };

        Some(message)
    }
}

/// Check if an event is meaningful (same as original implementation)
fn is_meaningful(event: &Event) -> bool {
    // Ignore lock files created by git for atomic updates
    let has_lock = event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".lock"))
            .unwrap_or(false)
    });
    if has_lock {
        return false;
    }

    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

/// Check if a path is git-related and should be processed
fn is_git_related_path(path: &PathBuf) -> bool {
    let path_str = path.to_string_lossy();
    
    // Include common git paths
    path_str.contains("/objects/") ||     // Loose objects
    path_str.contains("/refs/") ||        // References
    path_str.contains("/packs/") ||       // Pack files
    path_str.ends_with("/HEAD") ||        // HEAD file
    path_str.ends_with("/index") ||       // Index file
    path_str.contains("/hooks/") ||       // Git hooks
    path_str.contains("/.git/")           // Any file in .git directory
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_accumulator() {
        let mut accumulator = ChangeAccumulator::new();
        
        // Simulate creating a file
        let create_event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("/repo/.git/objects/ab/cd1234")],
            attrs: Default::default(),
        };
        accumulator.process_event(create_event);
        
        // Simulate modifying a file
        let modify_event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![PathBuf::from("/repo/.git/refs/heads/main")],
            attrs: Default::default(),
        };
        accumulator.process_event(modify_event);
        
        // Take changes
        let message = accumulator.take_changes().unwrap();
        
        if let crate::tui::message::Message::DirectFileChanges { added_files, modified_files, deleted_files } = message {
            assert!(added_files.contains(&PathBuf::from("/repo/.git/objects/ab/cd1234")));
            assert!(modified_files.contains(&PathBuf::from("/repo/.git/refs/heads/main")));
            assert!(deleted_files.is_empty());
        } else {
            panic!("Expected DirectFileChanges message");
        }
    }

    #[test]
    fn test_git_path_filtering() {
        assert!(is_git_related_path(&PathBuf::from("/repo/.git/objects/ab/cd1234")));
        assert!(is_git_related_path(&PathBuf::from("/repo/.git/refs/heads/main")));
        assert!(is_git_related_path(&PathBuf::from("/repo/.git/HEAD")));
        
        assert!(!is_git_related_path(&PathBuf::from("/repo/src/main.rs")));
        assert!(!is_git_related_path(&PathBuf::from("/repo/README.md")));
    }
}

