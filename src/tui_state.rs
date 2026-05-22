//! Shared TUI state bootstrap helpers.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;

pub type SharedApp = Arc<Mutex<dsx_tui::App>>;
pub type HistoryEvents = Option<(String, Vec<dsx_session::Event>)>;

pub async fn load_recent_history(
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
) -> HistoryEvents {
    if let (Some(sid), Some(p)) = (session_id, pool) {
        let sm = dsx_session::SessionManager::new(p);
        sm.get_recent_events(&sid, 24)
            .await
            .ok()
            .map(|events| (sid, events))
    } else {
        None
    }
}

pub fn configure_initial_app(
    app: &SharedApp,
    project_root: &Path,
    initial_mode: dsx_core::types::PermissionMode,
    api_base: String,
    api_key: String,
    allow_wide_scope: bool,
    history_events: HistoryEvents,
) {
    let mut app = app.lock().unwrap();
    app.api_base = api_base;
    app.api_key = api_key;
    app.allow_wide_scope = allow_wide_scope;
    app.mode = initial_mode.as_str().to_string();
    let budget_limits = dsx_agent::budget::current_limits();
    let budget_status = dsx_agent::budget::format_limits(budget_limits);
    app.budget_status = budget_status.clone();
    app.run_budget.max_tokens = budget_limits.max_run_tokens;
    app.run_budget.max_cost_usd = budget_limits.max_run_cost_usd;
    app.scope_lock.launch_scope = project_root.display().to_string();
    app.scope_lock.active_scope = project_root.display().to_string();
    app.add_message(
        "system",
        &format!("Launch workspace: {}", project_root.display()),
    );
    app.add_message("system", &format!("Budget fuse: {budget_status}"));
    app.add_message(
        "system",
        "Semantic indexing deferred until active task scope.",
    );
    if allow_wide_scope {
        app.add_message("system", "Wide scope guard disabled by explicit policy.");
    }
    app.add_message(
        "system",
        &format!(
            "Mode: {} — {}",
            initial_mode.as_str(),
            initial_mode.description()
        ),
    );
    load_history(&mut app, history_events);
    ensure_git(project_root, &mut app);
    if let Ok(files) = dsx_fs::list_files(project_root) {
        app.file_tree = files.into_iter().take(50).collect();
    }
}

pub fn start_active_scope_indexing(app: SharedApp, active_root: PathBuf, rt: &Handle) {
    rt.spawn(async move {
        match index_active_scope(&active_root).await {
            Ok(count) => {
                let mut app = app.lock().unwrap();
                app.add_message(
                    "system",
                    &format!(
                        "✓ Active-scope index complete: {count} symbols in {}.",
                        active_root.display()
                    ),
                );
            }
            Err(e) => {
                app.lock()
                    .unwrap()
                    .add_message("error", &format!("Active-scope indexing failed: {e}"));
            }
        }
    });
}

pub(crate) async fn load_startup_audit(app: &SharedApp, project_root: &Path) {
    match crate::workspace_audit::collect_workspace_audit(project_root, 5, true).await {
        Ok(audit) => apply_startup_audit(app, audit),
        Err(e) => app
            .lock()
            .unwrap()
            .add_message("error", &format!("Workspace audit failed: {e}")),
    }
}

pub(crate) async fn index_active_scope(active_root: &Path) -> anyhow::Result<usize> {
    let db_path = active_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    dsx_index::build_symbol_index(active_root, &pool).await
}

fn load_history(app: &mut dsx_tui::App, history_events: HistoryEvents) {
    let Some((sid, events)) = history_events else {
        return;
    };
    app.add_message("system", &format!("Session ID: {}", sid));
    if events.is_empty() {
        return;
    }
    app.add_message(
        "system",
        &format!(
            "✓ Loaded {} historical message events from SQLite.",
            events.len()
        ),
    );
    for event in events {
        load_history_event(app, &event);
    }
}

fn load_history_event(app: &mut dsx_tui::App, event: &dsx_session::Event) {
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.data_json) else {
        return;
    };
    if event.type_ == "user_msg" {
        if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
            app.add_message("user", &crate::session_state::history_excerpt(content));
        }
    } else if event.type_ == "assistant_msg" {
        if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
            app.add_message("assistant", &crate::session_state::history_excerpt(content));
        }
        if let Some(cost) = data.get("cost").and_then(|v| v.as_f64()) {
            app.cost = cost;
        }
        if let Some(tokens) = data.get("tokens").and_then(|v| v.as_u64()) {
            app.tokens = tokens;
        }
    }
}

fn apply_startup_audit(app: &SharedApp, audit: crate::workspace_audit::WorkspaceAudit) {
    let mut app = app.lock().unwrap();
    app.add_message("system", &startup_audit_line(&audit));
    app.run_ledger = crate::tui_run_health::panel_from_audit(&audit);
    let warning = startup_audit_warning(&audit);
    if !warning.is_empty() {
        app.scope_lock.warning = warning;
    }
}

fn startup_audit_line(audit: &crate::workspace_audit::WorkspaceAudit) -> String {
    format!(
        "Workspace audit: budget={}, running={}, stale>60m={}, scope_guard={}, line-limit={}",
        audit.budget,
        audit.running_runs,
        audit.stale_runs,
        audit.scope_violations,
        if audit.line_violations.is_empty() {
            "ok"
        } else {
            "fail"
        }
    )
}

fn startup_audit_warning(audit: &crate::workspace_audit::WorkspaceAudit) -> String {
    let mut parts = Vec::new();
    if audit.running_runs > 0 {
        parts.push(format!("{} running run(s)", audit.running_runs));
    }
    if audit.stale_runs > 0 {
        parts.push(format!("{} stale run(s)", audit.stale_runs));
    }
    if audit.scope_violations > 0 {
        parts.push(format!("{} scope escape(s)", audit.scope_violations));
    }
    if !audit.line_violations.is_empty() {
        parts.push("line-limit violation(s)".into());
    }
    parts.join("; ")
}

fn ensure_git(project_root: &Path, app: &mut dsx_tui::App) {
    if project_root.join(".git").exists() {
        return;
    }
    let result = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(project_root)
        .output();
    match result {
        Ok(output) if output.status.success() => {
            app.add_message("system", "✓ git init (for checkpoints)")
        }
        _ => app.add_message("system", "⚠ no git repo — checkpoints disabled"),
    }
}
