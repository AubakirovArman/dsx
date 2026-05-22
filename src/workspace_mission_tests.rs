use super::*;

#[tokio::test]
async fn mission_snapshot_uses_saved_note_and_run_health() {
    let root = temp_root("dsx_workspace_mission");
    let child = root.join("1234");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&child).unwrap();
    seed_summary(&root).await;
    seed_finished_run(&child).await;

    let snapshot = collect_mission_snapshot(&root, 6, true).await.unwrap();
    let value = mission_json(&snapshot);

    assert_eq!(snapshot.goal, "ship mission view");
    assert_eq!(snapshot.run_health.recent_runs, 1);
    assert_eq!(snapshot.run_health.scope_violations, 1);
    assert_eq!(value["mission"]["goal"], "ship mission view");
    assert_eq!(value["line_limit"]["ok"], true);
    assert!(value["scopes"].as_array().unwrap().len() >= 2);

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn mission_snapshot_falls_back_without_saved_notes() {
    let root = temp_root("dsx_workspace_mission_fallback");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let snapshot = collect_mission_snapshot(&root, 2, false).await.unwrap();

    assert_eq!(snapshot.goal, "No saved goal yet.");
    assert_eq!(snapshot.active_scope, root.display().to_string());

    let _ = std::fs::remove_dir_all(root);
}

async fn seed_summary(root: &std::path::Path) {
    let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    let mut summary = dsx_memory::TaskSummary::new(&root.display().to_string());
    summary.goal = "ship mission view".into();
    summary.done = "mission state persisted".into();
    summary.plan = "1. inspect\n2. verify".into();
    summary.last_changes = "added snapshot".into();
    summary.next_step = "run gates".into();
    summary.architecture = "workspace mission module".into();
    dsx_memory::upsert_task_summary(&pool, &summary)
        .await
        .unwrap();
}

async fn seed_finished_run(root: &std::path::Path) {
    let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    let id = dsx_memory::start_scoped_agent_run(
        &pool,
        &dsx_memory::AgentRunStart {
            session_id: None,
            project_root: &root.display().to_string(),
            task: "mission test",
            launch_scope: &root.display().to_string(),
            active_scope: &root.display().to_string(),
            scope_status: "Narrowed",
            scope_reason: "test",
        },
    )
    .await
    .unwrap();
    dsx_memory::finish_agent_run(
        &pool,
        &id,
        &dsx_memory::AgentRunUpdate {
            status: "completed".into(),
            prompt_tokens: 7,
            scope_violations: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

fn temp_root(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{name}_{nanos}"))
}
