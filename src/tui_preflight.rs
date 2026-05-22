//! TUI updates for blocked agent-start preflight decisions.

use std::path::Path;

pub(crate) fn block_if_needed(app: &mut dsx_tui::App, project_root: &Path, task: &str) -> bool {
    let Some(preflight) =
        crate::agent_preflight::blocked_agent_start(project_root, task, app.allow_wide_scope)
    else {
        return false;
    };
    let message = crate::agent_preflight::render_text(&preflight);
    app.task_brief = dsx_tui::TaskBriefPanel {
        goal: preflight.task.clone(),
        done: "Task blocked before model call.".into(),
        plan: "1. Pick an explicit child folder\n2. Retry the task or enable wide scope policy"
            .into(),
        last_changes: preflight.reason.clone(),
        next_step: blocked_next_step(&preflight),
        active_scope: preflight.active.clone(),
        constraints: "No model call or tool execution before a safe active scope is chosen.".into(),
        architecture: "Blocked before active-scope architecture collection.".into(),
    };
    app.scope_lock = dsx_tui::ScopeLockPanel {
        launch_scope: preflight.launch,
        active_scope: preflight.active,
        status: "Blocked".into(),
        reason: preflight.reason,
        warning: "No model call or tool execution started.".into(),
    };
    app.add_message("system", &message);
    true
}

fn blocked_next_step(preflight: &crate::agent_preflight::AgentPreflight) -> String {
    preflight
        .suggested_scopes
        .first()
        .map(|scope| format!("Retry with explicit scope {scope}, run preflight, or allow wide."))
        .unwrap_or_else(|| "Add an explicit child folder, run preflight, or allow wide.".into())
}
