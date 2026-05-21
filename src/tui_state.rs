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
    let budget_status = dsx_agent::budget::format_limits(dsx_agent::budget::current_limits());
    app.budget_status = budget_status.clone();
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
