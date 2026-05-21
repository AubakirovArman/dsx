//! Spawning agent tasks from the TUI.

use crate::event_convert::convert_event;
use crate::tui_state::SharedApp;
use std::path::{Path, PathBuf};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::AbortHandle;

pub async fn start_agent_task(
    app: &SharedApp,
    project_root: &Path,
    api_key: &str,
    session_id: &Option<String>,
    pool: &Option<sqlx::SqlitePool>,
    rt: &Handle,
) -> anyhow::Result<()> {
    let Some(prepared) = prepare_task(app, project_root, api_key) else {
        return Ok(());
    };
    crate::session_state::record_task_started(&prepared.active_root, &prepared.task).await?;
    persist_user_message(session_id, pool, rt, &prepared.task);

    let api_base = app.lock().unwrap().api_base.clone();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (approval_tx, mut approval_rx) = mpsc::unbounded_channel();
    let abort = spawn_agent(rt, prepared.clone(), api_base, Some(approval_tx), tx);
    app.lock().unwrap().agent_abort = Some(abort);

    let approval_app = app.clone();
    rt.spawn(async move {
        while let Some(req) = approval_rx.recv().await {
            approval_app.lock().unwrap().pending_approval = Some(dsx_tui::PendingApproval {
                tool_name: req.tool_name,
                arguments: req.arguments,
                tx: req.tx,
            });
        }
    });

    let stream_app = app.clone();
    let session_id = session_id.clone();
    let pool = pool.clone();
    let rt_copy = rt.clone();
    rt.spawn(async move {
        while let Some(event) = rx.recv().await {
            stream_app
                .lock()
                .unwrap()
                .handle_stream_event(&convert_event(&event));
        }
        finish_task(
            stream_app,
            session_id,
            pool,
            prepared.active_root,
            prepared.run_id,
            rt_copy,
        );
    });

    Ok(())
}

pub fn stop_agent_task(app: &SharedApp) -> bool {
    let mut app = app.lock().unwrap();
    let Some(abort) = app.agent_abort.take() else {
        app.add_message("system", "No active agent run to stop.");
        return false;
    };

    abort.abort();
    if let Some(approval) = app.pending_approval.take() {
        let _ = approval.tx.send(false);
    }
    app.active_run_id = None;
    app.current_reasoning.clear();
    app.agent_task = dsx_tui::AgentTask::Error("cancelled by user".into());
    app.task_brief.done = "Run cancelled by user.".into();
    app.task_brief.last_changes = "Abort requested from TUI.".into();
    app.task_brief.next_step = "Review partial output or enter a narrower task.".into();
    app.add_message("system", "⏹ Agent run cancelled by user.");
    true
}

#[derive(Clone)]
struct PreparedTask {
    run_id: u64,
    task: String,
    api_key: String,
    project_root: PathBuf,
    active_root: PathBuf,
    mode: dsx_core::types::PermissionMode,
}

fn prepare_task(app: &SharedApp, project_root: &Path, api_key: &str) -> Option<PreparedTask> {
    let mut app = app.lock().unwrap();
    let task = app.input.clone();
    if task.trim().is_empty() {
        app.scroll_offset = 0;
        return None;
    }
    if let Some(message) = task_start_blocker(&app) {
        app.add_message("system", message);
        return None;
    }
    app.input.clear();
    app.scroll_offset = 0;
    let mode = dsx_core::types::PermissionMode::parse(&app.mode)
        .unwrap_or(dsx_core::types::PermissionMode::Ask);
    let run_id = app.next_run_id.saturating_add(1);
    app.next_run_id = run_id;
    app.active_run_id = Some(run_id);
    let active_root = dsx_agent::scope::resolve_task_scope(project_root, &task)
        .map(|scope| scope.active_root)
        .unwrap_or_else(|_| project_root.to_path_buf());
    app.begin_task(&task, &active_root.display().to_string());
    app.add_message("user", &task);
    app.agent_task = dsx_tui::AgentTask::Running(task.clone());

    Some(PreparedTask {
        run_id,
        task,
        api_key: api_key.to_string(),
        project_root: project_root.to_path_buf(),
        active_root,
        mode,
    })
}

fn spawn_agent(
    rt: &Handle,
    task: PreparedTask,
    api_base: String,
    approval_tx: Option<mpsc::UnboundedSender<dsx_agent::ApprovalRequest>>,
    tx: mpsc::UnboundedSender<dsx_provider::streaming::StreamEvent>,
) -> AbortHandle {
    rt.spawn(async move {
        let config = dsx_agent::AgentConfig {
            project_root: task.project_root,
            api_key: task.api_key,
            api_base,
            max_iterations: 15,
            mode: task.mode,
            approval_tx,
        };
        let _ = dsx_agent::run_streaming(&task.task, &config, tx).await;
    })
    .abort_handle()
}

fn persist_user_message(
    session_id: &Option<String>,
    pool: &Option<sqlx::SqlitePool>,
    rt: &Handle,
    task: &str,
) {
    if let (Some(sid), Some(pool)) = (session_id.clone(), pool.clone()) {
        let sm = dsx_session::SessionManager::new(pool);
        let task = task.to_string();
        rt.spawn(async move {
            let _ = sm
                .record_event(&sid, "user_msg", &serde_json::json!({ "content": task }))
                .await;
        });
    }
}

fn task_start_blocker(app: &dsx_tui::App) -> Option<&'static str> {
    if app.pending_approval.is_some() {
        return Some("Agent is waiting for tool approval; answer it before starting a new task.");
    }
    if matches!(app.agent_task, dsx_tui::AgentTask::Running(_)) {
        return Some("Agent is already running; wait for the current task to finish.");
    }
    if app.agent_abort.is_some() {
        return Some("Agent is still stopping; wait for cleanup to finish.");
    }
    None
}

pub(crate) fn finish_task(
    app: SharedApp,
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
    active_root: PathBuf,
    run_id: u64,
    rt: Handle,
) {
    let (assistant, brief, tools, cost, tokens) = {
        let mut app = app.lock().unwrap();
        if app.active_run_id != Some(run_id) {
            return;
        }
        app.active_run_id = None;
        app.agent_abort = None;
        let completed = matches!(app.agent_task, dsx_tui::AgentTask::Running(_));
        if completed {
            app.agent_task = dsx_tui::AgentTask::Done("ready".into());
        }
        let assistant = completed
            .then(|| {
                app.messages
                    .last()
                    .filter(|m| m.role == "assistant")
                    .cloned()
            })
            .flatten();
        (
            assistant,
            app.task_brief.clone(),
            app.tool_timeline.clone(),
            app.cost,
            app.tokens,
        )
    };

    rt.spawn(async move {
        let _ = crate::session_state::record_task_finished(&active_root, &brief, &tools).await;
    });
    if let (Some(msg), Some(sid), Some(pool)) = (assistant, session_id, pool) {
        let sm = dsx_session::SessionManager::new(pool);
        rt.spawn(async move {
            let _ = sm
                .record_event(
                    &sid,
                    "assistant_msg",
                    &serde_json::json!({ "content": msg.content, "cost": cost, "tokens": tokens }),
                )
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn prepare_task_blocks_parallel_run_and_keeps_input() {
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        {
            let mut app = app.lock().unwrap();
            app.input = "second task".into();
            app.agent_task = dsx_tui::AgentTask::Running("first task".into());
        }

        let prepared = prepare_task(&app, std::path::Path::new("/tmp"), "key");

        assert!(prepared.is_none());
        let app = app.lock().unwrap();
        assert_eq!(app.input, "second task");
        assert!(
            app.messages
                .iter()
                .any(|msg| msg.content.contains("already running"))
        );
    }

    #[test]
    fn prepare_task_allows_idle_task() {
        let root = std::env::temp_dir().join("dsx_prepare_task_idle");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let app = Arc::new(Mutex::new(dsx_tui::App::new()));
        app.lock().unwrap().input = "do work".into();

        let prepared = prepare_task(&app, &root, "key").unwrap();

        assert_eq!(prepared.task, "do work");
        assert_eq!(prepared.api_key, "key");
        assert_eq!(prepared.run_id, 1);
        assert_eq!(app.lock().unwrap().active_run_id, Some(1));
        assert!(matches!(
            app.lock().unwrap().agent_task,
            dsx_tui::AgentTask::Running(_)
        ));

        let _ = std::fs::remove_dir_all(&root);
    }
}
