use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration structure for git-plumber
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitPlumberConfig {
    /// TUI-related configuration
    pub tui: TuiConfig,
}

/// TUI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Animation duration in seconds (default: 10)
    pub animation_duration_secs: u64,
    /// Whether to reduce motion/animations (default: false)
    pub reduced_motion: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            animation_duration_secs: 10,
            reduced_motion: false,
        }
    }
}

impl GitPlumberConfig {
    /// Load configuration from various sources in order of priority:
    /// 1. Command line arguments (handled elsewhere)
    /// 2. Environment variables
    /// 3. User config file (~/.config/git-plumber/config.toml)
    /// 4. System config file (/etc/git-plumber/config.toml)
    /// 5. Default values
    pub fn load() -> Result<Self, ConfigError> {
        let mut config_builder = Config::builder();

        // Start with default values
        config_builder = config_builder.add_source(Config::try_from(&Self::default())?);

        // Add system config file if it exists
        if let Some(system_config_path) = Self::get_system_config_path()
            && system_config_path.exists()
        {
            config_builder =
                config_builder.add_source(File::from(system_config_path).required(false));
        }

        // Add user config file if it exists
        if let Some(user_config_path) = Self::get_user_config_path()
            && user_config_path.exists()
        {
            config_builder =
                config_builder.add_source(File::from(user_config_path).required(false));
        }

        // Add environment variables with prefix "GIT_PLUMBER_"
        config_builder = config_builder.add_source(
            config::Environment::with_prefix("GIT_PLUMBER")
                .separator("_")
                .try_parsing(true),
        );

        let config = config_builder.build()?;
        config.try_deserialize()
    }

    /// Get the path to the user configuration file
    /// Returns: ~/.config/git-plumber/config.toml (on Linux/macOS)
    ///          %APPDATA%/git-plumber/config.toml (on Windows)
    #[must_use]
    pub fn get_user_config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "git-plumber")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
    }

    /// Get the path to the system configuration file
    /// Returns: /etc/git-plumber/config.toml (on Unix-like systems)
    ///          %PROGRAMDATA%/git-plumber/config.toml (on Windows)
    #[must_use]
    pub fn get_system_config_path() -> Option<PathBuf> {
        // On Unix-like systems, use /etc/git-plumber/config.toml
        #[cfg(unix)]
        {
            Some(PathBuf::from("/etc/git-plumber/config.toml"))
        }

        // On Windows, use %PROGRAMDATA%/git-plumber/config.toml
        #[cfg(windows)]
        {
            std::env::var("PROGRAMDATA").ok().map(|program_data| {
                PathBuf::from(program_data)
                    .join("git-plumber")
                    .join("config.toml")
            })
        }

        // Fallback for other platforms
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }

    /// Create a default configuration file at the user config location
    pub fn create_default_config_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
        if let Some(config_path) = Self::get_user_config_path() {
            // Create the parent directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Create the default config content
            let default_config = Self::default();
            let toml_content = toml::to_string_pretty(&default_config)?;

            // Write the file
            std::fs::write(&config_path, toml_content)?;
            Ok(config_path)
        } else {
            Err("Could not determine user config directory".into())
        }
    }

    /// Print information about config file locations
    pub fn print_config_info() {
        println!("Git Plumber Configuration");
        println!("========================");

        if let Some(user_path) = Self::get_user_config_path() {
            println!("User config file: {}", user_path.display());
            if user_path.exists() {
                println!("  Status: ✓ Found");
            } else {
                println!("  Status: ✗ Not found (will use defaults)");
            }
        }

        if let Some(system_path) = Self::get_system_config_path() {
            println!("System config file: {}", system_path.display());
            if system_path.exists() {
                println!("  Status: ✓ Found");
            } else {
                println!("  Status: ✗ Not found");
            }
        }

        println!("\nEnvironment variables:");
        println!("  GIT_PLUMBER_TUI_ANIMATION_DURATION_SECS");
        println!("  GIT_PLUMBER_TUI_REDUCED_MOTION");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GitPlumberConfig::default();
        assert_eq!(config.tui.animation_duration_secs, 10);
        assert!(!config.tui.reduced_motion);
    }

    #[test]
    fn test_config_serialization() {
        let config = GitPlumberConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: GitPlumberConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.tui.animation_duration_secs,
            config.tui.animation_duration_secs
        );
        assert_eq!(parsed.tui.reduced_motion, config.tui.reduced_motion);
    }
}
