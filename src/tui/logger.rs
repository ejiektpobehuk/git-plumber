use once_cell::sync::Lazy;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static LOGGER: Lazy<Mutex<Option<File>>> = Lazy::new(|| Mutex::new(None));

pub fn init_log(repo_path: &Path) {
    if let Some(path) = log_path_for_repo(repo_path) {
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
            if let Ok(mut guard) = LOGGER.lock() {
                *guard = Some(file);
            }
        }
    }
}

fn log_path_for_repo(repo_path: &Path) -> Option<PathBuf> {
    let dot_git = repo_path.join(".git");
    if dot_git.is_dir() {
        Some(dot_git.join("git-plumber.log"))
    } else {
        // .git is a file (worktree or submodule). Place log in repo root.
        Some(repo_path.join("git-plumber.log"))
    }
}

pub fn logln(msg: &str) {
    if let Ok(mut guard) = LOGGER.lock() {
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{msg}");
        }
    }
}
