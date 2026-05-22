//! Completing TUI agent runs and persisting task state.

use crate::tui_state::SharedApp;
use std::path::PathBuf;
use tokio::runtime::Handle;

pub(crate) fn finish_task(
    app: SharedApp,
    session_id: Option<String>,
    pool: Option<sqlx::SqlitePool>,
    active_root: PathBuf,
    run_id: u64,
    rt: Handle,
) {
    let (assistant, brief, tools, violations, last_violation, cost, tokens, ledger_id, snapshot) = {
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
        let assistant = if completed {
            app.messages
                .last()
                .filter(|m| m.role == "assistant")
                .cloned()
        } else {
            None
        };
        (
            assistant,
            app.task_brief.clone(),
            app.tool_timeline.clone(),
            app.scope_violations,
            app.last_scope_violation.clone(),
            app.cost,
            app.tokens,
            app.active_ledger_id.take(),
            snapshot,
        )
    };

    let summary_root = active_root.clone();
    rt.spawn(async move {
        let _ = crate::session_state::record_task_finished(
            &summary_root,
            &brief,
            &tools,
            violations,
            &last_violation,
        )
        .await;
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

pub(crate) fn persist_run_ledger(
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
