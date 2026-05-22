//! Main binary runtime helpers kept out of the entrypoint.

use crate::cli::{CliArgs, IndexAction, McpAction, WorkspaceAction};
use crate::handlers::{
    list_sessions, run_index_build, run_index_search, run_mcp_call, run_mcp_list,
};
use crate::workspace_notes::list_workspace_notes;
use crate::workspace_runs::list_agent_runs;
use crate::workspace_stale_runs::close_stale_runs;

pub fn load_config_or_default(project_root: &std::path::Path) -> dsx_config::AppConfig {
    match dsx_config::load_for_project(project_root) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: failed to load config: {e}");
            dsx_config::AppConfig::default()
        }
    }
}

pub fn initial_mode(
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

pub fn api_key(cli: &CliArgs, app_config: &dsx_config::AppConfig) -> Option<String> {
    cli.api_key
        .clone()
        .or_else(|| std::env::var(&app_config.provider.api_key_env).ok())
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
}

pub fn require_api_key(api_key: Option<String>) -> Option<String> {
    if api_key.is_none() {
        println!("(Set DEEPSEEK_API_KEY or use --api-key)");
    }
    api_key
}

pub async fn create_session(
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

pub async fn run_index_action(
    project_root: &std::path::Path,
    action: IndexAction,
) -> anyhow::Result<()> {
    match action {
        IndexAction::Build => run_index_build(project_root).await,
        IndexAction::Search { query, limit } => run_index_search(project_root, &query, limit).await,
    }
}

pub async fn run_mcp_action(action: McpAction) -> anyhow::Result<()> {
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

pub async fn run_workspace_action(
    project_root: std::path::PathBuf,
    api_key: Option<String>,
    api_base: String,
    mode: dsx_core::types::PermissionMode,
    allow_wide_scope: bool,
    action: Option<WorkspaceAction>,
) -> anyhow::Result<()> {
    match action {
        None | Some(WorkspaceAction::List) => list_sessions(&project_root).await,
        Some(WorkspaceAction::Runs { limit, all }) => {
            list_agent_runs(&project_root, limit, all).await
        }
        Some(WorkspaceAction::Audit { limit, all, json }) => {
            crate::workspace_audit::run_workspace_audit(&project_root, limit, all, json).await
        }
        Some(WorkspaceAction::Notes { limit, all, json }) => {
            list_workspace_notes(&project_root, limit, all, json).await
        }
        Some(WorkspaceAction::Mission { limit, all, json }) => {
            crate::workspace_mission::run_workspace_mission(&project_root, limit, all, json).await
        }
        Some(WorkspaceAction::CloseStaleRuns {
            older_than_minutes,
            dry_run,
        }) => close_stale_runs(&project_root, older_than_minutes, dry_run).await,
        Some(WorkspaceAction::Resume { id }) => {
            let Some(key) = require_api_key(api_key) else {
                return Ok(());
            };
            resume_session(project_root, key, api_base, mode, allow_wide_scope, id).await?;
        }
    }
    Ok(())
}

async fn resume_session(
    project_root: std::path::PathBuf,
    api_key: String,
    api_base: String,
    fallback_mode: dsx_core::types::PermissionMode,
    allow_wide_scope: bool,
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
            crate::tui_runner::run_tui(
                project_root,
                api_key,
                api_base,
                mode,
                allow_wide_scope,
                Some(session.id),
                Some(pool),
            )
            .await?;
        }
        _ => println!("Error: Session with ID '{}' not found.", id),
    }
    Ok(())
}
