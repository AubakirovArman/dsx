//! Compact context capsule preview for state-first continuation.

use std::path::Path;

pub(crate) struct ContextCapsule {
    pub(crate) task: String,
    pub(crate) clean_task: String,
    pub(crate) launch_scope: String,
    pub(crate) active_scope: String,
    pub(crate) narrowed: bool,
    pub(crate) active_exists: bool,
    pub(crate) task_state: dsx_agent::brief::TaskBriefParts,
    pub(crate) folder_notes: Vec<crate::workspace_notes::WorkspaceNote>,
    pub(crate) metrics: CapsuleMetrics,
}

pub(crate) struct CapsuleMetrics {
    pub(crate) task_state_chars: usize,
    pub(crate) folder_note_count: usize,
    pub(crate) estimated_capsule_tokens: u64,
}

pub async fn run_context_capsule(
    project_root: &Path,
    task: &str,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    let capsule = build_context_capsule(project_root, task, limit).await?;
    if json {
        println!("{}", capsule_json(&capsule));
    } else {
        print_capsule(&capsule);
    }
    Ok(())
}

pub(crate) async fn build_context_capsule(
    project_root: &Path,
    task: &str,
    limit: u32,
) -> anyhow::Result<ContextCapsule> {
    let scope = dsx_agent::scope::resolve_task_scope(project_root, task)?;
    let clean_task = dsx_agent::brief::clean_task_input(task);
    let ctx = collect_capsule_context(&scope.active_root).await?;
    let task_state = dsx_agent::brief::build_task_brief_parts(&clean_task, &scope, &ctx);
    let folder_notes = crate::workspace_notes::collect_workspace_notes(project_root, limit, true)
        .await
        .unwrap_or_default();
    let metrics = capsule_metrics(&task_state, &folder_notes);

    Ok(ContextCapsule {
        task: task.into(),
        clean_task,
        launch_scope: scope.launch_root.display().to_string(),
        active_scope: scope.active_root.display().to_string(),
        narrowed: scope.narrowed,
        active_exists: scope.active_root.exists(),
        task_state,
        folder_notes,
        metrics,
    })
}

async fn collect_capsule_context(active_root: &Path) -> anyhow::Result<dsx_context::AgentContext> {
    if active_root.exists() {
        return dsx_context::ContextManager::new()
            .collect(active_root, 250_000)
            .await;
    }

    Ok(dsx_context::AgentContext {
        project_root: active_root.display().to_string(),
        git_status: "active scope does not exist yet".into(),
        git_diff: String::new(),
        file_tree: Vec::new(),
        memories: Vec::new(),
        task_summary: None,
        max_tokens: 250_000,
    })
}

fn print_capsule(capsule: &ContextCapsule) {
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
    println!("\n{}", capsule.task_state.render());
    print_folder_notes(&capsule.folder_notes);
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

pub(crate) fn capsule_json(capsule: &ContextCapsule) -> serde_json::Value {
    serde_json::json!({
        "task": capsule.task,
        "clean_task": capsule.clean_task,
        "launch_scope": capsule.launch_scope,
        "active_scope": capsule.active_scope,
        "narrowed": capsule.narrowed,
        "active_exists": capsule.active_exists,
        "scope_contract": {
            "launch_scope": capsule.launch_scope,
            "active_scope": capsule.active_scope,
            "tool_root": capsule.active_scope,
            "status": if capsule.narrowed { "narrowed" } else { "wide" },
            "active_exists": capsule.active_exists,
            "rule": "read/write/commands are locked to active_scope",
            "warning": if capsule.narrowed { "" } else { "workspace-wide until a child folder is selected" },
        },
        "task_state": capsule.task_state,
        "folder_notes": crate::workspace_notes::notes_json_value(&capsule.folder_notes),
        "metrics": {
            "task_state_chars": capsule.metrics.task_state_chars,
            "folder_note_count": capsule.metrics.folder_note_count,
            "estimated_capsule_tokens": capsule.metrics.estimated_capsule_tokens,
        },
    })
}

fn capsule_metrics(
    task_state: &dsx_agent::brief::TaskBriefParts,
    notes: &[crate::workspace_notes::WorkspaceNote],
) -> CapsuleMetrics {
    let text = task_state.render();
    let note_chars = notes
        .iter()
        .map(|note| note.last_changes.len() + note.next_step.len() + note.architecture.len())
        .sum::<usize>();
    let chars = text.chars().count() + note_chars;
    CapsuleMetrics {
        task_state_chars: text.chars().count(),
        folder_note_count: notes.len(),
        estimated_capsule_tokens: (chars as u64 / 4).max(1),
    }
}

fn flatten(value: &str) -> String {
    value.replace('\n', " | ")
}
