//! TUI context-capsule budget preflight before model calls.

use crate::context_preview::ContextPreview;
use crate::tui_state::SharedApp;

pub(crate) async fn preflight_context_budget(app: &SharedApp, task: &str) -> bool {
    let project_root = { app.lock().unwrap().scope_lock.launch_scope.clone() };
    let Ok(project_root) = std::path::PathBuf::from(project_root).canonicalize() else {
        block_with_error(
            app,
            task,
            "Launch workspace is not readable for context preflight.",
        );
        return false;
    };
    let preview = match crate::context_preview::build_context_preview(&project_root, task).await {
        Ok(preview) => preview,
        Err(e) => {
            block_with_error(app, task, &format!("Context preflight failed: {e}"));
            return false;
        }
    };
    if let Err(e) = crate::context_preview::enforce_request_budget(&preview) {
        block_with_error(app, task, &e.to_string());
        return false;
    }
    mark_ready(app, &preview);
    true
}

pub(crate) fn mark_ready(app: &SharedApp, preview: &ContextPreview) {
    let line = crate::context_preview::budget_line(preview);
    let mut app = app.lock().unwrap();
    app.budget_status = line.clone();
    app.task_brief = dsx_tui::TaskBriefPanel {
        goal: preview.task_parts.goal.clone(),
        done: format!("Context capsule prepared. {}", preview.task_parts.done),
        plan: preview.task_parts.plan.clone(),
        last_changes: format!("{}\n{}", line, preview.task_parts.last_changes),
        next_step: preview.task_parts.next_step.clone(),
        active_scope: preview.task_parts.active_scope.clone(),
    };
    app.add_message("system", &format!("Context budget preflight: {line}"));
}

pub(crate) fn block_with_error(app: &SharedApp, task: &str, error: &str) {
    let mut app = app.lock().unwrap();
    app.active_run_id = None;
    app.active_ledger_id = None;
    app.agent_abort = None;
    app.agent_task = dsx_tui::AgentTask::Error("context budget blocked".into());
    app.input = task.into();
    app.cursor_pos = app.input.chars().count();
    app.task_brief.done = "Task blocked before model call.".into();
    app.task_brief.last_changes = error.chars().take(220).collect();
    app.task_brief.next_step = "Narrow the task scope or reduce context before retrying.".into();
    app.scope_lock.warning = "No model call started: context budget preflight blocked.".into();
    app.add_message(
        "system",
        &format!("Context budget preflight blocked: {error}"),
    );
}
