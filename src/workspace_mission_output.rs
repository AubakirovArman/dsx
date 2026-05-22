//! Text and JSON rendering for workspace mission snapshots.

use crate::workspace_mission::{MissionLineLimit, MissionNote, MissionRunHealth, MissionSnapshot};

pub(crate) fn print_mission(snapshot: &MissionSnapshot, all: bool) {
    println!(
        "Workspace mission{}: {}",
        if all { " across scopes" } else { "" },
        snapshot.workspace
    );
    print_field("goal", &snapshot.goal);
    print_field("done", &snapshot.done);
    print_field("plan", &snapshot.plan);
    print_field("last", &snapshot.last_changes);
    print_field("next", &snapshot.next_step);
    print_field("active", &snapshot.active_scope);
    print_field("arch", &snapshot.architecture);
    println!("  run-health: {}", run_health_line(&snapshot.run_health));
    println!("  line-limit: {}", line_limit_line(&snapshot.line_limit));
    print_notes(&snapshot.notes);
}

pub(crate) fn mission_json(snapshot: &MissionSnapshot) -> serde_json::Value {
    serde_json::json!({
        "workspace": snapshot.workspace,
        "mission": {
            "goal": snapshot.goal,
            "done": snapshot.done,
            "plan": snapshot.plan,
            "last_changes": snapshot.last_changes,
            "next_step": snapshot.next_step,
            "active_scope": snapshot.active_scope,
            "architecture": snapshot.architecture,
        },
        "run_health": run_health_json(&snapshot.run_health),
        "line_limit": {
            "ok": snapshot.line_limit.ok,
            "violations": snapshot.line_limit.violations,
            "pressure": snapshot.line_limit.pressure,
        },
        "scopes": snapshot.notes.iter().map(note_json).collect::<Vec<_>>(),
    })
}

fn print_field(label: &str, value: &str) {
    println!("  {label}: {}", flatten(value));
}

fn print_notes(notes: &[MissionNote]) {
    if notes.is_empty() {
        println!("  scopes: none");
        return;
    }
    println!("  scopes:");
    for note in notes.iter().take(6) {
        println!(
            "    [{}] {} next: {}",
            note.scope,
            if note.saved { "saved" } else { "fallback" },
            flatten(&note.next_step)
        );
    }
}

fn note_json(note: &MissionNote) -> serde_json::Value {
    serde_json::json!({
        "scope": note.scope,
        "saved": note.saved,
        "next_step": note.next_step,
        "architecture": note.architecture,
    })
}

fn run_health_json(health: &MissionRunHealth) -> serde_json::Value {
    serde_json::json!({
        "recent_runs": health.recent_runs,
        "running_runs": health.running_runs,
        "failed_runs": health.failed_runs,
        "cancelled_runs": health.cancelled_runs,
        "total_tokens": health.total_tokens,
        "scope_violations": health.scope_violations,
    })
}

fn run_health_line(health: &MissionRunHealth) -> String {
    format!(
        "{} recent, {} running, {} failed, {} cancelled, {} tok, {} blocked escape(s)",
        health.recent_runs,
        health.running_runs,
        health.failed_runs,
        health.cancelled_runs,
        health.total_tokens,
        health.scope_violations
    )
}

fn line_limit_line(line_limit: &MissionLineLimit) -> String {
    if !line_limit.violations.is_empty() {
        return format!("fail; {}", line_limit.violations.join(", "));
    }
    if !line_limit.pressure.is_empty() {
        return format!("ok; pressure: {}", line_limit.pressure.join(", "));
    }
    format!("ok; Rust files <= {}", crate::line_limit::MAX_RS_LINES)
}

fn flatten(value: &str) -> String {
    value.replace('\n', " | ")
}
