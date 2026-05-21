//! Shared TUI state bootstrap helpers.

use std::path::Path;
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
    history_events: HistoryEvents,
) {
    let mut app = app.lock().unwrap();
    app.api_base = api_base;
    app.api_key = api_key;
    app.mode = initial_mode.as_str().to_string();
    app.add_message("system", &format!("Project: {}", project_root.display()));
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
    if let Ok(files) = dsx_index::scan_project(project_root) {
        app.file_tree = files.into_iter().take(50).collect();
    }
}

pub fn start_indexing(
    app: SharedApp,
    project_root: std::path::PathBuf,
    pool: Option<sqlx::SqlitePool>,
    rt: &Handle,
) {
    let Some(pool) = pool else {
        return;
    };
    rt.spawn(async move {
        if let Ok(count) = dsx_index::build_symbol_index(&project_root, &pool).await {
            let mut app = app.lock().unwrap();
            app.add_message(
                "system",
                &format!(
                    "✓ Semantic Indexing complete: {count} structural symbols indexed in SQLite."
                ),
            );
        }
    });
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
