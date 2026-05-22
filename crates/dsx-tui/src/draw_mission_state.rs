//! Mission Control derived state helpers.

use crate::App;
use ratatui::style::Color;

pub(crate) fn mission_tool_counts(app: &App) -> (usize, usize, usize) {
    let ok = app
        .tool_timeline
        .iter()
        .filter(|entry| entry.status == "ok")
        .count();
    let failed = app
        .tool_timeline
        .iter()
        .filter(|entry| entry.status == "failed")
        .count();
    let blocked = app
        .tool_timeline
        .iter()
        .filter(|entry| entry.status == "blocked")
        .count();
    (ok, failed, blocked)
}

pub(crate) fn task_state(app: &App) -> String {
    match &app.agent_task {
        crate::AgentTask::Idle => "idle".into(),
        crate::AgentTask::Running(task) => format!("running: {}", truncate(task, 48)),
        crate::AgentTask::Done(_) => "done".into(),
        crate::AgentTask::Error(err) => format!("error: {}", truncate(err, 48)),
    }
}

pub(crate) fn scope_status(app: &App) -> String {
    let active = empty_as(&app.scope_lock.active_scope, "none");
    format!("{} -> {}", app.scope_lock.status, truncate(&active, 54))
}

pub(crate) fn scope_guard_text(app: &App) -> String {
    if app.scope_violations == 0 {
        return "0 blocked escape(s)".into();
    }
    format!(
        "{} blocked escape(s); last: {}",
        app.scope_violations,
        truncate(&app.last_scope_violation, 84)
    )
}

pub(crate) fn scope_color(app: &App) -> Color {
    if app.scope_violations > 0 {
        Color::LightRed
    } else if app.scope_lock.status == "Narrowed" {
        Color::LightGreen
    } else {
        Color::LightYellow
    }
}

pub(crate) fn tool_status_color(status: &str) -> Color {
    match status {
        "ok" => Color::LightGreen,
        "failed" | "blocked" => Color::LightRed,
        _ => Color::White,
    }
}

pub(crate) fn empty_as(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.into()
    } else {
        value.into()
    }
}

fn truncate(value: &str, limit: usize) -> String {
    let mut text: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        text.push_str("...");
    }
    text
}

#[cfg(test)]
#[path = "draw_mission_tests.rs"]
mod tests;
