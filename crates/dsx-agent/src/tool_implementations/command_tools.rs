//! Command execution built-in tool implementation.

use crate::tool_executor::ToolContext;
use crate::types::ToolResult;
use dsx_core::types::RiskLevel;
use dsx_permissions::{PermissionAction, classify_command, required_action};

pub async fn exec_run_command(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    if command.is_empty() {
        return result(
            id,
            "Error: command is required".into(),
            false,
            RiskLevel::Medium,
            false,
        );
    }
    if let Err(e) = super::command_scope::validate_command_scope(command, &ctx.workspace) {
        return result(
            id,
            format!("Command denied by active scope: {e}"),
            false,
            RiskLevel::Blocked,
            true,
        );
    }

    let cmd_risk = classify_command(command);
    let action = required_action(cmd_risk, ctx.mode);
    if matches!(action, PermissionAction::Deny) {
        return result(
            id,
            format!(
                "Command denied (risk: {:?}, mode: {:?})",
                cmd_risk, ctx.mode
            ),
            false,
            cmd_risk,
            true,
        );
    }
    if matches!(action, PermissionAction::Ask) && ctx.approval_tx.is_none() {
        return result(
            id,
            format!(
                "Command requires interactive approval (risk: {:?}), but no approval channel is available.",
                cmd_risk
            ),
            false,
            cmd_risk,
            true,
        );
    }

    match dsx_sandbox::run("sh", &["-lc", command], &ctx.workspace, 120).await {
        Ok(output) => result(
            id,
            command_output(&output),
            output.exit_code == Some(0),
            cmd_risk,
            false,
        ),
        Err(e) => result(
            id,
            format!("Failed to execute: {e}"),
            false,
            cmd_risk,
            false,
        ),
    }
}

fn command_output(output: &dsx_sandbox::RunResult) -> String {
    let mut content = format!(
        "Exit code: {}\nDuration: {} ms\n",
        output.exit_code.unwrap_or(-1),
        output.duration_ms
    );
    if !output.stdout.is_empty() {
        content.push_str(&format!("stdout:\n{}", output.stdout));
    }
    if !output.stderr.is_empty() {
        content.push_str(&format!("stderr:\n{}", output.stderr));
    }
    if output.truncated {
        content.push_str("\n... [output truncated]");
    }
    content
}

fn result(id: &str, content: String, success: bool, risk: RiskLevel, denied: bool) -> ToolResult {
    ToolResult {
        tool_call_id: id.into(),
        name: "run_command".into(),
        content,
        success,
        risk,
        denied,
    }
}
