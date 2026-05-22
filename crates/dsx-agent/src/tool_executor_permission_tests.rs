use crate::tool_executor::{ToolContext, execute};
use dsx_core::types::{PermissionMode, RiskLevel};
use dsx_permissions::classify_command;
use dsx_provider::streaming::ToolCallReady;

#[tokio::test]
async fn test_permission_deny_in_yolo() {
    let tmp = std::env::temp_dir().join("dsx_test_yolo_sudo");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let call = ToolCallReady {
        id: "call_1".into(),
        name: "run_command".into(),
        arguments: r#"{"command":"sudo rm -rf /"}"#.into(),
    };
    let ctx = ToolContext {
        workspace: tmp.clone(),
        mode: PermissionMode::Yolo,
        approval_tx: None,
    };

    let risk = classify_command("sudo rm -rf /");
    assert_eq!(risk, RiskLevel::Blocked);

    let result = execute(&call, &ctx).await;
    assert!(!result.success);
    assert!(result.denied);
    assert!(result.content.contains("Permission denied"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn test_ask_mode_denies_without_approval_channel() {
    let tmp = std::env::temp_dir().join("dsx_test_ask_without_channel");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let call = ToolCallReady {
        id: "call_1".into(),
        name: "write_file".into(),
        arguments: r#"{"path":"new.txt","content":"hello"}"#.into(),
    };
    let ctx = ToolContext {
        workspace: tmp.clone(),
        mode: PermissionMode::Ask,
        approval_tx: None,
    };

    let result = execute(&call, &ctx).await;
    assert!(!result.success);
    assert!(result.denied);
    assert!(!tmp.join("new.txt").exists());

    let _ = std::fs::remove_dir_all(&tmp);
}
