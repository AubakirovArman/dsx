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
    #[arg(short, long)]
    pub mode: Option<String>,

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
    /// Run benchmark/evaluation tasks from a JSON file
    Eval {
        /// JSON file containing one EvalTask or an array of EvalTask objects
        tasks_file: PathBuf,
        /// Only verify expected files/tests; do not call the model
        #[arg(long)]
        no_agent: bool,
    },
    /// Diagnose workspace readiness and local safety invariants
    Doctor,
    /// Preview the active task scope without calling the model
    Scope {
        /// Task text to resolve against the launch workspace
        #[arg(required = true)]
        task: Vec<String>,
    },
    /// Preview the compact model context for a task without calling the model
    Context {
        /// Task text to resolve and assemble context for
        #[arg(required = true)]
        task: Vec<String>,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Exit with an error when the estimated request exceeds the budget
        #[arg(long)]
        check: bool,
    },
    /// Build or query the local code index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
    /// Inspect or call tools from an MCP stdio server
    Mcp {
        #[command(subcommand)]
        action: McpAction,
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
    /// List recent agent run ledger rows
    Runs {
        /// Maximum runs to show
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
        /// Include .dsx run ledgers in child task scopes
        #[arg(long)]
        all: bool,
    },
    /// Show compact task notes saved for workspace scopes
    Notes {
        /// Maximum scopes to show
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
        /// Include direct child folders under the launch workspace
        #[arg(long)]
        all: bool,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Mark unfinished running run rows as stale across scopes
    CloseStaleRuns {
        /// Only close runs older than this many minutes
        #[arg(long, default_value_t = 60)]
        older_than_minutes: i64,
        /// Count matching rows without updating them
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum IndexAction {
    /// Build the SQLite symbol index for the workspace
    Build,
    /// Search symbols and file contents
    Search {
        /// Query text
        query: String,
        /// Maximum results per result type
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
    },
}

#[derive(Subcommand, Clone)]
pub enum McpAction {
    /// List tools from an MCP stdio server
    List {
        /// Server executable
        command: String,
        /// Arguments passed to the server executable
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Call one tool on an MCP stdio server
    Call {
        /// Tool name
        tool: String,
        /// Tool arguments as a JSON object
        arguments_json: String,
        /// Server executable
        command: String,
        /// Arguments passed to the server executable
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}
