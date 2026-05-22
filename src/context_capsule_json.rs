//! JSON rendering for context capsules.

use crate::context_capsule::{CapsuleRunHealth, ContextCapsule};

pub(crate) fn capsule_json(capsule: &ContextCapsule) -> serde_json::Value {
    serde_json::json!({
        "task": capsule.task,
        "clean_task": capsule.clean_task,
        "launch_scope": capsule.launch_scope,
        "active_scope": capsule.active_scope,
        "narrowed": capsule.narrowed,
        "active_exists": capsule.active_exists,
        "scope_contract": scope_contract_json(capsule),
        "handoff": handoff_json(capsule),
        "task_state": capsule.task_state,
        "folder_notes": crate::workspace_notes::notes_json_value(&capsule.folder_notes),
        "run_health": run_health_json(&capsule.run_health),
        "metrics": {
            "task_state_chars": capsule.metrics.task_state_chars,
            "folder_note_count": capsule.metrics.folder_note_count,
            "estimated_capsule_tokens": capsule.metrics.estimated_capsule_tokens,
        },
    })
}

fn handoff_json(capsule: &ContextCapsule) -> serde_json::Value {
    serde_json::json!({
        "goal": capsule.task_state.goal,
        "done": capsule.task_state.done,
        "plan": capsule.task_state.plan,
        "last_changes": capsule.task_state.last_changes,
        "next_step": capsule.task_state.next_step,
        "constraints": capsule.task_state.constraints,
        "surface_architecture": capsule.task_state.surface_architecture,
        "scope_contract": scope_contract_json(capsule),
        "folder_notes": crate::workspace_notes::notes_json_value(&capsule.folder_notes),
        "run_health": run_health_json(&capsule.run_health),
    })
}

fn scope_contract_json(capsule: &ContextCapsule) -> serde_json::Value {
    serde_json::json!({
        "launch_scope": capsule.launch_scope,
        "active_scope": capsule.active_scope,
        "tool_root": capsule.active_scope,
        "status": if capsule.narrowed { "narrowed" } else { "wide" },
        "active_exists": capsule.active_exists,
        "rule": "read/write/commands are locked to active_scope",
        "warning": if capsule.narrowed { "" } else { "workspace-wide until a child folder is selected" },
    })
}

fn run_health_json(health: &CapsuleRunHealth) -> serde_json::Value {
    serde_json::json!({
        "recent_runs": health.recent_runs,
        "running_runs": health.running_runs,
        "failed_runs": health.failed_runs,
        "cancelled_runs": health.cancelled_runs,
        "total_tokens": health.total_tokens,
        "compaction_events": health.compaction_events,
        "estimated_tokens_saved": health.estimated_tokens_saved,
        "scope_violations": health.scope_violations,
    })
}
