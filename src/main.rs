//! DSX Code — terminal coding agent entrypoint.

pub mod cli;
pub mod context_preview;
#[cfg(test)]
mod context_preview_tests;
pub mod doctor;
pub mod event_convert;
pub mod handlers;
pub mod run_ledger;
pub mod session_state;
#[cfg(test)]
mod session_state_tests;
pub mod task_scope;
#[cfg(test)]
mod task_scope_tests;
pub mod tui_keys;
#[cfg(test)]
mod tui_keys_tests;
pub mod tui_runner;
pub mod tui_settings_keys;
pub mod tui_state;
#[cfg(test)]
mod tui_state_tests;
pub mod tui_task;
#[cfg(test)]
mod tui_task_tests;
pub mod workspace_notes;
#[cfg(test)]
mod workspace_notes_tests;
pub mod workspace_runs;
pub mod workspace_stale_runs;

use clap::Parser;
use cli::{CliArgs, Command, IndexAction, McpAction, WorkspaceAction};
use context_preview::run_context_preview;
use handlers::{
    list_sessions, run_edit, run_eval, run_index_build, run_index_search, run_mcp_call,
    run_mcp_list, run_plan, run_scope_preview, task_preview,
};
use workspace_notes::list_workspace_notes;
use workspace_runs::list_agent_runs;
use workspace_stale_runs::close_stale_runs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CliArgs::parse();
    let project_root =
        std::fs::canonicalize(&cli.workspace).unwrap_or_else(|_| cli.workspace.clone());
    let app_config = load_config_or_default(&project_root);
    let mode = initial_mode(&cli, &app_config);
    let api_key = api_key(&cli, &app_config);
    let api_base = cli
        .api_base
        .clone()
        .unwrap_or_else(|| app_config.provider.openai_base_url.clone());

    match cli.command {
        None | Some(Command::Interactive) => {
            let (pool, session_id) = create_session(&project_root, mode).await;
            tui_runner::run_tui(
                project_root,
                api_key.unwrap_or_default(),
                api_base,
                mode,
                session_id,
                pool,
            )
            .await?;
        }
        Some(Command::Plan { task }) => {
            let Some(key) = require_api_key(api_key) else {
                return Ok(());
            };
            let desc = task.join(" ");
            println!("Planning: {}", task_preview(&desc));
            run_plan(project_root, key, api_base, &desc, mode).await?;
        }
        Some(Command::Edit { task }) => {
            let Some(key) = require_api_key(api_key) else {
                return Ok(());
            };
            let desc = task.join(" ");
            println!("Editing: {}", task_preview(&desc));
            run_edit(project_root, key, api_base, &desc, mode).await?;
        }
        Some(Command::Eval {
            tasks_file,
            no_agent,
        }) => run_eval(project_root, api_key, api_base, tasks_file, mode, no_agent).await?,
        Some(Command::Doctor) => {
            doctor::run_doctor(&project_root, &api_base, api_key.as_deref()).await?
        }
        Some(Command::Scope { task }) => {
            let desc = task.join(" ");
            run_scope_preview(&project_root, &desc);
        }
        Some(Command::Context { task, json, check }) => {
            let desc = task.join(" ");
            run_context_preview(&project_root, &desc, json, check).await?;
        }
        Some(Command::Index { action }) => run_index_action(&project_root, action).await?,
        Some(Command::Mcp { action }) => run_mcp_action(action).await?,
        Some(Command::Workspace { action }) => {
            run_workspace_action(project_root, api_key, api_base, mode, action).await?
        }
    }
    Ok(())
}

fn load_config_or_default(project_root: &std::path::Path) -> dsx_config::AppConfig {
    match dsx_config::load_for_project(project_root) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: failed to load config: {e}");
            dsx_config::AppConfig::default()
        }
    }
}

fn initial_mode(
    cli: &CliArgs,
    app_config: &dsx_config::AppConfig,
) -> dsx_core::types::PermissionMode {
    let mode_name = cli
        .mode
        .as_deref()
        .unwrap_or(app_config.app.default_mode.as_str());
    dsx_core::types::PermissionMode::parse(mode_name)
        .unwrap_or(dsx_core::types::PermissionMode::Ask)
}

fn api_key(cli: &CliArgs, app_config: &dsx_config::AppConfig) -> Option<String> {
    cli.api_key
        .clone()
        .or_else(|| std::env::var(&app_config.provider.api_key_env).ok())
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
}

fn require_api_key(api_key: Option<String>) -> Option<String> {
    if api_key.is_none() {
        println!("(Set DEEPSEEK_API_KEY or use --api-key)");
    }
    api_key
}

async fn create_session(
    project_root: &std::path::Path,
    mode: dsx_core::types::PermissionMode,
) -> (Option<sqlx::SqlitePool>, Option<String>) {
    let db_path = project_root.join(".dsx").join("sessions.db");
    match dsx_memory::open(&db_path).await {
        Ok(pool) => {
            let sm = dsx_session::SessionManager::new(pool.clone());
            match sm
                .create(&project_root.display().to_string(), mode.as_str())
                .await
            {
                Ok(session) => (Some(pool), Some(session.id)),
                Err(_) => (Some(pool), None),
            }
        }
        Err(_) => (None, None),
    }
}

async fn run_index_action(
    project_root: &std::path::Path,
    action: IndexAction,
) -> anyhow::Result<()> {
    match action {
        IndexAction::Build => run_index_build(project_root).await,
        IndexAction::Search { query, limit } => run_index_search(project_root, &query, limit).await,
    }
}

async fn run_mcp_action(action: McpAction) -> anyhow::Result<()> {
    match action {
        McpAction::List { command, args } => run_mcp_list(&command, &args).await,
        McpAction::Call {
            tool,
            arguments_json,
            command,
            args,
        } => run_mcp_call(&command, &args, &tool, &arguments_json).await,
    }
}

async fn run_workspace_action(
    project_root: std::path::PathBuf,
    api_key: Option<String>,
    api_base: String,
    mode: dsx_core::types::PermissionMode,
    action: Option<WorkspaceAction>,
) -> anyhow::Result<()> {
    match action {
        None | Some(WorkspaceAction::List) => list_sessions(&project_root).await,
        Some(WorkspaceAction::Runs { limit, all }) => {
            list_agent_runs(&project_root, limit, all).await
        }
        Some(WorkspaceAction::Notes { limit, all, json }) => {
            list_workspace_notes(&project_root, limit, all, json).await
        }
        Some(WorkspaceAction::CloseStaleRuns {
            older_than_minutes,
            dry_run,
        }) => close_stale_runs(&project_root, older_than_minutes, dry_run).await,
        Some(WorkspaceAction::Resume { id }) => {
            let Some(key) = require_api_key(api_key) else {
                return Ok(());
            };
            resume_session(project_root, key, api_base, mode, id).await?;
        }
    }
    Ok(())
}

async fn resume_session(
    project_root: std::path::PathBuf,
    api_key: String,
    api_base: String,
    fallback_mode: dsx_core::types::PermissionMode,
    id: String,
) -> anyhow::Result<()> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    let pool = match dsx_memory::open(&db_path).await {
        Ok(pool) => pool,
        Err(e) => {
            println!("Error: Failed to open sessions database: {e}");
            return Ok(());
        }
    };
    let sm = dsx_session::SessionManager::new(pool.clone());
    match sm.get(&id).await {
        Ok(Some(session)) => {
            let mode =
                dsx_core::types::PermissionMode::parse(&session.mode).unwrap_or(fallback_mode);
            println!("Resuming session {}...", session.id);
            tui_runner::run_tui(
                project_root,
                api_key,
                api_base,
                mode,
                Some(session.id),
                Some(pool),
            )
            .await?;
        }
        _ => println!("Error: Session with ID '{}' not found.", id),
    }
    Ok(())
}
