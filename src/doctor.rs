//! Workspace readiness diagnostics.

use crate::line_limit::{
    MAX_RS_LINES, PRESSURE_RS_LINES, rust_line_pressure, rust_line_violations,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

struct Check {
    status: CheckStatus,
    name: &'static str,
    detail: String,
}

pub async fn run_doctor(
    project_root: &Path,
    api_base: &str,
    api_key: Option<&str>,
) -> anyhow::Result<()> {
    let checks = collect_checks(project_root, api_base, api_key).await;
    println!("DSX doctor: {}", project_root.display());
    for check in &checks {
        println!("{} {} - {}", marker(check.status), check.name, check.detail);
    }

    if checks.iter().any(|check| check.status == CheckStatus::Fail) {
        anyhow::bail!("doctor found failing checks");
    }
    Ok(())
}

async fn collect_checks(project_root: &Path, api_base: &str, api_key: Option<&str>) -> Vec<Check> {
    let mut checks = Vec::new();
    checks.push(workspace_check(project_root));
    checks.push(api_check(api_base, api_key));
    checks.push(budget_check());
    checks.push(git_check(project_root));
    checks.push(memory_check(project_root).await);
    checks.push(run_ledger_check(project_root).await);
    checks.push(capsule_check(project_root).await);
    checks.push(line_limit_check(project_root));
    checks
}

fn workspace_check(project_root: &Path) -> Check {
    if project_root.is_dir() {
        ok("workspace", "workspace directory exists")
    } else {
        fail(
            "workspace",
            "workspace directory is missing or not a directory",
        )
    }
}

fn api_check(api_base: &str, api_key: Option<&str>) -> Check {
    if api_key.is_some_and(|key| !key.trim().is_empty()) {
        return ok("api", format!("key detected; base={api_base}"));
    }
    if api_base.contains("localhost") || api_base.contains("127.0.0.1") {
        return warn("api", format!("no key detected; local base={api_base}"));
    }
    warn("api", format!("no key detected for base={api_base}"))
}

fn budget_check() -> Check {
    let limits = dsx_agent::budget::current_limits();
    ok(
        "budget",
        format!(
            "token/cost fuse: {}",
            dsx_agent::budget::format_limits(limits)
        ),
    )
}

fn git_check(project_root: &Path) -> Check {
    if project_root.join(".git").is_dir() {
        ok("git", "git repository present")
    } else {
        warn("git", "no .git directory; TUI will attempt git init")
    }
}

async fn memory_check(project_root: &Path) -> Check {
    let db_path = project_root.join(".dsx").join("sessions.db");
    match dsx_memory::open(&db_path).await {
        Ok(pool) => {
            pool.close().await;
            ok("memory", format!("SQLite ready at {}", db_path.display()))
        }
        Err(e) => fail("memory", format!("failed to open SQLite: {e}")),
    }
}

async fn run_ledger_check(project_root: &Path) -> Check {
    match crate::workspace_runs::running_run_count(project_root).await {
        Ok(0) => ok("run-ledger", "no unfinished agent runs found"),
        Ok(count) => warn(
            "run-ledger",
            format!("{count} unfinished running run(s); use `dsx workspace runs --all` to inspect"),
        ),
        Err(e) => warn("run-ledger", format!("failed to inspect run ledger: {e}")),
    }
}

async fn capsule_check(project_root: &Path) -> Check {
    let preview = match crate::context_preview::build_context_preview(project_root, "doctor").await
    {
        Ok(preview) => preview,
        Err(e) => return fail("capsule", format!("context preview failed: {e}")),
    };
    if let Err(e) = crate::context_preview::enforce_request_budget(&preview) {
        return fail("capsule", e.to_string());
    }
    if !capsule_parts_ready(&preview.task_parts) {
        return fail(
            "capsule",
            "structured task state is missing required fields",
        );
    }
    ok(
        "capsule",
        format!(
            "structured context ready; request ~{} / {} tokens",
            preview.metrics.estimated_request_tokens, preview.metrics.max_request_tokens
        ),
    )
}

fn capsule_parts_ready(parts: &dsx_agent::brief::TaskBriefParts) -> bool {
    [
        &parts.goal,
        &parts.done,
        &parts.plan,
        &parts.last_changes,
        &parts.next_step,
        &parts.active_scope,
        &parts.constraints,
        &parts.surface_architecture,
    ]
    .iter()
    .all(|value| !value.trim().is_empty())
}

fn line_limit_check(project_root: &Path) -> Check {
    match rust_line_violations(project_root, MAX_RS_LINES) {
        Ok(violations) if violations.is_empty() => line_pressure_check(project_root),
        Ok(violations) => fail("line-limit", format_line_counts(&violations)),
        Err(e) => fail("line-limit", format!("failed to scan Rust files: {e}")),
    }
}

fn line_pressure_check(project_root: &Path) -> Check {
    match rust_line_pressure(project_root, PRESSURE_RS_LINES, MAX_RS_LINES) {
        Ok(files) if files.is_empty() => ok(
            "line-limit",
            format!("all Rust files <= {MAX_RS_LINES} lines"),
        ),
        Ok(files) => warn(
            "line-limit",
            format!(
                "all Rust files <= {MAX_RS_LINES}; near limit: {}",
                format_line_counts(&files)
            ),
        ),
        Err(e) => fail("line-limit", format!("failed to scan Rust files: {e}")),
    }
}

fn format_line_counts(files: &[crate::line_limit::FileLineCount]) -> String {
    files
        .iter()
        .take(5)
        .map(|item| format!("{}={} lines", item.path.display(), item.lines))
        .collect::<Vec<_>>()
        .join(", ")
}

fn ok(name: &'static str, detail: impl Into<String>) -> Check {
    check(CheckStatus::Ok, name, detail)
}

fn warn(name: &'static str, detail: impl Into<String>) -> Check {
    check(CheckStatus::Warn, name, detail)
}

fn fail(name: &'static str, detail: impl Into<String>) -> Check {
    check(CheckStatus::Fail, name, detail)
}

fn check(status: CheckStatus, name: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status,
        name,
        detail: detail.into(),
    }
}

fn marker(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "[ok]",
        CheckStatus::Warn => "[warn]",
        CheckStatus::Fail => "[fail]",
    }
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
