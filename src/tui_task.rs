//! Spawning agent tasks from the TUI.

use crate::event_convert::convert_event;
use crate::tui_state::SharedApp;
use std::path::{Path, PathBuf};
use tokio::{runtime::Handle, sync::mpsc, task::AbortHandle};

pub async fn start_agent_task(
    app: &SharedApp,
    project_root: &Path,
    api_key: &str,
    session_id: &Option<String>,
    pool: &Option<sqlx::SqlitePool>,
    rt: &Handle,
) -> anyhow::Result<()> {
    let Some(mut prepared) = prepare_task(app, project_root, api_key) else {
        return Ok(());
    };
    crate::session_state::record_task_started(&prepared.active_root, &prepared.task).await?;
    crate::tui_state::start_active_scope_indexing(app.clone(), prepared.active_root.clone(), rt);
    prepared.ledger_id = start_run_ledger(app, session_id, &prepared).await;
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

pub fn stop_agent_task(app: &SharedApp, rt: &Handle) -> bool {
    let mut app = app.lock().unwrap();
    let Some(abort) = app.agent_abort.take() else {
        app.add_message("system", "No active agent run to stop.");
        return false;
    };

    abort.abort();
    let ledger_id = app.active_ledger_id.take();
    let active_scope = app.task_brief.active_scope.clone();
    let snapshot = crate::run_ledger::RunLedgerSnapshot::from_app(
        &app,
        "cancelled",
        Some("cancelled by user".into()),
    );
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
    persist_run_ledger(rt, ledger_id, active_scope, snapshot);
    true
}

#[derive(Clone)]
pub(crate) struct PreparedTask {
    pub(crate) run_id: u64,
    pub(crate) ledger_id: Option<String>,
    pub(crate) task: String,
    pub(crate) api_key: String,
    pub(crate) project_root: PathBuf,
    pub(crate) active_root: PathBuf,
    pub(crate) mode: dsx_core::types::PermissionMode,
}

pub(crate) fn prepare_task(
    app: &SharedApp,
    project_root: &Path,
    api_key: &str,
) -> Option<PreparedTask> {
    let mut app = app.lock().unwrap();
    let task = app.input.clone();
    if task.trim().is_empty() {
        app.scroll_offset = 0;
        return None;
    }
    if let Some(message) = crate::tui_scope_guard::task_start_blocker(&app) {
        app.add_message("system", message);
        return None;
    }
    let scope = crate::task_scope::resolve_task_scope(project_root, &task);
    if let Some(message) = crate::scope_guard::wide_scope_blocker(
        project_root,
        &task,
        scope.narrowed,
        app.allow_wide_scope,
    ) {
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
    app.begin_task_scoped(
        &task,
        &scope.launch_label,
        &scope.active_label,
        scope.narrowed,
    );
    app.add_message("user", &task);
    app.agent_task = dsx_tui::AgentTask::Running(task.clone());

    Some(PreparedTask {
        run_id,
        ledger_id: None,
        task,
        api_key: api_key.to_string(),
        project_root: project_root.to_path_buf(),
        active_root: scope.active_root,
        mode,
    })
}

async fn start_run_ledger(
    app: &SharedApp,
    session_id: &Option<String>,
    task: &PreparedTask,
) -> Option<String> {
    match crate::run_ledger::record_started(&task.active_root, session_id.as_deref(), &task.task)
        .await
    {
        Ok(id) => {
            app.lock().unwrap().active_ledger_id = Some(id.clone());
            Some(id)
        }
        Err(e) => {
            app.lock()
                .unwrap()
                .add_message("error", &format!("Run ledger start failed: {e}"));
            None
        }
    }
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

pub(crate) fn finish_task(
    app: SharedApp,
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
    active_root: PathBuf,
    run_id: u64,
    rt: Handle,
) {
    let (assistant, brief, tools, cost, tokens, ledger_id, snapshot) = {
        let mut app = app.lock().unwrap();
        if app.active_run_id != Some(run_id) {
            return;
        }
        app.active_run_id = None;
        app.agent_abort = None;
        let (status, error) = run_status(&app.agent_task);
        let completed = status == "completed";
        if completed {
            app.agent_task = dsx_tui::AgentTask::Done("ready".into());
        }
        let snapshot = crate::run_ledger::RunLedgerSnapshot::from_app(&app, status, error);
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
            app.active_ledger_id.take(),
            snapshot,
        )
    };

    let summary_root = active_root.clone();
    rt.spawn(async move {
        let _ = crate::session_state::record_task_finished(&summary_root, &brief, &tools).await;
    });
    persist_run_ledger(&rt, ledger_id, active_root.clone(), snapshot);
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

fn run_status(task: &dsx_tui::AgentTask) -> (&'static str, Option<String>) {
    match task {
        dsx_tui::AgentTask::Error(err) => ("error", Some(err.clone())),
        _ => ("completed", None),
    }
}

fn persist_run_ledger(
    rt: &Handle,
    ledger_id: Option<String>,
    active_scope: impl Into<PathBuf>,
    snapshot: crate::run_ledger::RunLedgerSnapshot,
) {
    let Some(ledger_id) = ledger_id else {
        return;
    };
    let active_scope = active_scope.into();
    rt.spawn(async move {
        let _ = crate::run_ledger::record_finished(&active_scope, &ledger_id, snapshot).await;
    });
}
