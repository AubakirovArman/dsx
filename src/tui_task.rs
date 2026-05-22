//! Spawning agent tasks from the TUI.

use crate::event_convert::convert_event;
use crate::tui_state::SharedApp;
use crate::tui_task_finish::persist_run_ledger_and_refresh;
use std::path::{Path, PathBuf};
use tokio::{runtime::Handle, sync::mpsc, task::AbortHandle};

pub(crate) use crate::tui_task_finish::finish_task;

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
    if !crate::tui_context_budget::preflight_context_budget(app, &prepared.task).await {
        return Ok(());
    }
    crate::tui_state::start_active_scope_indexing(app.clone(), prepared.active_root.clone(), rt);
    prepared.ledger_id = crate::tui_run_ledger::start_run_ledger(app, session_id, &prepared).await;
    persist_user_message(session_id, pool, rt, &prepared.task);

    if let (Some(ref sid), Some(ref p)) = (session_id.clone(), pool.clone()) {
        let sm = dsx_session::SessionManager::new(p.clone());
        let sid_copy = sid.clone();
        let path_str = prepared.active_root.display().to_string();
        rt.spawn(async move {
            let _ = sm.update_project_root(&sid_copy, &path_str).await;
        });
    }

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
    let (ledger_id, active_scope, refresh_root, snapshot) = {
        let mut state = app.lock().unwrap();
        let Some(abort) = state.agent_abort.take() else {
            state.add_message("system", "No active agent run to stop.");
            return false;
        };
        abort.abort();
        let ledger_id = state.active_ledger_id.take();
        let active_scope = state.task_brief.active_scope.clone();
        let active_scope_path = PathBuf::from(scope_text(&active_scope));
        let snapshot = crate::run_ledger::RunLedgerSnapshot::from_app(
            &state,
            "cancelled",
            Some("cancelled by user".into()),
        );
        let refresh_root = crate::tui_run_health::refresh_root(&state, &active_scope_path);
        if let Some(approval) = state.pending_approval.take() {
            let _ = approval.tx.send(false);
        }
        state.active_run_id = None;
        state.current_reasoning.clear();
        state.agent_task = dsx_tui::AgentTask::Error("cancelled by user".into());
        state.task_brief.done = "Run cancelled by user.".into();
        state.task_brief.last_changes = "Abort requested from TUI.".into();
        state.task_brief.next_step = "Review partial output or enter a narrower task.".into();
        state.add_message("system", "⏹ Agent run cancelled by user.");
        (ledger_id, active_scope, refresh_root, snapshot)
    };
    persist_run_ledger_and_refresh(
        rt,
        app.clone(),
        ledger_id,
        active_scope,
        refresh_root,
        snapshot,
    );
    true
}

#[derive(Clone)]
pub(crate) struct PreparedTask {
    pub(crate) run_id: u64,
    pub(crate) ledger_id: Option<String>,
    pub(crate) task: String,
    pub(crate) api_key: String,
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
    if crate::tui_preflight::block_if_needed(&mut app, project_root, &task) {
        return None;
    }
    let scope = crate::task_scope::resolve_task_scope(project_root, &task);
    app.project_root = scope.active_root.clone();
    if let Ok(files) = dsx_fs::list_files(&app.project_root) {
        app.file_tree = files.into_iter().take(50).collect();
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
        active_root: scope.active_root,
        mode,
    })
}

pub(crate) fn agent_workspace(task: &PreparedTask) -> PathBuf {
    task.active_root.clone()
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
            project_root: agent_workspace(&task),
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

fn scope_text(scope: &str) -> &str {
    if scope.trim().is_empty() { "." } else { scope }
}
