//! Compact context capsule preview for state-first continuation.

use std::path::Path;

pub(crate) use crate::context_capsule_json::capsule_json;

pub(crate) struct ContextCapsule {
    pub(crate) task: String,
    pub(crate) clean_task: String,
    pub(crate) launch_scope: String,
    pub(crate) active_scope: String,
    pub(crate) narrowed: bool,
    pub(crate) active_exists: bool,
    pub(crate) task_state: dsx_agent::brief::TaskBriefParts,
    pub(crate) folder_notes: Vec<crate::workspace_notes::WorkspaceNote>,
    pub(crate) run_health: CapsuleRunHealth,
    pub(crate) metrics: CapsuleMetrics,
}

pub(crate) struct CapsuleMetrics {
    pub(crate) task_state_chars: usize,
    pub(crate) folder_note_count: usize,
    pub(crate) estimated_capsule_tokens: u64,
}

#[derive(Default)]
pub(crate) struct CapsuleRunHealth {
    pub(crate) recent_runs: usize,
    pub(crate) running_runs: usize,
    pub(crate) failed_runs: usize,
    pub(crate) cancelled_runs: usize,
    pub(crate) total_tokens: i64,
    pub(crate) compaction_events: i64,
    pub(crate) estimated_tokens_saved: i64,
    pub(crate) scope_violations: i64,
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
        crate::context_capsule_output::print_capsule(&capsule);
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
    let run_health = capsule_run_health(project_root, limit).await;
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
        run_health,
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

async fn capsule_run_health(project_root: &Path, limit: u32) -> CapsuleRunHealth {
    let Ok(runs) =
        crate::workspace_runs::collect_agent_runs(project_root, limit.max(1), true).await
    else {
        return CapsuleRunHealth::default();
    };

    let mut health = CapsuleRunHealth {
        recent_runs: runs.len(),
        ..Default::default()
    };
    for located in runs {
        let run = located.run;
        if run.status == "running" {
            health.running_runs += 1;
        }
        if run.status == "failed" || run.error.is_some() {
            health.failed_runs += 1;
        }
        if run.cancelled {
            health.cancelled_runs += 1;
        }
        health.total_tokens += run.total_tokens;
        health.compaction_events += run.compaction_events;
        health.estimated_tokens_saved += run.estimated_tokens_saved;
        health.scope_violations += run.scope_violations;
    }
    health
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
