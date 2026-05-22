//! Workspace run ledger listing across launch and child task scopes.

use std::path::{Path, PathBuf};

pub(crate) struct LocatedRun {
    pub(crate) db_path: PathBuf,
    pub(crate) run: dsx_memory::AgentRunRecord,
}

pub async fn list_agent_runs(project_root: &Path, limit: u32, all: bool, json: bool) {
    match collect_agent_runs(project_root, limit, all).await {
        Ok(runs) if json => println!(
            "{}",
            crate::workspace_runs_output::runs_json(project_root, limit, all, &runs)
        ),
        Ok(runs) if runs.is_empty() => println!("No agent runs yet."),
        Ok(runs) => crate::workspace_runs_output::print_runs(project_root, &runs, all),
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
#[path = "workspace_runs_tests.rs"]
mod tests;
