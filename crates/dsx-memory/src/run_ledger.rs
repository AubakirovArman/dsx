//! Agent run ledger for cost, status, and compaction audit trails.

use crate::run_ledger_row::AgentRunRow;
use chrono::Utc;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentRunUpdate {
    pub status: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub reasoning_tokens: u64,
    pub estimated_cost_usd: f64,
    pub compaction_events: u64,
    pub compacted_messages: u64,
    pub estimated_tokens_saved: u64,
    pub scope_violations: u64,
    pub last_scope_violation: String,
    pub error: Option<String>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentRunRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub project_root: String,
    pub task_excerpt: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub compaction_events: i64,
    pub compacted_messages: i64,
    pub estimated_tokens_saved: i64,
    pub launch_scope: String,
    pub active_scope: String,
    pub scope_status: String,
    pub scope_reason: String,
    pub scope_violations: i64,
    pub last_scope_violation: String,
    pub error: Option<String>,
    pub cancelled: bool,
}

pub struct AgentRunStart<'a> {
    pub session_id: Option<&'a str>,
    pub project_root: &'a str,
    pub task: &'a str,
    pub launch_scope: &'a str,
    pub active_scope: &'a str,
    pub scope_status: &'a str,
    pub scope_reason: &'a str,
}

pub async fn start_agent_run(
    pool: &SqlitePool,
    session_id: Option<&str>,
    project_root: &str,
    task: &str,
) -> anyhow::Result<String> {
    start_scoped_agent_run(
        pool,
        &AgentRunStart {
            session_id,
            project_root,
            task,
            launch_scope: project_root,
            active_scope: project_root,
            scope_status: "legacy",
            scope_reason: "Legacy start without explicit scope contract.",
        },
    )
    .await
}

pub async fn start_scoped_agent_run(
    pool: &SqlitePool,
    start: &AgentRunStart<'_>,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO agent_runs (
            id, session_id, project_root, task_excerpt, status, started_at,
            launch_scope, active_scope, scope_status, scope_reason
        )
        VALUES (?, ?, ?, ?, 'running', ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(start.session_id)
    .bind(start.project_root)
    .bind(excerpt(start.task, 1_200))
    .bind(now)
    .bind(start.launch_scope)
    .bind(start.active_scope)
    .bind(start.scope_status)
    .bind(start.scope_reason)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn finish_agent_run(
    pool: &SqlitePool,
    id: &str,
    update: &AgentRunUpdate,
) -> anyhow::Result<()> {
    let finished_at = Utc::now().to_rfc3339();
    let total = update.prompt_tokens + update.completion_tokens + update.reasoning_tokens;
    sqlx::query(
        r#"
        UPDATE agent_runs
        SET status = ?,
            finished_at = ?,
            prompt_tokens = ?,
            completion_tokens = ?,
            reasoning_tokens = ?,
            total_tokens = ?,
            estimated_cost_usd = ?,
            compaction_events = ?,
            compacted_messages = ?,
            estimated_tokens_saved = ?,
            scope_violations = ?,
            last_scope_violation = ?,
            error = ?,
            cancelled = ?
        WHERE id = ?
        "#,
    )
    .bind(&update.status)
    .bind(finished_at)
    .bind(to_i64(update.prompt_tokens))
    .bind(to_i64(update.completion_tokens))
    .bind(to_i64(update.reasoning_tokens))
    .bind(to_i64(total))
    .bind(update.estimated_cost_usd)
    .bind(to_i64(update.compaction_events))
    .bind(to_i64(update.compacted_messages))
    .bind(to_i64(update.estimated_tokens_saved))
    .bind(to_i64(update.scope_violations))
    .bind(&update.last_scope_violation)
    .bind(&update.error)
    .bind(i64::from(update.cancelled))
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_agent_run(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<AgentRunRecord>> {
    let row = sqlx::query_as::<_, AgentRunRow>(
        r#"
        SELECT id, session_id, project_root, task_excerpt, status, started_at,
               finished_at, prompt_tokens, completion_tokens, reasoning_tokens,
               total_tokens, estimated_cost_usd, compaction_events,
               compacted_messages, estimated_tokens_saved, launch_scope,
               active_scope, scope_status, scope_reason, scope_violations,
               last_scope_violation, error, cancelled
        FROM agent_runs
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(Into::into))
}

pub async fn recent_agent_runs(
    pool: &SqlitePool,
    project_root: &str,
    limit: i64,
) -> anyhow::Result<Vec<AgentRunRecord>> {
    let rows = sqlx::query_as::<_, AgentRunRow>(
        r#"
        SELECT id, session_id, project_root, task_excerpt, status, started_at,
               finished_at, prompt_tokens, completion_tokens, reasoning_tokens,
               total_tokens, estimated_cost_usd, compaction_events,
               compacted_messages, estimated_tokens_saved, launch_scope,
               active_scope, scope_status, scope_reason, scope_violations,
               last_scope_violation, error, cancelled
        FROM agent_runs
        WHERE project_root = ?
        ORDER BY started_at DESC
        LIMIT ?
        "#,
    )
    .bind(project_root)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn recent_agent_runs_any(
    pool: &SqlitePool,
    limit: i64,
) -> anyhow::Result<Vec<AgentRunRecord>> {
    let rows = sqlx::query_as::<_, AgentRunRow>(
        r#"
        SELECT id, session_id, project_root, task_excerpt, status, started_at,
               finished_at, prompt_tokens, completion_tokens, reasoning_tokens,
               total_tokens, estimated_cost_usd, compaction_events,
               compacted_messages, estimated_tokens_saved, launch_scope,
               active_scope, scope_status, scope_reason, scope_violations,
               last_scope_violation, error, cancelled
        FROM agent_runs
        ORDER BY started_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

fn excerpt(value: &str, limit: usize) -> String {
    let cleaned = value.trim();
    let mut out: String = cleaned.chars().take(limit).collect();
    if cleaned.chars().count() > limit {
        out.push_str("...");
    }
    out
}

fn to_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

#[cfg(test)]
#[path = "run_ledger_tests.rs"]
mod tests;
