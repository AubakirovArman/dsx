//! Best-effort app-level persistence for agent run ledger rows.

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct RunLedgerSnapshot {
    pub status: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub reasoning_tokens: u64,
    pub estimated_cost_usd: f64,
    pub compaction_events: u64,
    pub compacted_messages: u64,
    pub estimated_tokens_saved: u64,
    pub error: Option<String>,
    pub cancelled: bool,
}

impl RunLedgerSnapshot {
    pub fn from_app(app: &dsx_tui::App, status: &str, error: Option<String>) -> Self {
        Self {
            status: status.into(),
            prompt_tokens: app.tokens.saturating_sub(app.run_start_tokens),
            completion_tokens: 0,
            reasoning_tokens: 0,
            estimated_cost_usd: (app.cost - app.run_start_cost).max(0.0),
            compaction_events: app.compaction_events,
            compacted_messages: app.compacted_messages,
            estimated_tokens_saved: app.estimated_tokens_saved,
            cancelled: status == "cancelled",
            error,
        }
    }
}

pub async fn record_started(
    active_root: &Path,
    session_id: Option<&str>,
    task: &str,
) -> anyhow::Result<String> {
    let pool = open_pool(active_root).await?;
    let project_root = active_root.display().to_string();
    dsx_memory::start_agent_run(&pool, session_id, &project_root, task).await
}

pub async fn record_finished(
    active_root: &Path,
    ledger_id: &str,
    snapshot: RunLedgerSnapshot,
) -> anyhow::Result<()> {
    let pool = open_pool(active_root).await?;
    dsx_memory::finish_agent_run(&pool, ledger_id, &snapshot.into()).await
}

async fn open_pool(active_root: &Path) -> anyhow::Result<sqlx::SqlitePool> {
    dsx_memory::open(&active_root.join(".dsx").join("sessions.db")).await
}

impl From<RunLedgerSnapshot> for dsx_memory::AgentRunUpdate {
    fn from(value: RunLedgerSnapshot) -> Self {
        Self {
            status: value.status,
            prompt_tokens: value.prompt_tokens,
            completion_tokens: value.completion_tokens,
            reasoning_tokens: value.reasoning_tokens,
            estimated_cost_usd: value.estimated_cost_usd,
            compaction_events: value.compaction_events,
            compacted_messages: value.compacted_messages,
            estimated_tokens_saved: value.estimated_tokens_saved,
            error: value.error,
            cancelled: value.cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_uses_per_run_token_and_cost_delta() {
        let mut app = dsx_tui::App::new();
        app.run_start_tokens = 100;
        app.run_start_cost = 0.25;
        app.tokens = 160;
        app.cost = 0.40;
        app.compaction_events = 2;

        let snapshot = RunLedgerSnapshot::from_app(&app, "completed", None);

        assert_eq!(snapshot.prompt_tokens, 60);
        assert!((snapshot.estimated_cost_usd - 0.15).abs() < 0.0001);
        assert_eq!(snapshot.compaction_events, 2);
    }
}
