//! Workspace run ledger listing across launch and child task scopes.

use std::path::{Path, PathBuf};

pub(crate) struct LocatedRun {
    pub(crate) db_path: PathBuf,
    pub(crate) run: dsx_memory::AgentRunRecord,
}

pub async fn list_agent_runs(project_root: &Path, limit: u32, all: bool) {
    match collect_agent_runs(project_root, limit, all).await {
        Ok(runs) if runs.is_empty() => println!("No agent runs yet."),
        Ok(runs) => print_runs(project_root, &runs, all),
        Err(e) => println!("DB error: {e}"),
    }
}

pub async fn running_run_count(project_root: &Path) -> anyhow::Result<usize> {
    let mut count = 0usize;
    for db_path in discover_run_dbs(project_root)? {
        count += count_running_rows(&db_path).await?;
    }
    Ok(count)
}

pub(crate) async fn collect_agent_runs(
    project_root: &Path,
    limit: u32,
    all: bool,
) -> anyhow::Result<Vec<LocatedRun>> {
    let db_paths = if all {
        discover_run_dbs(project_root)?
    } else {
        vec![project_root.join(".dsx").join("sessions.db")]
    };
    let mut runs = Vec::new();
    for db_path in db_paths {
        let pool = dsx_memory::open(&db_path).await?;
        let mut rows = if all {
            dsx_memory::recent_agent_runs_any(&pool, limit as i64).await?
        } else {
            let project_root = project_root.display().to_string();
            dsx_memory::recent_agent_runs(&pool, &project_root, limit as i64).await?
        };
        runs.extend(rows.drain(..).map(|run| LocatedRun {
            db_path: db_path.clone(),
            run,
        }));
    }
    runs.sort_by(|a, b| b.run.started_at.cmp(&a.run.started_at));
    runs.truncate(limit as usize);
    Ok(runs)
}

async fn count_running_rows(db_path: &Path) -> anyhow::Result<usize> {
    let pool = dsx_memory::open(db_path).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_runs WHERE status = 'running' AND finished_at IS NULL",
    )
    .fetch_one(&pool)
    .await?;
    Ok(count.max(0) as usize)
}

pub(crate) fn discover_run_dbs(project_root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dbs = Vec::new();
    visit(project_root, &mut dbs)?;
    dbs.sort();
    dbs.dedup();
    Ok(dbs)
}

fn visit(dir: &Path, dbs: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if skip_dir(dir) {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some(".dsx") {
                let db = path.join("sessions.db");
                if db.exists() {
                    dbs.push(db);
                }
                continue;
            }
            visit(&path, dbs)?;
        }
    }
    Ok(())
}

fn skip_dir(dir: &Path) -> bool {
    matches!(
        dir.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target" | "node_modules")
    )
}

fn print_runs(project_root: &Path, runs: &[LocatedRun], all: bool) {
    println!(
        "Recent agent runs{}:",
        if all { " across scopes" } else { "" }
    );
    for located in runs {
        print_run(project_root, located, all);
    }
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

fn scope_text(value: &str) -> &str {
    if value.trim().is_empty() { "." } else { value }
}

fn scope_label(project_root: &Path, located: &LocatedRun) -> String {
    scope_label_for_db(project_root, &located.db_path)
}

pub(crate) fn scope_label_for_db(project_root: &Path, db_path: &Path) -> String {
    let db_scope = db_path
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or(project_root);
    db_scope
        .strip_prefix(project_root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn all_runs_discovers_child_scope_ledgers() {
        let root = temp_root("dsx_runs_all");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();

        seed_run(&root, "root task").await;
        seed_run(&child, "child task").await;

        let runs = collect_agent_runs(&root, 10, true).await.unwrap();

        assert_eq!(runs.len(), 2);
        assert!(runs.iter().any(|run| scope_label(&root, run) == "."));
        assert!(runs.iter().any(|run| scope_label(&root, run) == "1234"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn default_runs_stays_on_launch_scope() {
        let root = temp_root("dsx_runs_root_only");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();

        seed_run(&root, "root task").await;
        seed_run(&child, "child task").await;

        let runs = collect_agent_runs(&root, 10, false).await.unwrap();

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run.task_excerpt, "root task");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn running_count_reports_unfinished_runs_across_scopes() {
        let root = temp_root("dsx_runs_running_count");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();

        seed_run(&root, "done task").await;
        seed_running_run(&child, "stuck task").await;

        let count = running_run_count(&root).await.unwrap();

        assert_eq!(count, 1);

        let _ = std::fs::remove_dir_all(root);
    }

    async fn seed_run(root: &Path, task: &str) {
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let id = dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), task)
            .await
            .unwrap();
        dsx_memory::finish_agent_run(
            &pool,
            &id,
            &dsx_memory::AgentRunUpdate {
                status: "completed".into(),
                prompt_tokens: 1,
                scope_violations: 1,
                last_scope_violation: "read_file: denied by active scope".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    async fn seed_running_run(root: &Path, task: &str) {
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), task)
            .await
            .unwrap();
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
