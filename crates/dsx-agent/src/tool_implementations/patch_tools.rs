//! Patch-oriented built-in tool implementations.

use crate::tool_executor::ToolContext;
use crate::types::ToolResult;
use dsx_core::types::RiskLevel;

pub fn exec_propose_patch(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("no summary");
    let Some(changes) = args.get("changes").and_then(|v| v.as_array()) else {
        return result(
            id,
            "Error: 'changes' array is required".into(),
            false,
            RiskLevel::Medium,
            false,
        );
    };

    let mut results = Vec::new();
    let mut all_succeeded = true;
    let mut denied = false;
    let mut new_contents: Vec<(std::path::PathBuf, String)> = Vec::new();

    for change_val in changes {
        let outcome = stage_change(ctx, change_val, &mut results, &mut new_contents);
        if !outcome.succeeded {
            all_succeeded = false;
        }
        denied |= outcome.denied;
    }

    if all_succeeded {
        for (path, content) in &new_contents {
            if let Err(e) = std::fs::write(path, content) {
                results.push(format!("✗ Write error {}: {e}", path.display()));
                all_succeeded = false;
            }
        }
    }

    result(
        id,
        format!("Patch proposal: {summary}\n\n{}", results.join("\n")),
        all_succeeded,
        if denied {
            RiskLevel::Blocked
        } else {
            RiskLevel::Medium
        },
        denied,
    )
}

struct StageOutcome {
    succeeded: bool,
    denied: bool,
}

fn stage_change(
    ctx: &ToolContext,
    change_val: &serde_json::Value,
    results: &mut Vec<String>,
    new_contents: &mut Vec<(std::path::PathBuf, String)>,
) -> StageOutcome {
    let path = change_val
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let file_change = dsx_patch::FileChange {
        path: path.into(),
        search: change_val
            .get("search")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .into(),
        replace: change_val
            .get("replace")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .into(),
    };

    let full_path = match dsx_fs::resolve_path(&ctx.workspace, path) {
        Ok(path) => path,
        Err(e) => {
            let error = e.to_string();
            if super::is_scope_error_text(&error) {
                results.push(format!("✗ {path}: denied by active scope — {error}"));
                return StageOutcome {
                    succeeded: false,
                    denied: true,
                };
            }
            results.push(format!("✗ {path}: path error — {error}"));
            return StageOutcome {
                succeeded: false,
                denied: false,
            };
        }
    };
    let original = match std::fs::read_to_string(&full_path) {
        Ok(original) => original,
        Err(e) => {
            results.push(format!("✗ {path}: cannot read — {e}"));
            return StageOutcome {
                succeeded: false,
                denied: false,
            };
        }
    };

    match dsx_patch::apply_change(&original, &file_change) {
        dsx_patch::ApplyResult::Applied {
            path,
            tier,
            content,
        } => {
            new_contents.push((full_path, content));
            results.push(format!("✓ {path} (tier {tier})"));
            StageOutcome {
                succeeded: true,
                denied: false,
            }
        }
        dsx_patch::ApplyResult::Failed { path, reason } => {
            results.push(format!("✗ {path}: {reason}"));
            StageOutcome {
                succeeded: false,
                denied: false,
            }
        }
    }
}

fn result(id: &str, content: String, success: bool, risk: RiskLevel, denied: bool) -> ToolResult {
    ToolResult {
        tool_call_id: id.into(),
        name: "propose_patch".into(),
        content,
        success,
        risk,
        denied,
    }
}
