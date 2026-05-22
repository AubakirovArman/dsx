//! DSX Agent — unit tests for tool_executor.

#[cfg(test)]
pub(crate) fn test_ctx(tmp: &std::path::Path) -> crate::tool_executor::ToolContext {
    crate::tool_executor::ToolContext {
        workspace: tmp.to_path_buf(),
        mode: dsx_core::types::PermissionMode::Ask,
        approval_tx: None,
    }
}

#[cfg(test)]
#[path = "tool_executor_file_tests.rs"]
mod file_tests;

#[cfg(test)]
#[path = "tool_executor_permission_tests.rs"]
mod permission_tests;

#[cfg(test)]
#[path = "tool_executor_mcp_tests.rs"]
mod mcp_tests;
