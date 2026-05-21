//! DSX Tool Executor — orchestrate the safe and authorized execution of tools.

use crate::types::ToolResult;
use dsx_core::types::{PermissionMode, RiskLevel};
use dsx_permissions::{PermissionAction, classify_command, required_action};
use dsx_provider::streaming::ToolCallReady;

/// Context for tool execution.
pub struct ToolContext {
    /// Workspace root path.
    pub workspace: std::path::PathBuf,
    /// Current permission mode.
    pub mode: PermissionMode,
    /// Channel for interactive approvals.
    pub approval_tx: Option<tokio::sync::mpsc::UnboundedSender<super::ApprovalRequest>>,
}

/// Execute a single tool call from the agent.
pub async fn execute(call: &ToolCallReady, ctx: &ToolContext) -> ToolResult {
    let args = match serde_json::from_str::<serde_json::Value>(&call.arguments) {
        Ok(v) => v,
        Err(e) => {
            return ToolResult {
                tool_call_id: call.id.clone(),
                name: call.name.clone(),
                content: format!("Error parsing arguments as JSON: {e}"),
                success: false,
                risk: RiskLevel::Read,
                denied: false,
            };
        }
    };

    // Determine risk and check permissions
    let risk = tool_risk(&call.name);
    let mut action = required_action(risk, ctx.mode);

    // Re-classify command risk if running a command
    let final_risk = if call.name == "run_command" {
        let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let r = classify_command(cmd);
        action = required_action(r, ctx.mode);
        r
    } else {
        risk
    };

    if matches!(action, PermissionAction::Deny) {
        return ToolResult {
            tool_call_id: call.id.clone(),
            name: call.name.clone(),
            content: format!(
                "Permission denied: tool '{}' (risk level {:?}) is blocked in mode {:?}",
                call.name, final_risk, ctx.mode
            ),
            success: false,
            risk: final_risk,
            denied: true,
        };
    }

    if matches!(action, PermissionAction::Ask) {
        let Some(ref approval_tx) = ctx.approval_tx else {
            return ToolResult {
                tool_call_id: call.id.clone(),
                name: call.name.clone(),
                content: format!(
                    "Permission requires interactive approval for '{}' (risk level {:?}), but no approval channel is available. Re-run in TUI or use an explicit non-interactive mode.",
                    call.name, final_risk
                ),
                success: false,
                risk: final_risk,
                denied: true,
            };
        };

        let (tx_reply, rx_reply) = tokio::sync::oneshot::channel();
        let req = super::ApprovalRequest {
            tool_name: call.name.clone(),
            arguments: call.arguments.clone(),
            tx: tx_reply,
        };

        if approval_tx.send(req).is_ok() {
            match rx_reply.await {
                Ok(true) => {}
                _ => {
                    return ToolResult {
                        tool_call_id: call.id.clone(),
                        name: call.name.clone(),
                        content: "Tool execution denied by user.".into(),
                        success: false,
                        risk: final_risk,
                        denied: true,
                    };
                }
            }
        } else {
            return ToolResult {
                tool_call_id: call.id.clone(),
                name: call.name.clone(),
                content: "Approval channel closed, tool denied.".into(),
                success: false,
                risk: final_risk,
                denied: true,
            };
        }
    }

    // Git checkpoint before any write operation
    if final_risk >= RiskLevel::Medium {
        let label = format!("pre-{}: {}", call.name, &call.id[..8.min(call.id.len())]);
        if let Err(e) = dsx_git::checkpoint(&label, &ctx.workspace) {
            tracing::warn!("Git checkpoint failed (non-fatal): {e}");
        }
    }

    match call.name.as_str() {
        "read_file" => crate::tool_implementations::exec_read_file(&call.id, &args, ctx),
        "list_files" => crate::tool_implementations::exec_list_files(&call.id, &args, ctx),
        "grep" => crate::tool_implementations::exec_grep(&call.id, &args, ctx),
        "write_file" => crate::tool_implementations::exec_write_file(&call.id, &args, ctx),
        "propose_patch" => crate::tool_implementations::exec_propose_patch(&call.id, &args, ctx),
        "mcp_list_tools" => {
            crate::tool_implementations::exec_mcp_list_tools(&call.id, &args, ctx).await
        }
        "mcp_call" => crate::tool_implementations::exec_mcp_call(&call.id, &args, ctx).await,
        "run_command" => crate::tool_implementations::exec_run_command(&call.id, &args, ctx).await,
        name => ToolResult {
            tool_call_id: call.id.clone(),
            name: name.into(),
            content: format!("Unknown tool: {name}"),
            success: false,
            risk: RiskLevel::Read,
            denied: false,
        },
    }
}

/// Map tool name to risk level.
pub fn tool_risk(name: &str) -> RiskLevel {
    match name {
        "read_file" | "list_files" | "grep" | "mcp_list_tools" => RiskLevel::Read,
        "write_file" | "propose_patch" | "mcp_call" => RiskLevel::Medium,
        "run_command" => RiskLevel::Medium, // re-classified by command content
        _ => RiskLevel::Medium,
    }
}
