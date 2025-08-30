use std::fmt;

// Include the generated build information
pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub struct VersionInfo {
    pub version: &'static str,
    pub commit_hash: Option<&'static str>,
    pub commit_hash_short: Option<&'static str>,
    pub git_version: Option<&'static str>,
    pub is_dirty: bool,
    pub target: &'static str,
    pub profile: &'static str,
    pub rustc_version: &'static str,
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionInfo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: built_info::PKG_VERSION,
            commit_hash: built_info::GIT_COMMIT_HASH,
            commit_hash_short: built_info::GIT_COMMIT_HASH_SHORT,
            git_version: built_info::GIT_VERSION,
            is_dirty: built_info::GIT_DIRTY.unwrap_or(false),
            target: built_info::TARGET,
            profile: built_info::PROFILE,
            rustc_version: built_info::RUSTC_VERSION,
        }
    }

    #[must_use]
    pub fn is_development_build(&self) -> bool {
        // Consider it a dev build if:
        // 1. The working directory is dirty, OR
        // 2. We have commits after the last release tag, OR
        // 3. Building in debug mode
        self.is_dirty
            || self.profile == "debug"
            || self.git_version.is_some_and(|git_ver| {
                // Parse git version like "v0.1.0-15-ge1c9641" to detect commits after tag
                git_ver.contains('-') && !git_ver.ends_with("-0-g")
            })
    }

    #[must_use]
    pub fn short_version(&self) -> String {
        if self.is_development_build() {
            if let Some(git_version) = self.git_version {
                // Parse the git version string like "v0.1.0-15-ge1c9641"
                if git_version.contains('-') {
                    let parts: Vec<&str> = git_version.split('-').collect();
                    if parts.len() >= 3 {
                        let commits_after = parts[parts.len() - 2];
                        let commit_hash = parts[parts.len() - 1]
                            .strip_prefix('g')
                            .unwrap_or(parts[parts.len() - 1]);
                        if self.is_dirty {
                            format!(
                                "v{}-dev+{}.{}.dirty",
                                self.version, commits_after, commit_hash
                            )
                        } else {
                            format!("v{}-dev+{}.{}", self.version, commits_after, commit_hash)
                        }
                    } else if self.is_dirty {
                        format!("v{}-dev+dirty", self.version)
                    } else {
                        format!("v{}-dev", self.version)
                    }
                } else if let Some(hash) = self.commit_hash_short {
                    if self.is_dirty {
                        format!("v{}-dev+dirty.{}", self.version, hash)
                    } else {
                        format!("v{}-dev+{}", self.version, hash)
                    }
                } else {
                    format!("v{}-dev", self.version)
                }
            } else if let Some(hash) = self.commit_hash_short {
                if self.is_dirty {
                    format!("v{}-dev+dirty.{}", self.version, hash)
                } else {
                    format!("v{}-dev+{}", self.version, hash)
                }
            } else {
                format!("v{}-dev", self.version)
            }
        } else {
            format!("v{}", self.version)
        }
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "git-plumber\n\nVersion: {}", self.short_version())?;

        if self.is_development_build() {
            if let Some(git_version) = self.git_version {
                writeln!(f, "Git version notation: {git_version}")?;
            }
            if let Some(commit_hash) = self.commit_hash {
                writeln!(f, "Commit hash: {commit_hash}")?;
            }
            if let Some(commits_ahead) = self
                .git_version
                .and_then(|v| v.split('-').nth(1).and_then(|n| n.parse::<u32>().ok()))
            {
                writeln!(f, "Commits since last tag: {commits_ahead}")?;
            }

            if self.is_dirty {
                writeln!(f, "Working directory: dirty")?;
            }

            writeln!(f, "Profile: {}", self.profile)?;
            writeln!(f, "Target: {}", self.target)?;
            writeln!(f, "Rust: {}", self.rustc_version)?;
        }

        Ok(())
    }
}

#[must_use]
pub fn get_version_info() -> VersionInfo {
    VersionInfo::new()
}
