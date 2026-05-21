//! Spawning agent tasks from the TUI.

use crate::event_convert::convert_event;
use crate::tui_state::SharedApp;
use std::path::{Path, PathBuf};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

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
    spawn_agent(rt, prepared.clone(), api_base, Some(approval_tx), tx);

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
        finish_task(stream_app, session_id, pool, prepared.active_root, rt_copy);
    });

    Ok(())
}

#[derive(Clone)]
struct PreparedTask {
    task: String,
    api_key: String,
    project_root: PathBuf,
    active_root: PathBuf,
    mode: dsx_core::types::PermissionMode,
}

fn prepare_task(app: &SharedApp, project_root: &Path, api_key: &str) -> Option<PreparedTask> {
    let mut app = app.lock().unwrap();
    let task = app.input.clone();
    app.input.clear();
    app.scroll_offset = 0;
    if task.trim().is_empty() {
        return None;
    }
    let mode = dsx_core::types::PermissionMode::parse(&app.mode)
        .unwrap_or(dsx_core::types::PermissionMode::Ask);
    let active_root = dsx_agent::scope::resolve_task_scope(project_root, &task)
        .map(|scope| scope.active_root)
        .unwrap_or_else(|_| project_root.to_path_buf());
    app.begin_task(&task, &active_root.display().to_string());
    app.add_message("user", &task);
    app.agent_task = dsx_tui::AgentTask::Running(task.clone());

    Some(PreparedTask {
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
) {
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
    });
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

fn finish_task(
    app: SharedApp,
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
    active_root: PathBuf,
    rt: Handle,
) {
    let (assistant, brief, tools, cost, tokens) = {
        let mut app = app.lock().unwrap();
        app.agent_task = dsx_tui::AgentTask::Done("ready".into());
        let assistant = app
            .messages
            .last()
            .filter(|m| m.role == "assistant")
            .cloned();
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
