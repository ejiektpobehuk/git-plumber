use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

pub mod formatters;

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

    /// View a pack file
    Pack {
        /// Path to the pack file
        #[arg(required = false)]
        file: Option<PathBuf>,
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
        Some(Commands::Pack { file }) => {
            file.as_ref().map_or_else(
                || {
                    // No file specified, list available pack files
                    match plumber.list_pack_files() {
                        Ok(pack_files) => {
                            if pack_files.is_empty() {
                                println!("No pack files found");
                            } else {
                                println!("Available pack files:");
                                for (i, file) in pack_files.iter().enumerate() {
                                    println!("{}. {}", i + 1, file.display());
                                }
                                println!("\nUse 'pack <file_path>' to view a specific pack file");
                            }
                            Ok(())
                        }
                        Err(e) => Err(format!("Error listing pack files: {e}")),
                    }
                },
                |file_path| {
                    // Parse the specified pack file with rich formatting
                    plumber.parse_pack_file_rich(file_path)
                },
            )
        }
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}
