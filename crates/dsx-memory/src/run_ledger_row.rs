//! SQLite row mapping for agent run ledger records.

use crate::run_ledger::AgentRunRecord;

#[derive(sqlx::FromRow)]
pub(crate) struct AgentRunRow {
    pub(crate) id: String,
    pub(crate) session_id: Option<String>,
    pub(crate) project_root: String,
    pub(crate) task_excerpt: String,
    pub(crate) status: String,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) prompt_tokens: i64,
    pub(crate) completion_tokens: i64,
    pub(crate) reasoning_tokens: i64,
    pub(crate) total_tokens: i64,
    pub(crate) estimated_cost_usd: f64,
    pub(crate) compaction_events: i64,
    pub(crate) compacted_messages: i64,
    pub(crate) estimated_tokens_saved: i64,
    pub(crate) launch_scope: String,
    pub(crate) active_scope: String,
    pub(crate) scope_status: String,
    pub(crate) scope_reason: String,
    pub(crate) scope_violations: i64,
    pub(crate) last_scope_violation: String,
    pub(crate) error: Option<String>,
    pub(crate) cancelled: i64,
}

impl From<AgentRunRow> for AgentRunRecord {
    fn from(row: AgentRunRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            project_root: row.project_root,
            task_excerpt: row.task_excerpt,
            status: row.status,
            started_at: row.started_at,
            finished_at: row.finished_at,
            prompt_tokens: row.prompt_tokens,
            completion_tokens: row.completion_tokens,
            reasoning_tokens: row.reasoning_tokens,
            total_tokens: row.total_tokens,
            estimated_cost_usd: row.estimated_cost_usd,
            compaction_events: row.compaction_events,
            compacted_messages: row.compacted_messages,
            estimated_tokens_saved: row.estimated_tokens_saved,
            launch_scope: row.launch_scope,
            active_scope: row.active_scope,
            scope_status: row.scope_status,
            scope_reason: row.scope_reason,
            scope_violations: row.scope_violations,
            last_scope_violation: row.last_scope_violation,
            error: row.error,
            cancelled: row.cancelled != 0,
        }
    }
}
