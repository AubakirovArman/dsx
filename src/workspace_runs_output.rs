//! Text and JSON rendering for workspace run ledgers.

use std::path::Path;

use crate::workspace_runs::LocatedRun;

struct RunsSummary {
    total: usize,
    running: usize,
    failed: usize,
    cancelled: usize,
    total_tokens: i64,
    estimated_cost_usd: f64,
    compaction_events: i64,
    estimated_tokens_saved: i64,
    scope_violations: i64,
}

pub(crate) fn print_runs(project_root: &Path, runs: &[LocatedRun], all: bool) {
    println!(
        "Recent agent runs{}:",
        if all { " across scopes" } else { "" }
    );
    for located in runs {
        print_run(project_root, located, all);
    }
}

pub(crate) fn runs_json(
    project_root: &Path,
    limit: u32,
    all: bool,
    runs: &[LocatedRun],
) -> serde_json::Value {
    serde_json::json!({
        "workspace": project_root.display().to_string(),
        "limit": limit,
        "all": all,
        "summary": summary_json(&runs_summary(runs)),
        "runs": runs.iter().map(|run| run_json(project_root, run)).collect::<Vec<_>>(),
    })
}

fn print_run(project_root: &Path, located: &LocatedRun, all: bool) {
    let run = &located.run;
    println!(
        "  {}  {}  {} tok  ${:.4}  compact:{}/~{}tok  scope:{}  {}",
        &run.id[..8.min(run.id.len())],
        run.status,
        run.total_tokens,
        run.estimated_cost_usd,
        run.compaction_events,
        run.estimated_tokens_saved,
        run.scope_violations,
        run.started_at.chars().take(19).collect::<String>(),
    );
    if all {
        println!("      scope: {}", scope_label(project_root, located));
    }
    if !run.active_scope.trim().is_empty() {
        println!(
            "      contract: {} -> {} ({})",
            scope_text(&run.launch_scope),
            scope_text(&run.active_scope),
            run.scope_status
        );
        println!(
            "      reason: {}",
            crate::handlers::task_preview(&run.scope_reason)
        );
    }
    println!("      {}", crate::handlers::task_preview(&run.task_excerpt));
    if let Some(error) = &run.error {
        println!("      error: {}", crate::handlers::task_preview(error));
    }
    if run.scope_violations > 0 {
        println!(
            "      scope_guard: {}",
            crate::handlers::task_preview(&run.last_scope_violation)
        );
    }
}

fn runs_summary(runs: &[LocatedRun]) -> RunsSummary {
    let mut summary = RunsSummary {
        total: runs.len(),
        running: 0,
        failed: 0,
        cancelled: 0,
        total_tokens: 0,
        estimated_cost_usd: 0.0,
        compaction_events: 0,
        estimated_tokens_saved: 0,
        scope_violations: 0,
    };
    for located in runs {
        add_run_to_summary(&mut summary, &located.run);
    }
    summary
}

fn add_run_to_summary(summary: &mut RunsSummary, run: &dsx_memory::AgentRunRecord) {
    summary.total_tokens += run.total_tokens;
    summary.estimated_cost_usd += run.estimated_cost_usd;
    summary.compaction_events += run.compaction_events;
    summary.estimated_tokens_saved += run.estimated_tokens_saved;
    summary.scope_violations += run.scope_violations;
    if run.status == "running" {
        summary.running += 1;
    }
    if run.status == "failed" {
        summary.failed += 1;
    }
    if run.cancelled {
        summary.cancelled += 1;
    }
}

fn summary_json(summary: &RunsSummary) -> serde_json::Value {
    serde_json::json!({
        "total": summary.total,
        "running": summary.running,
        "failed": summary.failed,
        "cancelled": summary.cancelled,
        "total_tokens": summary.total_tokens,
        "estimated_cost_usd": summary.estimated_cost_usd,
        "compaction_events": summary.compaction_events,
        "estimated_tokens_saved": summary.estimated_tokens_saved,
        "scope_violations": summary.scope_violations,
    })
}

fn run_json(project_root: &Path, located: &LocatedRun) -> serde_json::Value {
    let run = &located.run;
    serde_json::json!({
        "id": run.id,
        "session_id": run.session_id,
        "status": run.status,
        "scope": scope_label(project_root, located),
        "project_root": run.project_root,
        "task": run.task_excerpt,
        "started_at": run.started_at,
        "finished_at": run.finished_at,
        "tokens": {
            "prompt": run.prompt_tokens,
            "completion": run.completion_tokens,
            "reasoning": run.reasoning_tokens,
            "total": run.total_tokens,
        },
        "estimated_cost_usd": run.estimated_cost_usd,
        "compaction": {
            "events": run.compaction_events,
            "messages": run.compacted_messages,
            "estimated_tokens_saved": run.estimated_tokens_saved,
        },
        "scope_contract": {
            "launch_scope": run.launch_scope,
            "active_scope": run.active_scope,
            "status": run.scope_status,
            "reason": run.scope_reason,
            "violations": run.scope_violations,
            "last_violation": run.last_scope_violation,
        },
        "error": run.error,
        "cancelled": run.cancelled,
    })
}

fn scope_text(value: &str) -> &str {
    if value.trim().is_empty() { "." } else { value }
}

fn scope_label(project_root: &Path, located: &LocatedRun) -> String {
    crate::workspace_runs::scope_label_for_db(project_root, &located.db_path)
}
