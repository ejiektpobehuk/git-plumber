use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub fn spawn_git_watcher(
    repo_path: PathBuf,
    tx_ui: crossbeam_channel::Sender<crate::tui::message::Message>,
) -> Result<RecommendedWatcher, notify::Error> {
    // Std mpsc channel for notify callback; we'll bridge to crossbeam
    let (tx_fs, rx_fs) = std_mpsc::channel::<notify::Result<Event>>();

    // Recommended watcher with closure callback
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx_fs.send(res);
    })?;

    // No extra config necessary for notify v8 here; we do our own debounce below.

    // Watch the .git directory recursively if it exists
    let git_dir = repo_path.join(".git");
    if git_dir.exists() {
        watcher.watch(&git_dir, RecursiveMode::Recursive)?;
    }

    // Quiet-period debounce with dynamic blocking: block when idle, timeout when pending
    std::thread::spawn(move || {
        let quiet = Duration::from_millis(200);
        let mut pending = false;

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
                        pending = true;
                    }
                }
                Ok(Err(_e)) => {
                    // Treat watcher errors as meaningful: schedule a refresh after quiet period
                    pending = true;
                }
                Err(std_mpsc::RecvTimeoutError::Timeout) => {
                    if pending {
                        let _ = tx_ui.send(crate::tui::message::Message::Refresh);
                        pending = false;
                    }
                }
                Err(std_mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(watcher)
}

fn is_meaningful(event: &Event) -> bool {
    // Ignore lock files created by git for atomic updates
    let has_lock = event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".lock"))
    });
    if has_lock {
        return false;
    }

    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}
