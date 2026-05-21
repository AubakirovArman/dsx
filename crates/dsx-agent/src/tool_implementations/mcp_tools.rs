//! MCP built-in tool implementations.

use crate::tool_executor::ToolContext;
use crate::types::ToolResult;
use dsx_core::types::RiskLevel;

pub async fn exec_mcp_list_tools(
    id: &str,
    args: &serde_json::Value,
    ctx: &ToolContext,
) -> ToolResult {
    let requested_server = args.get("server").and_then(|v| v.as_str());
    let config = match dsx_mcp::load_config(&ctx.workspace) {
        Ok(config) => config,
        Err(e) => {
            return result(
                id,
                "mcp_list_tools",
                format!("Failed to load MCP config: {e}"),
                false,
                RiskLevel::Read,
            );
        }
    };

    let servers = select_mcp_servers(&config, requested_server);
    if servers.is_empty() {
        let content = if let Some(server) = requested_server {
            format!("No enabled MCP server named '{server}' is configured.")
        } else {
            "No enabled MCP servers configured.".into()
        };
        return result(id, "mcp_list_tools", content, true, RiskLevel::Read);
    }

    let mut lines = Vec::new();
    let mut success = true;
    for server in servers {
        match dsx_mcp::list_server_tools(server).await {
            Ok(tools) => push_server_tools(&mut lines, server, tools),
            Err(e) => {
                success = false;
                lines.push(format!("Server: {}", server.name));
                lines.push(format!("  error: {e}"));
            }
        }
    }

    result(
        id,
        "mcp_list_tools",
        super::truncate_content(&lines.join("\n"), 50_000),
        success,
        RiskLevel::Read,
    )
}

pub async fn exec_mcp_call(id: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolResult {
    let Some(server_name) = required_str(args, "server") else {
        return result(
            id,
            "mcp_call",
            "Error: server is required".into(),
            false,
            RiskLevel::Medium,
        );
    };
    let Some(tool) = required_str(args, "tool") else {
        return result(
            id,
            "mcp_call",
            "Error: tool is required".into(),
            false,
            RiskLevel::Medium,
        );
    };
    let arguments = args
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !arguments.is_object() {
        return result(
            id,
            "mcp_call",
            "Error: arguments must be a JSON object".into(),
            false,
            RiskLevel::Medium,
        );
    }

    let config = match dsx_mcp::load_config(&ctx.workspace) {
        Ok(config) => config,
        Err(e) => {
            return result(
                id,
                "mcp_call",
                format!("Failed to load MCP config: {e}"),
                false,
                RiskLevel::Medium,
            );
        }
    };
    let Some(server) = dsx_mcp::enabled_servers(&config).find(|server| server.name == server_name)
    else {
        return result(
            id,
            "mcp_call",
            format!("No enabled MCP server named '{server_name}' is configured."),
            false,
            RiskLevel::Medium,
        );
    };

    match dsx_mcp::call_server_tool(server, tool, arguments).await {
        Ok(tool_result) => {
            let success = !tool_result.is_error;
            let content = serde_json::to_string_pretty(&tool_result)
                .map(|text| super::truncate_content(&text, 50_000))
                .unwrap_or_else(|e| format!("Failed to serialize MCP tool result: {e}"));
            result(id, "mcp_call", content, success, RiskLevel::Medium)
        }
        Err(e) => result(
            id,
            "mcp_call",
            format!("MCP tool call failed: {e}"),
            false,
            RiskLevel::Medium,
        ),
    }
}

fn push_server_tools(
    lines: &mut Vec<String>,
    server: &dsx_mcp::McpServerConfig,
    tools: Vec<dsx_mcp::McpTool>,
) {
    lines.push(format!("Server: {}", server.name));
    if tools.is_empty() {
        lines.push("  (no tools)".into());
        return;
    }
    for tool in tools {
        let description = tool.description.unwrap_or_default();
        let schema = serde_json::to_string(&tool.input_schema).unwrap_or_else(|_| "{}".into());
        lines.push(format!("  - {}: {}", tool.name, description));
        lines.push(format!("    inputSchema: {schema}"));
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

fn required_str<'a>(args: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|value| !value.trim().is_empty())
}

fn result(id: &str, name: &str, content: String, success: bool, risk: RiskLevel) -> ToolResult {
    ToolResult {
        tool_call_id: id.into(),
        name: name.into(),
        content,
        success,
        risk,
        denied: false,
    }
}
