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
    pub scope_violations: u64,
    pub last_scope_violation: String,
    pub error: Option<String>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunScopeContract {
    pub launch_scope: String,
    pub active_scope: String,
    pub scope_status: String,
    pub scope_reason: String,
}

impl RunScopeContract {
    pub fn from_app(app: &dsx_tui::App) -> Self {
        Self {
            launch_scope: app.scope_lock.launch_scope.clone(),
            active_scope: app.scope_lock.active_scope.clone(),
            scope_status: app.scope_lock.status.clone(),
            scope_reason: app.scope_lock.reason.clone(),
        }
    }
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
            scope_violations: app.scope_violations,
            last_scope_violation: app.last_scope_violation.clone(),
            cancelled: status == "cancelled",
            error,
        }
    }
}

pub async fn record_started(
    active_root: &Path,
    session_id: Option<&str>,
    task: &str,
    contract: RunScopeContract,
) -> anyhow::Result<String> {
    let pool = open_pool(active_root).await?;
    let project_root = active_root.display().to_string();
    dsx_memory::start_scoped_agent_run(
        &pool,
        &dsx_memory::AgentRunStart {
            session_id,
            project_root: &project_root,
            task,
            launch_scope: &contract.launch_scope,
            active_scope: &contract.active_scope,
            scope_status: &contract.scope_status,
            scope_reason: &contract.scope_reason,
        },
    )
    .await
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
            scope_violations: value.scope_violations,
            last_scope_violation: value.last_scope_violation,
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
        app.scope_violations = 1;
        app.last_scope_violation = "read_file: denied".into();

        let snapshot = RunLedgerSnapshot::from_app(&app, "completed", None);

        assert_eq!(snapshot.prompt_tokens, 60);
        assert!((snapshot.estimated_cost_usd - 0.15).abs() < 0.0001);
        assert_eq!(snapshot.compaction_events, 2);
        assert_eq!(snapshot.scope_violations, 1);
        assert!(snapshot.last_scope_violation.contains("read_file"));
    }

    #[tokio::test]
    async fn record_started_persists_scope_contract() {
        let root = temp_root("dsx_run_contract");
        let active = root.join("1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&active).unwrap();
        let mut app = dsx_tui::App::new();
        app.begin_task_scoped(
            "build",
            &root.display().to_string(),
            &active.display().to_string(),
            true,
        );
        let contract = RunScopeContract::from_app(&app);

        let id = record_started(&active, None, "build", contract)
            .await
            .unwrap();
        let pool = dsx_memory::open(&active.join(".dsx").join("sessions.db"))
            .await
            .unwrap();
        let run = dsx_memory::load_agent_run(&pool, &id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(run.launch_scope, root.display().to_string());
        assert_eq!(run.active_scope, active.display().to_string());
        assert_eq!(run.scope_status, "Narrowed");

        pool.close().await;
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
