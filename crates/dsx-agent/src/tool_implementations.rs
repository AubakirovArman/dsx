//! DSX Agent — concrete tool execution implementations.

use crate::tool_executor::ToolContext;
use crate::types::ToolResult;
use dsx_core::types::RiskLevel;
use dsx_permissions::{PermissionAction, classify_command, required_action};

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
    let target = match dsx_fs::resolve_path(&ctx.workspace, subdir) {
        Ok(path) => path,
        Err(e) => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "list_files".into(),
                content: format!("Path error: {e}"),
                success: false,
                risk: RiskLevel::Read,
                denied: false,
            };
        }
    };
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
    let target = match dsx_fs::resolve_path(&ctx.workspace, subdir) {
        Ok(path) => path,
        Err(e) => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "grep".into(),
                content: format!("Path error: {e}"),
                success: false,
                risk: RiskLevel::Read,
                denied: false,
            };
        }
    };

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
                    let rel = entry
                        .path()
                        .strip_prefix(&ctx.workspace)
                        .unwrap_or(entry.path());
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

pub fn exec_write_file(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let overwrite = args
        .get("overwrite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let full_path = match dsx_fs::resolve_path_allow_missing(&ctx.workspace, path) {
        Ok(path) => path,
        Err(e) => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "write_file".into(),
                content: format!("Path error: {e}"),
                success: false,
                risk: RiskLevel::Medium,
                denied: false,
            };
        }
    };

    if full_path.exists() && !overwrite {
        return ToolResult {
            tool_call_id: id.into(),
            name: "write_file".into(),
            content: format!("Refusing to overwrite existing file without overwrite=true: {path}"),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        };
    }

    if let Some(parent) = full_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return ToolResult {
            tool_call_id: id.into(),
            name: "write_file".into(),
            content: format!("Failed to create parent directory for {path}: {e}"),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        };
    }

    match std::fs::write(&full_path, content) {
        Ok(()) => ToolResult {
            tool_call_id: id.into(),
            name: "write_file".into(),
            content: format!("Wrote {} bytes to {path}", content.len()),
            success: true,
            risk: RiskLevel::Medium,
            denied: false,
        },
        Err(e) => ToolResult {
            tool_call_id: id.into(),
            name: "write_file".into(),
            content: format!("Failed to write {path}: {e}"),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        },
    }
}

pub fn exec_propose_patch(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("no summary");
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
        let path = change_val
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let search = change_val
            .get("search")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let replace = change_val
            .get("replace")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let file_change = dsx_patch::FileChange {
            path: path.into(),
            search: search.into(),
            replace: replace.into(),
        };

        let full_path = match dsx_fs::resolve_path(&ctx.workspace, path) {
            Ok(path) => path,
            Err(e) => {
                results.push(format!("✗ {path}: path error — {e}"));
                all_succeeded = false;
                continue;
            }
        };
        match std::fs::read_to_string(&full_path) {
            Ok(original) => {
                let result = dsx_patch::apply_change(&original, &file_change);
                match result {
                    dsx_patch::ApplyResult::Applied {
                        path: p,
                        tier,
                        content: patched,
                    } => {
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

    let output = format!("Patch proposal: {summary}\n\n{}", results.join("\n"));

    ToolResult {
        tool_call_id: id.into(),
        name: "propose_patch".into(),
        content: output,
        success: all_succeeded,
        risk: RiskLevel::Medium,
        denied: false,
    }
}

pub async fn exec_run_command(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
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
            content: format!(
                "Command denied (risk: {:?}, mode: {:?})",
                cmd_risk, ctx.mode
            ),
            success: false,
            risk: cmd_risk,
            denied: true,
        };
    }
    if matches!(action, PermissionAction::Ask) && ctx.approval_tx.is_none() {
        return ToolResult {
            tool_call_id: id.into(),
            name: "run_command".into(),
            content: format!(
                "Command requires interactive approval (risk: {:?}), but no approval channel is available.",
                cmd_risk
            ),
            success: false,
            risk: cmd_risk,
            denied: true,
        };
    }

    match dsx_sandbox::run("sh", &["-lc", command], &ctx.workspace, 120).await {
        Ok(output) => {
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
            ToolResult {
                tool_call_id: id.into(),
                name: "run_command".into(),
                content,
                success: output.exit_code == Some(0),
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

pub async fn exec_mcp_list_tools(
    id: &str,
    args: &serde_json::Value,
    ctx: &ToolContext,
) -> ToolResult {
    let requested_server = args.get("server").and_then(|v| v.as_str());
    let config = match dsx_mcp::load_config(&ctx.workspace) {
        Ok(config) => config,
        Err(e) => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "mcp_list_tools".into(),
                content: format!("Failed to load MCP config: {e}"),
                success: false,
                risk: RiskLevel::Read,
                denied: false,
            };
        }
    };

    let servers = select_mcp_servers(&config, requested_server);
    if servers.is_empty() {
        let content = if let Some(server) = requested_server {
            format!("No enabled MCP server named '{server}' is configured.")
        } else {
            "No enabled MCP servers configured.".into()
        };
        return ToolResult {
            tool_call_id: id.into(),
            name: "mcp_list_tools".into(),
            content,
            success: true,
            risk: RiskLevel::Read,
            denied: false,
        };
    }

    let mut lines = Vec::new();
    let mut success = true;
    for server in servers {
        match dsx_mcp::list_server_tools(server).await {
            Ok(tools) => {
                lines.push(format!("Server: {}", server.name));
                if tools.is_empty() {
                    lines.push("  (no tools)".into());
                    continue;
                }
                for tool in tools {
                    let description = tool.description.unwrap_or_default();
                    let schema =
                        serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".into());
                    lines.push(format!("  - {}: {}", tool.name, description));
                    lines.push(format!("    inputSchema: {schema}"));
                }
            }
            Err(e) => {
                success = false;
                lines.push(format!("Server: {}", server.name));
                lines.push(format!("  error: {e}"));
            }
        }
    }

    ToolResult {
        tool_call_id: id.into(),
        name: "mcp_list_tools".into(),
        content: truncate_content(&lines.join("\n"), 50_000),
        success,
        risk: RiskLevel::Read,
        denied: false,
    }
}

pub async fn exec_mcp_call(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let server_name = match args.get("server").and_then(|v| v.as_str()) {
        Some(server) if !server.trim().is_empty() => server,
        _ => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "mcp_call".into(),
                content: "Error: server is required".into(),
                success: false,
                risk: RiskLevel::Medium,
                denied: false,
            };
        }
    };
    let tool = match args.get("tool").and_then(|v| v.as_str()) {
        Some(tool) if !tool.trim().is_empty() => tool,
        _ => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "mcp_call".into(),
                content: "Error: tool is required".into(),
                success: false,
                risk: RiskLevel::Medium,
                denied: false,
            };
        }
    };
    let arguments = args
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !arguments.is_object() {
        return ToolResult {
            tool_call_id: id.into(),
            name: "mcp_call".into(),
            content: "Error: arguments must be a JSON object".into(),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        };
    }

    let config = match dsx_mcp::load_config(&ctx.workspace) {
        Ok(config) => config,
        Err(e) => {
            return ToolResult {
                tool_call_id: id.into(),
                name: "mcp_call".into(),
                content: format!("Failed to load MCP config: {e}"),
                success: false,
                risk: RiskLevel::Medium,
                denied: false,
            };
        }
    };

    let Some(server) = dsx_mcp::enabled_servers(&config).find(|server| server.name == server_name)
    else {
        return ToolResult {
            tool_call_id: id.into(),
            name: "mcp_call".into(),
            content: format!("No enabled MCP server named '{server_name}' is configured."),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        };
    };

    match dsx_mcp::call_server_tool(server, tool, arguments).await {
        Ok(result) => {
            let success = !result.is_error;
            let content = match serde_json::to_string_pretty(&result) {
                Ok(text) => truncate_content(&text, 50_000),
                Err(e) => format!("Failed to serialize MCP tool result: {e}"),
            };
            ToolResult {
                tool_call_id: id.into(),
                name: "mcp_call".into(),
                content,
                success,
                risk: RiskLevel::Medium,
                denied: false,
            }
        }
        Err(e) => ToolResult {
            tool_call_id: id.into(),
            name: "mcp_call".into(),
            content: format!("MCP tool call failed: {e}"),
            success: false,
            risk: RiskLevel::Medium,
            denied: false,
        },
    }
}

fn select_mcp_servers<'a>(
    config: &'a dsx_mcp::McpConfig,
    requested_server: Option<&str>,
) -> Vec<&'a dsx_mcp::McpServerConfig> {
    dsx_mcp::enabled_servers(config)
        .filter(|server| requested_server.is_none_or(|name| server.name == name))
        .collect()
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
