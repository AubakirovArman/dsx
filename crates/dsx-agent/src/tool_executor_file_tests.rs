use super::test_ctx;
use crate::tool_executor::tool_risk;
use dsx_core::types::RiskLevel;

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
