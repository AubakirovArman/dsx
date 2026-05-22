//! Human-readable rendering for context capsules.

use crate::context_capsule::ContextCapsule;

pub(crate) fn print_capsule(capsule: &ContextCapsule) {
    println!("Context capsule:");
    println!("  Task: {}", crate::handlers::task_preview(&capsule.task));
    println!(
        "  Clean task: {}",
        crate::handlers::task_preview(&capsule.clean_task)
    );
    println!("  Launch: {}", capsule.launch_scope);
    println!("  Active: {}", capsule.active_scope);
    println!(
        "  Status: {}",
        if capsule.narrowed { "NARROWED" } else { "WIDE" }
    );
    println!(
        "  Active exists: {}",
        if capsule.active_exists { "yes" } else { "no" }
    );
    println!("  Scope contract: tools locked to active scope");
    if !capsule.narrowed {
        println!("  Scope warning: workspace-wide until a child folder is selected");
    }
    println!(
        "  Capsule estimate: ~{} token(s), {} folder note(s)",
        capsule.metrics.estimated_capsule_tokens, capsule.metrics.folder_note_count
    );
    print_handoff(capsule);
    println!("\n{}", capsule.task_state.render());
    print_folder_notes(&capsule.folder_notes);
}

fn print_handoff(capsule: &ContextCapsule) {
    println!("\nSession handoff:");
    println!(
        "  goal: {}",
        crate::handlers::task_preview(&capsule.task_state.goal)
    );
    println!("  done: {}", flatten(&capsule.task_state.done));
    println!("  next: {}", flatten(&capsule.task_state.next_step));
    println!("  tool root: {}", capsule.active_scope);
    println!(
        "  health: {} recent, {} running, {} failed, {} cancelled, {} blocked escape(s)",
        capsule.run_health.recent_runs,
        capsule.run_health.running_runs,
        capsule.run_health.failed_runs,
        capsule.run_health.cancelled_runs,
        capsule.run_health.scope_violations
    );
}

fn print_folder_notes(notes: &[crate::workspace_notes::WorkspaceNote]) {
    if notes.is_empty() {
        println!("\nFolder notes:\n  - (none)");
        return;
    }

    println!("\nFolder notes:");
    for note in notes {
        println!(
            "  - {} [{}]",
            note.label,
            if note.saved { "saved" } else { "fallback" }
        );
        println!("    last: {}", flatten(&note.last_changes));
        println!("    next: {}", flatten(&note.next_step));
        println!("    arch: {}", flatten(&note.architecture));
    }
}

fn flatten(value: &str) -> String {
    value.replace('\n', " | ")
}
