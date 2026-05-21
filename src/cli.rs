//! DSX CLI — command line parser and options.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// DSX Code — a terminal coding agent powered by DeepSeek V4.
#[derive(Parser)]
#[command(name = "dsx", version, about, long_about = None)]
pub struct CliArgs {
    /// Workspace root directory
    #[arg(short, long, default_value = ".")]
    pub workspace: PathBuf,

    /// Override permission mode
    #[arg(short, long, default_value = "ask")]
    pub mode: String,

    /// Optional DeepSeek API key
    #[arg(short, long)]
    pub api_key: Option<String>,

    /// Override the API base URL (defaults to https://api.deepseek.com)
    #[arg(short = 'b', long)]
    pub api_base: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run in planning mode
    Plan {
        /// Description of the planning task
        #[arg(required = true)]
        task: Vec<String>,
    },
    /// Run in direct edit mode
    Edit {
        /// Description of the task to edit
        #[arg(required = true)]
        task: Vec<String>,
    },
    /// Interactive TUI mode (default)
    Interactive,
    /// Workspace and sessions operations
    Workspace {
        #[command(subcommand)]
        action: Option<WorkspaceAction>,
    },
}

#[derive(Subcommand, Clone)]
pub enum WorkspaceAction {
    /// List recent sessions
    List,
    /// Resume a previous session
    Resume {
        /// Session ID to resume
        #[arg(required = true)]
        id: String,
    },
}
