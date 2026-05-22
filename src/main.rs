//! DSX Code — terminal coding agent entrypoint.
pub mod agent_preflight;
#[cfg(test)]
mod agent_preflight_tests;
pub mod cli;
pub mod cli_context_budget;
#[cfg(test)]
mod cli_tests;
pub mod context_budget_advice;
pub mod context_capsule;
#[cfg(test)]
mod context_capsule_tests;
pub mod context_preview;
pub mod context_preview_metrics;
pub mod context_preview_output;
#[cfg(test)]
mod context_preview_tests;
pub mod doctor;
pub mod event_convert;
pub mod handlers;
pub mod line_limit;
pub mod main_runtime;
pub mod run_ledger;
pub mod scope_guard;
pub mod session_state;
#[cfg(test)]
mod session_state_tests;
pub mod task_scope;
#[cfg(test)]
mod task_scope_tests;
pub mod tui_context_budget;
#[cfg(test)]
mod tui_context_budget_tests;
pub mod tui_keys;
#[cfg(test)]
mod tui_keys_tests;
pub mod tui_preflight;
pub mod tui_run_ledger;
pub mod tui_runner;
pub mod tui_scope_guard;
pub mod tui_settings_keys;
pub mod tui_state;
#[cfg(test)]
mod tui_state_tests;
pub mod tui_task;
#[cfg(test)]
mod tui_task_tests;
pub mod workspace_audit;
pub mod workspace_notes;
#[cfg(test)]
mod workspace_notes_tests;
pub mod workspace_runs;
pub mod workspace_stale_runs;

use agent_preflight::run_agent_preflight;
use clap::Parser;
use cli::{CliArgs, Command};
use context_capsule::run_context_capsule;
use context_preview::run_context_preview;
use handlers::{run_edit, run_eval, run_plan, run_scope_preview, task_preview};
use main_runtime::{
    api_key, create_session, initial_mode, load_config_or_default, require_api_key,
    run_index_action, run_mcp_action, run_workspace_action,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CliArgs::parse();
    let project_root =
        std::fs::canonicalize(&cli.workspace).unwrap_or_else(|_| cli.workspace.clone());
    let app_config = load_config_or_default(&project_root);
    let mode = initial_mode(&cli, &app_config);
    let api_key = api_key(&cli, &app_config);
    let allow_wide_scope = cli.allow_wide_scope || app_config.scope.allow_wide;
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
                allow_wide_scope,
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
            run_plan(project_root, key, api_base, &desc, mode, allow_wide_scope).await?;
        }
        Some(Command::Edit { task }) => {
            let Some(key) = require_api_key(api_key) else {
                return Ok(());
            };
            let desc = task.join(" ");
            println!("Editing: {}", task_preview(&desc));
            run_edit(project_root, key, api_base, &desc, mode, allow_wide_scope).await?;
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
        Some(Command::Preflight { task, json, check }) => {
            let desc = task.join(" ");
            run_agent_preflight(&project_root, &desc, allow_wide_scope, json, check)?;
        }
        Some(Command::Context {
            task,
            json,
            check,
            require_narrow,
        }) => {
            let desc = task.join(" ");
            run_context_preview(&project_root, &desc, json, check, require_narrow).await?;
        }
        Some(Command::Capsule { task, limit, json }) => {
            let desc = task.join(" ");
            run_context_capsule(&project_root, &desc, limit, json).await?;
        }
        Some(Command::Index { action }) => run_index_action(&project_root, action).await?,
        Some(Command::Mcp { action }) => run_mcp_action(action).await?,
        Some(Command::Workspace { action }) => {
            run_workspace_action(
                project_root,
                api_key,
                api_base,
                mode,
                allow_wide_scope,
                action,
            )
            .await?
        }
    }
    Ok(())
}
