//! One-screen workspace health report across scope, memory, and budget state.

use std::path::Path;

const STALE_MINUTES: i64 = 60;

pub(crate) struct WorkspaceAudit {
    pub(crate) workspace: String,
    pub(crate) budget: String,
    pub(crate) running_runs: usize,
    pub(crate) stale_runs: i64,
    pub(crate) line_violations: Vec<String>,
    pub(crate) line_pressure: Vec<String>,
    pub(crate) runs: Vec<AuditRun>,
    pub(crate) notes: Vec<AuditNote>,
    pub(crate) scope_violations: i64,
}

pub(crate) struct AuditRun {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) scope: String,
    pub(crate) contract: String,
    pub(crate) task: String,
    pub(crate) scope_violations: i64,
}

pub(crate) struct AuditNote {
    pub(crate) scope: String,
    pub(crate) saved: bool,
    pub(crate) next_step: String,
    pub(crate) scope_violations: u64,
}

pub async fn run_workspace_audit(project_root: &Path, limit: u32, all: bool, json: bool) {
    match collect_workspace_audit(project_root, limit, all).await {
        Ok(audit) if json => println!("{}", audit_json(&audit)),
        Ok(audit) => print_audit(&audit, all),
        Err(e) => println!("Workspace audit error: {e}"),
    }
}

pub(crate) async fn collect_workspace_audit(
    project_root: &Path,
    limit: u32,
    all: bool,
) -> anyhow::Result<WorkspaceAudit> {
    let runs = crate::workspace_runs::collect_agent_runs(project_root, limit, all).await?;
    let notes = crate::workspace_notes::collect_workspace_notes(project_root, limit, all).await?;
    let line_violations =
        crate::line_limit::rust_line_violations(project_root, crate::line_limit::MAX_RS_LINES)?
            .into_iter()
            .map(line_count_label)
            .collect::<Vec<_>>();
    let line_pressure = crate::line_limit::rust_line_pressure(
        project_root,
        crate::line_limit::PRESSURE_RS_LINES,
        crate::line_limit::MAX_RS_LINES,
    )?
    .into_iter()
    .map(line_count_label)
    .collect::<Vec<_>>();
    let running_runs = crate::workspace_runs::running_run_count(project_root).await?;
    let stale_runs = crate::workspace_stale_runs::stale_run_count(project_root, STALE_MINUTES)
        .await
        .unwrap_or(0);
    let audit_runs = runs
        .iter()
        .map(|run| audit_run(project_root, run))
        .collect::<Vec<_>>();
    let scope_violations = audit_runs.iter().map(|run| run.scope_violations).sum();

    Ok(WorkspaceAudit {
        workspace: project_root.display().to_string(),
        budget: dsx_agent::budget::format_limits(dsx_agent::budget::current_limits()),
        running_runs,
        stale_runs,
        line_violations,
        line_pressure,
        runs: audit_runs,
        notes: notes
            .iter()
            .map(|note| AuditNote {
                scope: note.label.clone(),
                saved: note.saved,
                next_step: note.next_step.clone(),
                scope_violations: note.scope_violations,
            })
            .collect(),
        scope_violations,
    })
}

fn audit_run(project_root: &Path, located: &crate::workspace_runs::LocatedRun) -> AuditRun {
    let run = &located.run;
    AuditRun {
        id: run.id.chars().take(8).collect(),
        status: run.status.clone(),
        scope: crate::workspace_runs::scope_label_for_db(project_root, &located.db_path),
        contract: format!(
            "{} -> {} ({})",
            scope_text(&run.launch_scope),
            scope_text(&run.active_scope),
            run.scope_status
        ),
        task: crate::handlers::task_preview(&run.task_excerpt),
        scope_violations: run.scope_violations,
    }
}

fn print_audit(audit: &WorkspaceAudit, all: bool) {
    println!(
        "Workspace audit{}: {}",
        if all { " across scopes" } else { "" },
        audit.workspace
    );
    println!("  budget: {}", audit.budget);
    println!(
        "  run-ledger: running={} stale>{}m={}",
        audit.running_runs, STALE_MINUTES, audit.stale_runs
    );
    println!("  line-limit: {}", line_status(audit));
    println!(
        "  scope-guard: {} blocked escape(s)",
        audit.scope_violations
    );
    print_runs(audit);
    print_notes(audit);
}

fn print_runs(audit: &WorkspaceAudit) {
    if audit.runs.is_empty() {
        println!("  recent-runs: none");
        return;
    }
    println!("  recent-runs:");
    for run in &audit.runs {
        println!(
            "    {} {} [{}] {}",
            run.id, run.status, run.scope, run.contract
        );
        println!("      task: {}", run.task);
    }
}

fn print_notes(audit: &WorkspaceAudit) {
    if audit.notes.is_empty() {
        println!("  notes: none");
        return;
    }
    println!("  notes:");
    for note in audit.notes.iter().take(5) {
        let saved = if note.saved { "saved" } else { "fallback" };
        println!("    [{}] {} next: {}", note.scope, saved, note.next_step);
    }
}

fn audit_json(audit: &WorkspaceAudit) -> serde_json::Value {
    serde_json::json!({
        "workspace": audit.workspace,
        "budget": audit.budget,
        "running_runs": audit.running_runs,
        "stale_runs_60m": audit.stale_runs,
        "line_limit": {
            "ok": audit.line_violations.is_empty(),
            "violations": audit.line_violations,
            "pressure": audit.line_pressure,
        },
        "scope_violations": audit.scope_violations,
        "runs": audit.runs.iter().map(run_json).collect::<Vec<_>>(),
        "notes": audit.notes.iter().map(note_json).collect::<Vec<_>>(),
    })
}

fn run_json(run: &AuditRun) -> serde_json::Value {
    serde_json::json!({
        "id": run.id,
        "status": run.status,
        "scope": run.scope,
        "contract": run.contract,
        "task": run.task,
        "scope_violations": run.scope_violations,
    })
}

fn note_json(note: &AuditNote) -> serde_json::Value {
    serde_json::json!({
        "scope": note.scope,
        "saved": note.saved,
        "next_step": note.next_step,
        "scope_violations": note.scope_violations,
    })
}

fn line_status(audit: &WorkspaceAudit) -> String {
    if audit.line_violations.is_empty() {
        let mut status = format!("ok; Rust files <= {}", crate::line_limit::MAX_RS_LINES);
        if !audit.line_pressure.is_empty() {
            status.push_str("; pressure: ");
            status.push_str(&audit.line_pressure.join(", "));
        }
        return status;
    }
    format!("fail; {}", audit.line_violations.join(", "))
}

fn line_count_label(item: crate::line_limit::FileLineCount) -> String {
    format!("{}={} lines", item.path.display(), item.lines)
}

fn scope_text(value: &str) -> &str {
    if value.trim().is_empty() { "." } else { value }
}

#[cfg(test)]
#[path = "workspace_audit_tests.rs"]
mod tests;
