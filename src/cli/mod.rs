use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

pub mod formatters;

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
pub enum Commands {
    /// Start the TUI interface
    Tui {},

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
        print!("{}", crate::version::get_version_info());
        return Ok(());
    }

    let plumber = crate::GitPlumber::new(&cli.repo_path);

    match &cli.command {
        Some(Commands::Tui {}) => {
            // This will be implemented later
            crate::tui::run_tui(plumber)
        }
        Some(Commands::List { object_type }) => {
            match object_type.as_str() {
                "pack" => {
                    // List pack files only
                    match plumber.list_pack_files() {
                        Ok(pack_files) => {
                            if pack_files.is_empty() {
                                println!("No pack files found");
                            } else {
                                println!("Found {} pack files:", pack_files.len());
                                for (i, file) in pack_files.iter().enumerate() {
                                    println!("{}. {}", i + 1, file.display());
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
                            println!("Loose object statistics:");
                            println!("{}", stats.summary());
                            println!();

                            // Show all loose objects
                            match plumber.list_parsed_loose_objects(stats.total_count) {
                                Ok(loose_objects) => {
                                    if loose_objects.is_empty() {
                                        println!("No loose objects found");
                                    } else {
                                        println!("Loose objects:");
                                        for (i, obj) in loose_objects.iter().enumerate() {
                                            let (short_hash, rest_hash) = obj.object_id.split_at(8);
                                            println!(
                                                "{}. \x1b[1m{}\x1b[22m{} ({}) - {} bytes",
                                                i + 1,
                                                short_hash,
                                                rest_hash,
                                                obj.object_type,
                                                obj.size
                                            );
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
                    println!("Pack files:");
                    match plumber.list_pack_files() {
                        Ok(pack_files) => {
                            if pack_files.is_empty() {
                                println!("  No pack files found");
                            } else {
                                for file in pack_files {
                                    println!("  {}", file.display());
                                }
                            }
                        }
                        Err(e) => {
                            has_error = true;
                            error_messages.push(format!("Error listing pack files: {e}"));
                            println!("  Error listing pack files: {e}");
                        }
                    }

                    println!();

                    // List loose objects
                    println!("Loose objects:");
                    match plumber.get_loose_object_stats() {
                        Ok(stats) => {
                            println!("  {}", stats.summary().replace('\n', "\n  "));
                        }
                        Err(e) => {
                            has_error = true;
                            error_messages.push(format!("Error getting loose object stats: {e}"));
                            println!("  Error getting loose object stats: {e}");
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
            let mut cmd = Cli::command();
            cmd.print_help().map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}
