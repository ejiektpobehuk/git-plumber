use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

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
        /// The type of objects to list (e.g., 'pack')
        #[arg(default_value = "all")]
        object_type: String,
    },

    /// View a pack file
    Pack {
        /// Path to the pack file
        #[arg(required = false)]
        file: Option<PathBuf>,
    },
}

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
            if object_type == "pack" {
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
            } else {
                // For now, just list pack files for "all" type too
                match plumber.list_pack_files() {
                    Ok(pack_files) => {
                        println!("Pack files:");
                        if pack_files.is_empty() {
                            println!("  No pack files found");
                        } else {
                            for file in pack_files {
                                println!("  {}", file.display());
                            }
                        }
                        Ok(())
                    }
                    Err(e) => Err(format!("Error listing files: {}", e)),
                }
            }
        }
        Some(Commands::Pack { file }) => {
            match file {
                Some(file_path) => {
                    // Parse the specified pack file
                    plumber.parse_pack_file(file_path)
                }
                None => {
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
                        Err(e) => Err(format!("Error listing pack files: {}", e)),
                    }
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
