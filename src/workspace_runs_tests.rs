use super::*;

#[tokio::test]
async fn all_runs_discovers_child_scope_ledgers() {
    let root = temp_root("dsx_runs_all");
    let child = root.join("1234");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&child).unwrap();

    seed_run(&root, "root task").await;
    seed_run(&child, "child task").await;

    let runs = collect_agent_runs(&root, 10, true).await.unwrap();

    assert_eq!(runs.len(), 2);
    assert!(runs.iter().any(|run| scope_label(&root, run) == "."));
    assert!(runs.iter().any(|run| scope_label(&root, run) == "1234"));

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn default_runs_stays_on_launch_scope() {
    let root = temp_root("dsx_runs_root_only");
    let child = root.join("1234");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&child).unwrap();

    seed_run(&root, "root task").await;
    seed_run(&child, "child task").await;

    let runs = collect_agent_runs(&root, 10, false).await.unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run.task_excerpt, "root task");

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn running_count_reports_unfinished_runs_across_scopes() {
    let root = temp_root("dsx_runs_running_count");
    let child = root.join("1234");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&child).unwrap();

    seed_run(&root, "done task").await;
    seed_running_run(&child, "stuck task").await;

    let count = running_run_count(&root).await.unwrap();

    assert_eq!(count, 1);

    let _ = std::fs::remove_dir_all(root);
}

async fn seed_run(root: &Path, task: &str) {
    let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    let id = dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), task)
        .await
        .unwrap();
    dsx_memory::finish_agent_run(
        &pool,
        &id,
        &dsx_memory::AgentRunUpdate {
            status: "completed".into(),
            prompt_tokens: 1,
            scope_violations: 1,
            last_scope_violation: "read_file: denied by active scope".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

async fn seed_running_run(root: &Path, task: &str) {
    let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), task)
        .await
        .unwrap();
}

fn temp_root(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{name}_{nanos}"))
}
