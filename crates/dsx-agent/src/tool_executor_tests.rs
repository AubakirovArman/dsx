//! DSX Agent — unit tests for tool_executor.

#[cfg(test)]
mod tests {
    use crate::tool_executor::{ToolContext, execute, tool_risk};
    use dsx_core::types::{PermissionMode, RiskLevel};
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
        assert!(
            result.content.contains("error")
                || result.content.contains("Path error")
                || result.content.contains("Error reading")
        );

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

    #[test]
    fn test_write_file_creates_nested_file() {
        let tmp = std::env::temp_dir().join("dsx_test_write_file");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let args = serde_json::json!({
            "path": "app/src/main.rs",
            "content": "fn main() {}\n"
        });
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_write_file("call_1", &args, &ctx);

        assert!(result.success);
        assert_eq!(
            std::fs::read_to_string(tmp.join("app/src/main.rs")).unwrap(),
            "fn main() {}\n"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_write_file_blocks_path_traversal() {
        let tmp = std::env::temp_dir().join("dsx_test_write_traversal");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let args = serde_json::json!({
            "path": "../outside.txt",
            "content": "nope"
        });
        let ctx = test_ctx(&tmp);
        let result = crate::tool_implementations::exec_write_file("call_1", &args, &ctx);

        assert!(!result.success);
        assert!(result.content.contains("path traversal blocked"));

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

    #[test]
    fn test_tool_risk_mapping() {
        assert_eq!(tool_risk("read_file"), RiskLevel::Read);
        assert_eq!(tool_risk("list_files"), RiskLevel::Read);
        assert_eq!(tool_risk("grep"), RiskLevel::Read);
        assert_eq!(tool_risk("mcp_list_tools"), RiskLevel::Read);
        assert_eq!(tool_risk("write_file"), RiskLevel::Medium);
        assert_eq!(tool_risk("propose_patch"), RiskLevel::Medium);
        assert_eq!(tool_risk("mcp_call"), RiskLevel::Medium);
    }

    #[tokio::test]
    async fn test_mcp_list_tools_from_project_config() {
        let tmp = std::env::temp_dir().join("dsx_test_agent_mcp_list");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".deepseek-code")).unwrap();

        std::fs::write(
            tmp.join(".deepseek-code").join("mcp.json"),
            r#"{
                "servers": [
                    {
                        "name": "local",
                        "command": "python3",
                        "args": [
                            "-c",
                            "import json, sys\n\ndef read_frame():\n    header = b''\n    while not header.endswith(b'\\r\\n\\r\\n'):\n        b = sys.stdin.buffer.read(1)\n        if not b:\n            return None\n        header += b\n    length = 0\n    for line in header.decode().splitlines():\n        if line.lower().startswith('content-length:'):\n            length = int(line.split(':', 1)[1].strip())\n    return json.loads(sys.stdin.buffer.read(length))\n\ndef write_frame(obj):\n    data = json.dumps(obj).encode()\n    sys.stdout.buffer.write(f'Content-Length: {len(data)}\\r\\n\\r\\n'.encode() + data)\n    sys.stdout.buffer.flush()\n\nwhile True:\n    msg = read_frame()\n    if msg is None:\n        break\n    method = msg.get('method')\n    if method == 'initialize':\n        write_frame({'jsonrpc':'2.0','id':msg['id'],'result':{'protocolVersion':'2024-11-05','capabilities':{},'serverInfo':{'name':'fake'}}})\n    elif method == 'notifications/initialized':\n        pass\n    elif method == 'tools/list':\n        write_frame({'jsonrpc':'2.0','id':msg['id'],'result':{'tools':[{'name':'echo','description':'Echo args','inputSchema':{'type':'object','properties':{'value':{'type':'number'}}}}]}})\n"
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        let call = ToolCallReady {
            id: "call_1".into(),
            name: "mcp_list_tools".into(),
            arguments: r#"{"server":"local"}"#.into(),
        };
        let ctx = ToolContext {
            workspace: tmp.clone(),
            mode: PermissionMode::Ask,
            approval_tx: None,
        };

        let result = execute(&call, &ctx).await;
        assert!(result.success, "{}", result.content);
        assert!(result.content.contains("Server: local"));
        assert!(result.content.contains("echo"));
        assert!(result.content.contains("inputSchema"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
