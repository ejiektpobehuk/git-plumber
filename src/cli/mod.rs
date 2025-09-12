use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;

pub mod formatters;

/// Safe print function that handles broken pipe errors gracefully
///
/// When output is piped to commands like `head` that close early,
/// subsequent writes will fail with a broken pipe error. This is
/// expected behavior and should not cause the program to panic.
pub fn safe_print(content: &str) -> Result<(), String> {
    match io::stdout().write_all(content.as_bytes()) {
        Ok(()) => {
            // Attempt to flush, but ignore broken pipe errors
            match io::stdout().flush() {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
                Err(e) => Err(format!("Error flushing stdout: {e}")),
            }
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(format!("Error writing to stdout: {e}")),
    }
}

/// Safe println function that handles broken pipe errors gracefully
pub fn safe_println(content: &str) -> Result<(), String> {
    safe_print(&format!("{content}\n"))
}

/// Determine if a string looks like a git object hash
fn is_likely_hash(input: &str) -> bool {
    // Must be 4-40 characters and all hex
    if input.len() < 4 || input.len() > 40 {
        return false;
    }

    // Check if all characters are valid hex
    input.chars().all(|c| c.is_ascii_hexdigit())
}

/// Determine if a string looks like a file path
fn is_likely_path(input: &str) -> bool {
    // Contains path separators or file extensions
    input.contains('/') || input.contains('\\') || input.contains('.')
}

#[derive(Parser)]
#[command(name = "git-plumber")]
#[command(about = "Explorer for git internals, the plumbing", long_about = None)]
pub struct Cli {
    #[arg(long = "repo", short = 'r', default_value = ".", global = true)]
    pub repo_path: PathBuf,

    /// Show version information
    #[arg(long = "version", short = 'v', action = clap::ArgAction::SetTrue)]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show configuration file locations and current values
    Show,
    /// Create a default configuration file
    Init,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the TUI interface
    Tui {
        /// Reduce motion/animations in the TUI
        #[arg(long = "reduced-motion", short = 'm', action = clap::ArgAction::SetTrue)]
        reduced_motion: bool,
        /// Animation duration in seconds (overrides config file)
        #[arg(long = "animation-duration")]
        animation_duration: Option<u64>,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
    },

    /// List objects
    List {
        #[arg(
            default_value = "all",
            help = "The type of objects to list:\n  pack  - for pack files only\n  loose - for loose objects only\n  all   - for everything supported\n"
        )]
        object_type: String,
    },

    /// View an object or file with detailed formatting
    View {
        /// Object hash (4-40 hex chars) or file path to view
        #[arg(
            required = true,
            help = "Object hash (4-40 characters) or path to file"
        )]
        target: String,
    },
}

/// Run the CLI application
///
/// # Errors
///
/// This function will return an error if:
/// - The repository is not a valid git repository
/// - Pack files cannot be read or parsed
/// - File system operations fail
/// - Command parsing fails
pub fn run() -> Result<(), String> {
    let cli = Cli::parse();

    // Handle version flag first
    if cli.version {
        safe_print(&crate::version::get_version_info().to_string())?;
        return Ok(());
    }

    // Load configuration from files and environment
    let config = crate::config::GitPlumberConfig::load()
        .map_err(|e| format!("Failed to load configuration: {e}"))?;

    let plumber = crate::GitPlumber::new(&cli.repo_path);

    match &cli.command {
        Some(Commands::Tui {
            reduced_motion,
            animation_duration,
        }) => {
            // CLI arguments override config file values
            let final_reduced_motion = *reduced_motion || config.tui.reduced_motion;
            let final_animation_duration =
                animation_duration.unwrap_or(config.tui.animation_duration_secs);

            crate::tui::run_tui(
                plumber,
                crate::tui::RunOptions {
                    reduced_motion: final_reduced_motion,
                    animation_duration_secs: final_animation_duration,
                },
            )
        }
        Some(Commands::Config { config_command }) => match config_command {
            ConfigCommands::Show => {
                crate::config::GitPlumberConfig::print_config_info();
                println!("\nCurrent configuration:");
                println!(
                    "  Animation duration: {} seconds",
                    config.tui.animation_duration_secs
                );
                println!("  Reduced motion: {}", config.tui.reduced_motion);
                Ok(())
            }
            ConfigCommands::Init => {
                match crate::config::GitPlumberConfig::create_default_config_file() {
                    Ok(path) => {
                        println!("Created default configuration file at: {}", path.display());
                        println!("You can edit this file to customize git-plumber settings.");
                        Ok(())
                    }
                    Err(e) => Err(format!("Failed to create config file: {e}")),
                }
            }
        },
        Some(Commands::List { object_type }) => {
            match object_type.as_str() {
                "pack" => {
                    // List pack files only
                    match plumber.list_pack_files() {
                        Ok(pack_files) => {
                            if pack_files.is_empty() {
                                safe_println("No pack files found")?;
                            } else {
                                safe_println(&format!("Found {} pack files:", pack_files.len()))?;
                                for (i, file) in pack_files.iter().enumerate() {
                                    safe_println(&format!("{}. {}", i + 1, file.display()))?;
                                }
                            }
                            Ok(())
                        }
                        Err(e) => Err(format!("Error listing pack files: {e}")),
                    }
                }
                "loose" => {
                    // List loose objects only
                    match plumber.get_loose_object_stats() {
                        Ok(stats) => {
                            safe_println("Loose object statistics:")?;
                            safe_println(&stats.summary())?;
                            safe_println("")?;

                            // Show all loose objects
                            match plumber.list_parsed_loose_objects(stats.total_count) {
                                Ok(loose_objects) => {
                                    if loose_objects.is_empty() {
                                        safe_println("No loose objects found")?;
                                    } else {
                                        safe_println("Loose objects:")?;
                                        for (i, obj) in loose_objects.iter().enumerate() {
                                            let (short_hash, rest_hash) = obj.object_id.split_at(8);
                                            safe_println(&format!(
                                                "{}. \x1b[1m{}\x1b[22m{} ({}) - {} bytes",
                                                i + 1,
                                                short_hash,
                                                rest_hash,
                                                obj.object_type,
                                                obj.size
                                            ))?;
                                        }
                                    }
                                    Ok(())
                                }
                                Err(e) => Err(format!("Error listing loose objects: {e}")),
                            }
                        }
                        Err(e) => Err(format!("Error getting loose object stats: {e}")),
                    }
                }
                _ => {
                    // List all object types
                    let mut has_error = false;
                    let mut error_messages = Vec::new();

                    // List pack files
                    safe_println("Pack files:")?;
                    match plumber.list_pack_files() {
                        Ok(pack_files) => {
                            if pack_files.is_empty() {
                                safe_println("  No pack files found")?;
                            } else {
                                for file in pack_files {
                                    safe_println(&format!("  {}", file.display()))?;
                                }
                            }
                        }
                        Err(e) => {
                            has_error = true;
                            error_messages.push(format!("Error listing pack files: {e}"));
                            safe_println(&format!("  Error listing pack files: {e}"))?;
                        }
                    }

                    safe_println("")?;

                    // List loose objects
                    safe_println("Loose objects:")?;
                    match plumber.get_loose_object_stats() {
                        Ok(stats) => {
                            safe_println(&format!("  {}", stats.summary().replace('\n', "\n  ")))?;
                        }
                        Err(e) => {
                            has_error = true;
                            error_messages.push(format!("Error getting loose object stats: {e}"));
                            safe_println(&format!("  Error getting loose object stats: {e}"))?;
                        }
                    }

                    if has_error {
                        Err(error_messages.join("; "))
                    } else {
                        Ok(())
                    }
                }
            }
        }
        Some(Commands::View { target }) => {
            // Determine if target is a hash or path
            if is_likely_path(target) && !is_likely_hash(target) {
                // Treat as file path
                let path = PathBuf::from(target);
                if path.exists() {
                    // Check if it's a pack file or other git object file
                    if path.extension().and_then(|s| s.to_str()) == Some("pack") {
                        plumber.parse_pack_file_rich(&path)
                    } else {
                        // Try to parse as loose object file
                        plumber.view_file_as_object(&path)
                    }
                } else {
                    Err(format!("File not found: {}", path.display()))
                }
            } else if is_likely_hash(target) {
                // Treat as object hash
                plumber.view_object_by_hash(target)
            } else {
                // Ambiguous - try both approaches
                let path = PathBuf::from(target);
                if path.exists() {
                    // File exists, treat as path
                    if path.extension().and_then(|s| s.to_str()) == Some("pack") {
                        plumber.parse_pack_file_rich(&path)
                    } else {
                        plumber.view_file_as_object(&path)
                    }
                } else if target.chars().all(|c| c.is_ascii_hexdigit()) {
                    // Looks like hex but too short or too long
                    if target.len() < 4 {
                        Err(format!(
                            "Hash too short: '{target}'. Git object hashes must be at least 4 characters long."
                        ))
                    } else if target.len() > 40 {
                        Err(format!(
                            "Hash too long: '{target}'. Git object hashes must be at most 40 characters long."
                        ))
                    } else {
                        // Valid length hex but object not found
                        plumber.view_object_by_hash(target)
                    }
                } else {
                    Err(format!(
                        "Invalid target: '{target}' is neither a valid file path nor object hash (hashes must be 4-40 hex characters)"
                    ))
                }
            }
        }
        None => {
            // Default to TUI mode with configuration values
            crate::tui::run_tui(
                plumber,
                crate::tui::RunOptions {
                    reduced_motion: config.tui.reduced_motion,
                    animation_duration_secs: config.tui.animation_duration_secs,
                },
            )
        }
    }
}
