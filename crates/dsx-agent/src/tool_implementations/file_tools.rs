//! File-oriented built-in tool implementations.

use crate::tool_executor::ToolContext;
use crate::types::ToolResult;
use dsx_core::types::RiskLevel;

pub fn exec_read_file(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    match dsx_fs::resolve_path(&ctx.workspace, path) {
        Ok(resolved) => match std::fs::read_to_string(&resolved) {
            Ok(content) => ToolResult {
                tool_call_id: id.into(),
                name: "read_file".into(),
                content: format!(
                    "File: {path}\n\n{}",
                    super::truncate_content(&content, 50_000)
                ),
                success: true,
                risk: RiskLevel::Read,
                denied: false,
            },
            Err(e) => read_error(id, "read_file", format!("Error reading {path}: {e}")),
        },
        Err(e) => read_error(id, "read_file", format!("Path error: {e}")),
    }
}

pub fn exec_list_files(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let subdir = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let target = match dsx_fs::resolve_path(&ctx.workspace, subdir) {
        Ok(path) => path,
        Err(e) => return read_error(id, "list_files", format!("Path error: {e}")),
    };

    match dsx_fs::list_files(&target) {
        Ok(files) => ToolResult {
            tool_call_id: id.into(),
            name: "list_files".into(),
            content: format!("Files in {subdir}:\n{}", files.join("\n")),
            success: true,
            risk: RiskLevel::Read,
            denied: false,
        },
        Err(e) => read_error(id, "list_files", format!("Error listing {subdir}: {e}")),
    }
}

pub fn exec_grep(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    let subdir = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let target = match dsx_fs::resolve_path(&ctx.workspace, subdir) {
        Ok(path) => path,
        Err(e) => return read_error(id, "grep", format!("Path error: {e}")),
    };
    if pattern.is_empty() {
        return read_error(id, "grep", "Error: pattern is required".into());
    }

    let mut matches = Vec::new();
    let walker = ignore::WalkBuilder::new(&target).hidden(false).build();
    for entry in walker.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        collect_matches(ctx, pattern, &mut matches, entry.path());
        if matches.len() >= 100 {
            break;
        }
    }

    ToolResult {
        tool_call_id: id.into(),
        name: "grep".into(),
        content: grep_output(pattern, &matches),
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
        Err(e) => return medium_error(id, "write_file", format!("Path error: {e}")),
    };
    if full_path.exists() && !overwrite {
        return medium_error(
            id,
            "write_file",
            format!("Refusing to overwrite existing file without overwrite=true: {path}"),
        );
    }
    if let Some(parent) = full_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return medium_error(
            id,
            "write_file",
            format!("Failed to create parent directory for {path}: {e}"),
        );
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
        Err(e) => medium_error(id, "write_file", format!("Failed to write {path}: {e}")),
    }
}

fn collect_matches(
    ctx: &ToolContext,
    pattern: &str,
    matches: &mut Vec<String>,
    path: &std::path::Path,
) {
    if let Ok(content) = std::fs::read_to_string(path) {
        for (line_num, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                let rel = path.strip_prefix(&ctx.workspace).unwrap_or(path);
                matches.push(format!("{}:{}: {}", rel.display(), line_num + 1, line));
                if matches.len() >= 100 {
                    break;
                }
            }
        }
    }
}

fn grep_output(pattern: &str, matches: &[String]) -> String {
    if matches.is_empty() {
        return format!("No matches for pattern: {pattern}");
    }
    let count = matches.len();
    let mut out = format!("{count} match(es) for '{pattern}':\n{}", matches.join("\n"));
    if count >= 100 {
        out.push_str("\n... (results truncated at 100)");
    }
    out
}

fn read_error(id: &str, name: &str, content: String) -> ToolResult {
    ToolResult {
        tool_call_id: id.into(),
        name: name.into(),
        content,
        success: false,
        risk: RiskLevel::Read,
        denied: false,
    }
}

fn medium_error(id: &str, name: &str, content: String) -> ToolResult {
    ToolResult {
        tool_call_id: id.into(),
        name: name.into(),
        content,
        success: false,
        risk: RiskLevel::Medium,
        denied: false,
    }
}
