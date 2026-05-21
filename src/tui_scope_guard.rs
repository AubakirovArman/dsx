//! TUI task-start guardrails for concurrent runs.

pub(crate) fn task_start_blocker(app: &dsx_tui::App) -> Option<&'static str> {
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
