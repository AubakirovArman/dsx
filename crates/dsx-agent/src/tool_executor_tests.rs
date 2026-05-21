//! DSX Agent — unit tests for tool_executor.

#[cfg(test)]
mod tests {
    use crate::tool_executor::{execute, ToolContext, tool_risk};
    use dsx_core::types::{RiskLevel, PermissionMode};
    use dsx_permissions::classify_command;
    use dsx_provider::streaming::ToolCallReady;

    fn test_ctx(tmp: &std::path::Path) -> ToolContext {
        ToolContext {
            workspace: tmp.to_path_buf(),
            mode: PermissionMode::Ask,
            approval_tx: None,
        }
    }

    #[test]
    fn test_read_file_found() {
        let tmp = std::env::temp_dir().join("dsx_test_read");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("hello.txt"), "hello world").unwrap();

        let args = serde_json::json!({"path": "hello.txt"});
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_read_file("call_1", &args, &ctx);

        assert!(result.success);
        assert!(result.content.contains("hello world"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_file_not_found() {
        let tmp = std::env::temp_dir().join("dsx_test_not_found");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let args = serde_json::json!({"path": "missing.txt"});
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_read_file("call_1", &args, &ctx);

        assert!(!result.success);
        assert!(result.content.contains("error") || result.content.contains("Path error") || result.content.contains("Error reading"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_list_files() {
        let tmp = std::env::temp_dir().join("dsx_test_list");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("a.txt"), "a").unwrap();
        std::fs::write(tmp.join("b.txt"), "b").unwrap();

        let args = serde_json::json!({"path": "."});
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_list_files("call_1", &args, &ctx);

        assert!(result.success);
        assert!(result.content.contains("a.txt"));
        assert!(result.content.contains("b.txt"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_grep_finds_match() {
        let tmp = std::env::temp_dir().join("dsx_test_grep");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("src.rs"), "fn main() {\n  println!(\"hello\");\n}").unwrap();

        let args = serde_json::json!({"pattern": "println", "path": "."});
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_grep("call_1", &args, &ctx);

        assert!(result.success);
        assert!(result.content.contains("src.rs"));
        assert!(result.content.contains("println"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_grep_no_match() {
        let tmp = std::env::temp_dir().join("dsx_test_grep_none");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("src.rs"), "fn main() {}").unwrap();

        let args = serde_json::json!({"pattern": "missing", "path": "."});
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_grep("call_1", &args, &ctx);

        assert!(result.success);
        assert!(result.content.contains("No matches for pattern"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_propose_patch_success() {
        let tmp = std::env::temp_dir().join("dsx_test_patch_success");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        std::fs::write(tmp.join("a.rs"), "fn main() {\n  let a = 1;\n}").unwrap();

        let args = serde_json::json!({
            "summary": "change let a",
            "changes": [
                {
                    "path": "a.rs",
                    "search": "let a = 1;",
                    "replace": "let a = 2;"
                }
            ]
        });

        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_propose_patch("call_1", &args, &ctx);

        assert!(result.success);
        assert!(result.content.contains("✓ a.rs"));

        let updated = std::fs::read_to_string(tmp.join("a.rs")).unwrap();
        assert!(updated.contains("let a = 2;"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

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

    #[test]
    fn test_tool_risk_mapping() {
        assert_eq!(tool_risk("read_file"), RiskLevel::Read);
        assert_eq!(tool_risk("list_files"), RiskLevel::Read);
        assert_eq!(tool_risk("grep"), RiskLevel::Read);
        assert_eq!(tool_risk("propose_patch"), RiskLevel::Medium);
    }
}
