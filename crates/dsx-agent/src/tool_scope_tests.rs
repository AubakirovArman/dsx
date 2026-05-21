//! Regression tests for hard active-scope tool boundaries.

#[cfg(test)]
mod tests {
    use crate::tool_executor::ToolContext;
    use crate::tool_implementations;
    use dsx_core::types::{PermissionMode, RiskLevel};
    use std::path::PathBuf;

    #[test]
    fn file_tools_deny_parent_scope_from_child_workspace() {
        let fixture = ScopeFixture::new("dsx_tool_scope_files");
        let ctx = fixture.ctx();

        let read = tool_implementations::exec_read_file(
            "read",
            &serde_json::json!({"path": "../parent.txt"}),
            &ctx,
        );
        assert_scope_denied(&read);

        let list =
            tool_implementations::exec_list_files("list", &serde_json::json!({"path": ".."}), &ctx);
        assert_scope_denied(&list);

        let grep = tool_implementations::exec_grep(
            "grep",
            &serde_json::json!({"path": "..", "pattern": "parent"}),
            &ctx,
        );
        assert_scope_denied(&grep);

        let write = tool_implementations::exec_write_file(
            "write",
            &serde_json::json!({"path": "../new.txt", "content": "nope"}),
            &ctx,
        );
        assert_scope_denied(&write);
        assert!(!fixture.launch.join("new.txt").exists());
    }

    #[test]
    fn patch_tool_denies_parent_scope_from_child_workspace() {
        let fixture = ScopeFixture::new("dsx_tool_scope_patch");
        let result = tool_implementations::exec_propose_patch(
            "patch",
            &serde_json::json!({
                "summary": "escape",
                "changes": [{
                    "path": "../parent.txt",
                    "search": "parent",
                    "replace": "changed"
                }]
            }),
            &fixture.ctx(),
        );

        assert_scope_denied(&result);
        assert_eq!(
            std::fs::read_to_string(fixture.launch.join("parent.txt")).unwrap(),
            "parent\n"
        );
    }

    #[tokio::test]
    async fn command_tool_denies_parent_scope_from_child_workspace() {
        let fixture = ScopeFixture::new("dsx_tool_scope_command");
        let result = tool_implementations::exec_run_command(
            "cmd",
            &serde_json::json!({"command": "find .. -maxdepth 1 -type f"}),
            &fixture.ctx(),
        )
        .await;

        assert_scope_denied(&result);
    }

    fn assert_scope_denied(result: &crate::ToolResult) {
        assert!(!result.success, "{}", result.content);
        assert!(result.denied, "{}", result.content);
        assert_eq!(result.risk, RiskLevel::Blocked);
        assert!(
            result.content.contains("active scope"),
            "{}",
            result.content
        );
    }

    struct ScopeFixture {
        launch: PathBuf,
        active: PathBuf,
    }

    impl ScopeFixture {
        fn new(name: &str) -> Self {
            let launch = temp_root(name);
            let _ = std::fs::remove_dir_all(&launch);
            let active = launch.join("1234");
            std::fs::create_dir_all(active.join("src")).unwrap();
            std::fs::write(launch.join("parent.txt"), "parent\n").unwrap();
            std::fs::write(active.join("src/main.rs"), "fn main() {}\n").unwrap();
            Self { launch, active }
        }

        fn ctx(&self) -> ToolContext {
            ToolContext {
                workspace: self.active.clone(),
                mode: PermissionMode::Yolo,
                approval_tx: None,
            }
        }
    }

    impl Drop for ScopeFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.launch);
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
