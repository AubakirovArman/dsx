//! DSX Agent — concrete tool execution implementations.

use dsx_core::types::RiskLevel;
use dsx_permissions::{classify_command, required_action, PermissionAction};
use crate::types::ToolResult;
use crate::tool_executor::ToolContext;

pub fn exec_read_file(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    match dsx_fs::resolve_path(&ctx.workspace, path) {
        Ok(resolved) => match std::fs::read_to_string(&resolved) {
            Ok(content) => {
                let truncated = truncate_content(&content, 50_000);
                ToolResult {
                    tool_call_id: id.into(),
                    name: "read_file".into(),
                    content: format!("File: {path}\n\n{truncated}"),
                    success: true,
                    risk: RiskLevel::Read,
                    denied: false,
                }
            }
            Err(e) => ToolResult {
                tool_call_id: id.into(),
                name: "read_file".into(),
                content: format!("Error reading {path}: {e}"),
                success: false,
                risk: RiskLevel::Read,
                denied: false,
            },
        },
        Err(e) => ToolResult {
            tool_call_id: id.into(),
            name: "read_file".into(),
            content: format!("Path error: {e}"),
            success: false,
            risk: RiskLevel::Read,
            denied: false,
        },
    }
}

pub fn exec_list_files(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let subdir = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let target = ctx.workspace.join(subdir);
    match dsx_fs::list_files(&target) {
        Ok(files) => {
            let listing = files.join("\n");
            ToolResult {
                tool_call_id: id.into(),
                name: "list_files".into(),
                content: format!("Files in {subdir}:\n{listing}"),
                success: true,
                risk: RiskLevel::Read,
                denied: false,
            }
        }
        Err(e) => ToolResult {
            tool_call_id: id.into(),
            name: "list_files".into(),
            content: format!("Error listing {subdir}: {e}"),
            success: false,
            risk: RiskLevel::Read,
            denied: false,
        },
    }
}

pub fn exec_grep(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let subdir = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let target = ctx.workspace.join(subdir);

    if pattern.is_empty() {
        return ToolResult {
            tool_call_id: id.into(),
            name: "grep".into(),
            content: "Error: pattern is required".into(),
            success: false,
            risk: RiskLevel::Read,
            denied: false,
        };
    }

    let mut matches = Vec::new();
    let walker = ignore::WalkBuilder::new(&target).hidden(false).build();

    for entry in walker.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for (line_num, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    let rel = entry.path().strip_prefix(&ctx.workspace).unwrap_or(entry.path());
                    matches.push(format!("{}:{}: {}", rel.display(), line_num + 1, line));
                    if matches.len() >= 100 {
                        break;
                    }
                }
            }
        }
        if matches.len() >= 100 {
            break;
        }
    }

    let output = if matches.is_empty() {
        format!("No matches for pattern: {pattern}")
    } else {
        let count = matches.len();
        let mut out = format!("{count} match(es) for '{pattern}':\n");
        out.push_str(&matches.join("\n"));
        if count >= 100 {
            out.push_str("\n... (results truncated at 100)");
        }
        out
    };

    ToolResult {
        tool_call_id: id.into(),
        name: "grep".into(),
        content: output,
        success: true,
        risk: RiskLevel::Read,
        denied: false,
    }
}

pub fn exec_propose_patch(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let summary = args.get("summary").and_then(|v| v.as_str()).unwrap_or("no summary");
    let changes = args.get("changes").and_then(|v| v.as_array());

    let Some(changes) = changes else {
        return ToolResult {
            tool_call_id: id.into(),
            name: "propose_patch".into(),
            content: "Error: 'changes' array is required".into(),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        };
    };

    let mut results = Vec::new();
    let mut all_succeeded = true;
    let mut new_contents: Vec<(std::path::PathBuf, String)> = Vec::new();

    for change_val in changes {
        let path = change_val.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let search = change_val.get("search").and_then(|v| v.as_str()).unwrap_or("");
        let replace = change_val.get("replace").and_then(|v| v.as_str()).unwrap_or("");

        let file_change = dsx_patch::FileChange {
            path: path.into(),
            search: search.into(),
            replace: replace.into(),
        };

        let full_path = ctx.workspace.join(path);
        match std::fs::read_to_string(&full_path) {
            Ok(original) => {
                let result = dsx_patch::apply_change(&original, &file_change);
                match result {
                    dsx_patch::ApplyResult::Applied { path: p, tier, content: patched } => {
                        // Collect for write-back
                        new_contents.push((full_path.clone(), patched));
                        results.push(format!("✓ {p} (tier {tier})"));
                    }
                    dsx_patch::ApplyResult::Failed { path: p, reason } => {
                        results.push(format!("✗ {p}: {reason}"));
                        all_succeeded = false;
                    }
                }
            }
            Err(e) => {
                results.push(format!("✗ {path}: cannot read — {e}"));
                all_succeeded = false;
            }
        }
    }

    // Write patched files back if all succeeded
    if all_succeeded {
        for (path, content) in &new_contents {
            if let Err(e) = std::fs::write(path, content) {
                results.push(format!("✗ Write error {}: {e}", path.display()));
                all_succeeded = false;
            }
        }
    }

    let output = format!(
        "Patch proposal: {summary}\n\n{}",
        results.join("\n")
    );

    ToolResult {
        tool_call_id: id.into(),
        name: "propose_patch".into(),
        content: output,
        success: all_succeeded,
        risk: RiskLevel::Medium,
        denied: false,
    }
}

pub fn exec_run_command(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    if command.is_empty() {
        return ToolResult {
            tool_call_id: id.into(),
            name: "run_command".into(),
            content: "Error: command is required".into(),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        };
    }

    // Re-classify command risk
    let cmd_risk = classify_command(command);
    let action = required_action(cmd_risk, ctx.mode);
    if matches!(action, PermissionAction::Deny) {
        return ToolResult {
            tool_call_id: id.into(),
            name: "run_command".into(),
            content: format!("Command denied (risk: {:?}, mode: {:?})", cmd_risk, ctx.mode),
            success: false,
            risk: cmd_risk,
            denied: true,
        };
    }

    // Parse command into parts (simple whitespace split)
    let parts: Vec<&str> = command.split_whitespace().collect();
    let (cmd, cmd_args) = if parts.is_empty() {
        ("", &[][..])
    } else {
        (parts[0], &parts[1..])
    };

    let result = std::process::Command::new(cmd)
        .args(cmd_args)
        .current_dir(&ctx.workspace)
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut content = format!("Exit code: {}\n", output.status.code().unwrap_or(-1));
            if !stdout.is_empty() {
                content.push_str(&format!("stdout:\n{stdout}"));
            }
            if !stderr.is_empty() {
                content.push_str(&format!("stderr:\n{stderr}"));
            }
            ToolResult {
                tool_call_id: id.into(),
                name: "run_command".into(),
                content,
                success: output.status.success(),
                risk: cmd_risk,
                denied: false,
            }
        }
        Err(e) => ToolResult {
            tool_call_id: id.into(),
            name: "run_command".into(),
            content: format!("Failed to execute: {e}"),
            success: false,
            risk: cmd_risk,
            denied: false,
        },
    }
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        content.to_string()
    } else {
        let mut truncated = content[..max_chars].to_string();
        truncated.push_str(&format!(
            "\n\n... [truncated at {max_chars} chars, total {} chars]",
            content.len()
        ));
        truncated
    }
}
