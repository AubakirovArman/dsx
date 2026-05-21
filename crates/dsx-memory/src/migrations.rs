//! Lightweight additive migrations for existing SQLite databases.

use sqlx::{Row, SqlitePool};

pub(crate) async fn add_missing_columns(pool: &SqlitePool) -> anyhow::Result<()> {
    add_column(
        pool,
        "agent_runs",
        "launch_scope",
        "launch_scope TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column(
        pool,
        "agent_runs",
        "active_scope",
        "active_scope TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column(
        pool,
        "agent_runs",
        "scope_status",
        "scope_status TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column(
        pool,
        "agent_runs",
        "scope_reason",
        "scope_reason TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column(
        pool,
        "agent_runs",
        "scope_violations",
        "scope_violations INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column(
        pool,
        "agent_runs",
        "last_scope_violation",
        "last_scope_violation TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    add_column(
        pool,
        "task_summaries",
        "scope_violations",
        "scope_violations INTEGER NOT NULL DEFAULT 0",
    )
    .await?;
    add_column(
        pool,
        "task_summaries",
        "last_scope_violation",
        "last_scope_violation TEXT NOT NULL DEFAULT ''",
    )
    .await
}

async fn add_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    for row in sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await?
    {
        let name: String = row.try_get("name")?;
        if name == column {
            return Ok(());
        }
    }
    sqlx::query(&format!("ALTER TABLE {table} ADD COLUMN {definition}"))
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn adds_scope_columns_to_existing_tables() {
        let db_path =
            std::env::temp_dir().join(format!("dsx_migration_{}.db", uuid::Uuid::new_v4()));
        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&url).await.unwrap();
        sqlx::query(
            "CREATE TABLE agent_runs (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                project_root TEXT NOT NULL,
                task_excerpt TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                reasoning_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                estimated_cost_usd REAL NOT NULL DEFAULT 0,
                compaction_events INTEGER NOT NULL DEFAULT 0,
                compacted_messages INTEGER NOT NULL DEFAULT 0,
                estimated_tokens_saved INTEGER NOT NULL DEFAULT 0,
                error TEXT,
                cancelled INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE task_summaries (
                project_root TEXT PRIMARY KEY,
                goal TEXT NOT NULL DEFAULT '',
                done TEXT NOT NULL DEFAULT '',
                plan TEXT NOT NULL DEFAULT '',
                last_changes TEXT NOT NULL DEFAULT '',
                next_step TEXT NOT NULL DEFAULT '',
                active_scope TEXT NOT NULL DEFAULT '',
                constraints TEXT NOT NULL DEFAULT '',
                architecture TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool = crate::open(&db_path).await.unwrap();

        assert!(has_column(&pool, "agent_runs", "scope_violations").await);
        assert!(has_column(&pool, "agent_runs", "last_scope_violation").await);
        assert!(has_column(&pool, "agent_runs", "launch_scope").await);
        assert!(has_column(&pool, "agent_runs", "active_scope").await);
        assert!(has_column(&pool, "agent_runs", "scope_status").await);
        assert!(has_column(&pool, "agent_runs", "scope_reason").await);
        assert!(has_column(&pool, "task_summaries", "scope_violations").await);
        assert!(has_column(&pool, "task_summaries", "last_scope_violation").await);

        pool.close().await;
        let _ = std::fs::remove_file(db_path);
    }

    async fn has_column(pool: &SqlitePool, table: &str, column: &str) -> bool {
        let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
            .fetch_all(pool)
            .await
            .unwrap();
        rows.iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .any(|name| name == column)
    }
}
