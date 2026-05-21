//! DSX Memory — SQLite-backed persistent storage.
//!
//! Schema follows section 11.6 of the architecture document:
//! - sessions, events, usage_records, memory_items, file_summaries,
//!   command_runs, patches, checkpoints, symbols

use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub project_root: Option<String>,
    pub scope: String,
    pub type_: String,
    pub content: String,
    pub confidence: f64,
    pub updated_at: String,
}

/// Open (or create) the memory database at the given path.
pub async fn open(path: &Path) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let pool = SqlitePool::connect(&url).await?;

    // Configure SQLite performance and high concurrency optimizations
    let _ = sqlx::query("PRAGMA journal_mode=WAL;").execute(&pool).await;
    let _ = sqlx::query("PRAGMA busy_timeout=10000;")
        .execute(&pool)
        .await;

    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project_root TEXT NOT NULL,
            mode TEXT NOT NULL DEFAULT 'ask',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            message_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            ts TEXT NOT NULL,
            type TEXT NOT NULL,
            data_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS usage_records (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            ts TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            thinking_enabled INTEGER NOT NULL,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            reasoning_tokens INTEGER,
            cache_hit_tokens INTEGER,
            cache_miss_tokens INTEGER,
            estimated_cost_usd REAL
        );

        CREATE TABLE IF NOT EXISTS memory_items (
            id TEXT PRIMARY KEY,
            project_root TEXT,
            scope TEXT NOT NULL,
            type TEXT NOT NULL,
            content TEXT NOT NULL,
            source_event_ids_json TEXT NOT NULL,
            confidence REAL NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            expires_at TEXT,
            archived INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS file_summaries (
            id TEXT PRIMARY KEY,
            project_root TEXT NOT NULL,
            path TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            language TEXT,
            summary TEXT NOT NULL,
            symbols_json TEXT,
            imports_json TEXT,
            updated_at TEXT NOT NULL,
            UNIQUE(project_root, path, file_hash)
        );

        CREATE TABLE IF NOT EXISTS command_runs (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            ts TEXT NOT NULL,
            cwd TEXT NOT NULL,
            command TEXT NOT NULL,
            risk_level TEXT NOT NULL,
            approved_by TEXT,
            exit_code INTEGER,
            duration_ms INTEGER,
            stdout_excerpt TEXT,
            stderr_excerpt TEXT,
            output_truncated INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS patches (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            ts TEXT NOT NULL,
            summary TEXT NOT NULL,
            status TEXT NOT NULL,
            base_git_sha TEXT,
            patch_json TEXT NOT NULL,
            validation_json TEXT,
            user_feedback TEXT
        );

        CREATE TABLE IF NOT EXISTS checkpoints (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            ts TEXT NOT NULL,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            git_ref TEXT,
            dirty_state_hash TEXT,
            metadata_json TEXT
        );

        CREATE TABLE IF NOT EXISTS symbols (
            id TEXT PRIMARY KEY,
            project_root TEXT NOT NULL,
            path TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            language TEXT,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            signature TEXT NOT NULL,
            parent_symbol_id TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Return recent active memories for the project and global user scope.
pub async fn recent_memory_items(
    pool: &SqlitePool,
    project_root: &str,
    limit: i64,
) -> anyhow::Result<Vec<MemoryItem>> {
    let rows = sqlx::query_as::<_, MemoryItemRow>(
        r#"
        SELECT id, project_root, scope, type, content, confidence, updated_at
        FROM memory_items
        WHERE archived = 0
          AND (project_root IS NULL OR project_root = ?)
        ORDER BY updated_at DESC
        LIMIT ?
        "#,
    )
    .bind(project_root)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MemoryItem {
            id: row.id,
            project_root: row.project_root,
            scope: row.scope,
            type_: row.type_,
            content: row.content,
            confidence: row.confidence,
            updated_at: row.updated_at,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct MemoryItemRow {
    id: String,
    project_root: Option<String>,
    scope: String,
    #[sqlx(rename = "type")]
    type_: String,
    content: String,
    confidence: f64,
    updated_at: String,
}
