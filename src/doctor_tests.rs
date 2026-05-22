use super::*;
use std::path::PathBuf;

#[test]
fn detects_rust_files_over_line_limit() {
    let root = temp_root("dsx_doctor_line_limit");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/ok.rs"), "x\n".repeat(3)).unwrap();
    std::fs::write(root.join("src/too_long.rs"), "x\n".repeat(4)).unwrap();
    std::fs::create_dir_all(root.join("target/debug")).unwrap();
    std::fs::write(root.join("target/debug/ignored.rs"), "x\n".repeat(10)).unwrap();

    let violations = crate::line_limit::rust_line_violations(&root, 3).unwrap();

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].path, PathBuf::from("src/too_long.rs"));
    assert_eq!(violations[0].lines, 4);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn warns_on_rust_files_near_line_limit() {
    let root = temp_root("dsx_doctor_line_pressure");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/almost.rs"),
        "x\n".repeat(crate::line_limit::PRESSURE_RS_LINES),
    )
    .unwrap();

    let check = line_limit_check(&root);

    assert_eq!(check.status, CheckStatus::Warn);
    assert!(check.detail.contains("near limit"));

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn capsule_check_reports_structured_context() {
    let root = temp_root("dsx_doctor_capsule");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

    let check = capsule_check(&root).await;

    assert_eq!(check.status, CheckStatus::Ok);
    assert_eq!(check.name, "capsule");
    assert!(check.detail.contains("structured context ready"));

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn mission_health_reports_clean_snapshot() {
    let root = temp_root("dsx_doctor_mission_clean");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let check = mission_health_check(&root).await;

    assert_eq!(check.status, CheckStatus::Ok);
    assert_eq!(check.name, "mission");
    assert!(check.detail.contains("handoff clean"));

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn mission_health_fails_on_scope_violations() {
    let root = temp_root("dsx_doctor_mission_scope");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    seed_scope_violation_run(&root).await;

    let check = mission_health_check(&root).await;

    assert_eq!(check.status, CheckStatus::Fail);
    assert!(check.detail.contains("blocked scope"));

    let _ = std::fs::remove_dir_all(root);
}

async fn seed_scope_violation_run(root: &std::path::Path) {
    let pool = dsx_memory::open(&root.join(".dsx").join("sessions.db"))
        .await
        .unwrap();
    let id = dsx_memory::start_agent_run(&pool, None, &root.display().to_string(), "scope test")
        .await
        .unwrap();
    dsx_memory::finish_agent_run(
        &pool,
        &id,
        &dsx_memory::AgentRunUpdate {
            status: "completed".into(),
            scope_violations: 1,
            last_scope_violation: "read_file: denied by active scope".into(),
            ..Default::default()
        },
    )
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
