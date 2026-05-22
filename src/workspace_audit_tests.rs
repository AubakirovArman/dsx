use super::*;

#[tokio::test]
async fn audit_surfaces_scope_contracts_and_notes() {
    let root = temp_root("dsx_workspace_audit");
    let child = root.join("1234");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&child).unwrap();
    seed_run(&child, &root).await;
    seed_note(&child).await;

    let audit = collect_workspace_audit(&root, 10, true).await.unwrap();
    let value = audit_json(&audit);

    assert_eq!(audit.running_runs, 0);
    assert_eq!(audit.scope_violations, 1);
    assert!(audit.runs[0].contract.contains("Narrowed"));
    assert_eq!(value["line_limit"]["ok"], true);
    assert!(value["line_limit"]["pressure"].is_array());
    assert_eq!(value["runs"][0]["scope"], "1234");
    assert_eq!(value["runs"][0]["total_tokens"], 7);
    assert_eq!(value["runs"][0]["estimated_tokens_saved"], 120);

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn audit_surfaces_line_pressure_before_limit_failures() {
    let root = temp_root("dsx_workspace_audit_pressure");
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("almost.rs"),
        "fn repeated() {}\n".repeat(crate::line_limit::PRESSURE_RS_LINES),
    )
    .unwrap();

    let audit = collect_workspace_audit(&root, 10, true).await.unwrap();
    let text_status = line_status(&audit);
    let value = audit_json(&audit);

    assert!(audit.line_violations.is_empty());
    assert_eq!(audit.line_pressure.len(), 1);
    assert!(text_status.contains("pressure: src/almost.rs=270 lines"));
    assert_eq!(
        value["line_limit"]["pressure"][0],
        "src/almost.rs=270 lines"
    );

    let _ = std::fs::remove_dir_all(root);
}

async fn seed_run(child: &Path, root: &Path) {
    let pool = dsx_memory::open(&child.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    let id = dsx_memory::start_scoped_agent_run(
        &pool,
        &dsx_memory::AgentRunStart {
            session_id: None,
            project_root: &child.display().to_string(),
            task: "build 1234",
            launch_scope: &root.display().to_string(),
            active_scope: &child.display().to_string(),
            scope_status: "Narrowed",
            scope_reason: "Task selected a subfolder.",
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
            estimated_tokens_saved: 120,
            scope_violations: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

async fn seed_note(child: &Path) {
    let pool = dsx_memory::open(&child.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    let mut summary = dsx_memory::TaskSummary::new(&child.display().to_string());
    summary.next_step = "verify gates".into();
    dsx_memory::upsert_task_summary(&pool, &summary)
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
