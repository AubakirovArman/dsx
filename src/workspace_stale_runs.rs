//! Mark orphaned running run ledger rows as stale.

use std::path::Path;

struct StaleCloseResult {
    scope: String,
    matched: i64,
}

pub async fn close_stale_runs(project_root: &Path, older_than_minutes: i64, dry_run: bool) {
    let older_than_minutes = older_than_minutes.max(0);
    match close_stale_runs_inner(project_root, older_than_minutes, dry_run).await {
        Ok(results) => print_results(older_than_minutes, dry_run, &results),
        Err(e) => println!("DB error: {e}"),
    }
}

pub(crate) async fn stale_run_count(
    project_root: &Path,
    older_than_minutes: i64,
) -> anyhow::Result<i64> {
    let mut total = 0;
    for db_path in crate::workspace_runs::discover_run_dbs(project_root)? {
        total += count_stale_rows(&db_path, older_than_minutes.max(0)).await?;
    }
    Ok(total)
}

async fn close_stale_runs_inner(
    project_root: &Path,
    older_than_minutes: i64,
    dry_run: bool,
) -> anyhow::Result<Vec<StaleCloseResult>> {
    let mut results = Vec::new();
    for db_path in crate::workspace_runs::discover_run_dbs(project_root)? {
        let matched = if dry_run {
            count_stale_rows(&db_path, older_than_minutes).await?
        } else {
            close_stale_rows(&db_path, older_than_minutes).await?
        };
        if matched > 0 {
            results.push(StaleCloseResult {
                scope: crate::workspace_runs::scope_label_for_db(project_root, &db_path),
                matched,
            });
        }
    }
    Ok(results)
}

async fn count_stale_rows(db_path: &Path, older_than_minutes: i64) -> anyhow::Result<i64> {
    let pool = dsx_memory::open(db_path).await?;
    let count = sqlx::query_scalar::<_, i64>(STALE_COUNT_SQL)
        .bind(stale_modifier(older_than_minutes))
        .fetch_one(&pool)
        .await?;
    Ok(count)
}

async fn close_stale_rows(db_path: &Path, older_than_minutes: i64) -> anyhow::Result<i64> {
    let pool = dsx_memory::open(db_path).await?;
    let result = sqlx::query(STALE_CLOSE_SQL)
        .bind(format!(
            "marked stale by dsx workspace close-stale-runs after {older_than_minutes} minute(s)"
        ))
        .bind(stale_modifier(older_than_minutes))
        .execute(&pool)
        .await?;
    Ok(result.rows_affected().min(i64::MAX as u64) as i64)
}

fn stale_modifier(minutes: i64) -> String {
    format!("-{minutes} minutes")
}

fn print_results(older_than_minutes: i64, dry_run: bool, results: &[StaleCloseResult]) {
    let action = if dry_run { "Would mark" } else { "Marked" };
    let total: i64 = results.iter().map(|item| item.matched).sum();
    if total == 0 {
        println!("No stale running runs older than {older_than_minutes} minute(s).");
        return;
    }
    println!("{action} {total} stale running run(s):");
    for result in results {
        println!("  {}  {}", result.scope, result.matched);
    }
}

const STALE_COUNT_SQL: &str = r#"
SELECT COUNT(*)
FROM agent_runs
WHERE status = 'running'
  AND finished_at IS NULL
  AND datetime(started_at) <= datetime('now', ?)
"#;

const STALE_CLOSE_SQL: &str = r#"
UPDATE agent_runs
SET status = 'stale',
    finished_at = strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now'),
    error = ?
WHERE status = 'running'
  AND finished_at IS NULL
  AND datetime(started_at) <= datetime('now', ?)
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn dry_run_counts_without_updating() {
        let root = temp_root("dsx_stale_dry_run");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let id = seed_running_run(&root, "stuck").await;

        let results = close_stale_runs_inner(&root, 0, true).await.unwrap();

        assert_eq!(results[0].matched, 1);
        assert_eq!(load_status(&root, &id).await, "running");

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn closes_stale_runs_across_child_scopes() {
        let root = temp_root("dsx_stale_child");
        let child = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&child).unwrap();
        let id = seed_running_run(&child, "stuck").await;

        let results = close_stale_runs_inner(&root, 0, false).await.unwrap();

        assert_eq!(results[0].scope, "1234");
        assert_eq!(results[0].matched, 1);
        assert_eq!(load_status(&child, &id).await, "stale");

        let _ = std::fs::remove_dir_all(root);
    }

    async fn seed_running_run(root: &Path, task: &str) -> String {
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), task)
            .await
            .unwrap()
    }

    async fn load_status(root: &Path, id: &str) -> String {
        let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        dsx_memory::load_agent_run(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .status
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
