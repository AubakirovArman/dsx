//! Tests for agent run ledger persistence.

use super::*;

#[tokio::test]
async fn stores_and_finishes_agent_run() {
    let db_path = std::env::temp_dir().join(format!("dsx_agent_run_{}.db", uuid::Uuid::new_v4()));
    let pool = crate::open(&db_path).await.unwrap();

    let id = start_agent_run(&pool, Some("sid"), "/tmp/project", "do useful work")
        .await
        .unwrap();
    finish_agent_run(
        &pool,
        &id,
        &AgentRunUpdate {
            status: "completed".into(),
            prompt_tokens: 10,
            completion_tokens: 5,
            reasoning_tokens: 2,
            estimated_cost_usd: 0.01,
            compaction_events: 1,
            compacted_messages: 8,
            estimated_tokens_saved: 120,
            scope_violations: 2,
            last_scope_violation: "read_file: denied by active scope".into(),
            error: None,
            cancelled: false,
        },
    )
    .await
    .unwrap();

    let run = load_agent_run(&pool, &id).await.unwrap().unwrap();

    assert_eq!(run.session_id.as_deref(), Some("sid"));
    assert_eq!(run.status, "completed");
    assert_eq!(run.total_tokens, 17);
    assert_eq!(run.compaction_events, 1);
    assert_eq!(run.estimated_tokens_saved, 120);
    assert_eq!(run.launch_scope, "/tmp/project");
    assert_eq!(run.active_scope, "/tmp/project");
    assert_eq!(run.scope_violations, 2);
    assert!(run.last_scope_violation.contains("read_file"));
    assert!(run.finished_at.is_some());
    assert_eq!(
        recent_agent_runs(&pool, "/tmp/project", 10)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(recent_agent_runs_any(&pool, 10).await.unwrap().len(), 1);

    let _ = pool.close().await;
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn stores_pre_run_scope_contract() {
    let db_path = std::env::temp_dir().join(format!("dsx_agent_scope_{}.db", uuid::Uuid::new_v4()));
    let pool = crate::open(&db_path).await.unwrap();

    let id = start_scoped_agent_run(
        &pool,
        &AgentRunStart {
            session_id: None,
            project_root: "/tmp/sites/1234",
            task: "build only 1234",
            launch_scope: "/tmp/sites",
            active_scope: "/tmp/sites/1234",
            scope_status: "Narrowed",
            scope_reason: "Task selected a subfolder.",
        },
    )
    .await
    .unwrap();

    let run = load_agent_run(&pool, &id).await.unwrap().unwrap();

    assert_eq!(run.launch_scope, "/tmp/sites");
    assert_eq!(run.active_scope, "/tmp/sites/1234");
    assert_eq!(run.scope_status, "Narrowed");
    assert!(run.scope_reason.contains("subfolder"));

    let _ = pool.close().await;
    let _ = std::fs::remove_file(db_path);
}
