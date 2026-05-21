//! DSX Tools — tool definitions and registry for the agent loop.

use dsx_core::types::RiskLevel;
use serde::{Deserialize, Serialize};

/// A tool that the agent can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub risk: RiskLevel,
}

/// Built-in tool definitions.
pub struct ToolRegistry;

impl ToolRegistry {
    pub fn builtin_specs() -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "read_file".into(),
                description: "Read a file from the active task scope.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative file path"}
                    },
                    "required": ["path"]
                }),
                risk: RiskLevel::Read,
            },
            ToolSpec {
                name: "list_files".into(),
                description: "List files in a directory inside the active task scope.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative directory path"}
                    }
                }),
                risk: RiskLevel::Read,
            },
            ToolSpec {
                name: "grep".into(),
                description: "Search for a pattern inside the active task scope.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string"},
                        "path": {"type": "string"}
                    },
                    "required": ["pattern"]
                }),
                risk: RiskLevel::Read,
            },
            ToolSpec {
                name: "run_command".into(),
                description: "Run a shell command in the active task scope (requires approval).".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Shell command to execute"}
                    },
                    "required": ["command"]
                }),
                risk: RiskLevel::Medium,
            },
            ToolSpec {
                name: "write_file".into(),
                description: "Create or overwrite a UTF-8 text file in the active task scope. Creates parent directories when needed.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative file path"},
                        "content": {"type": "string", "description": "Complete file contents to write"},
                        "overwrite": {"type": "boolean", "description": "Allow overwriting an existing file. Defaults to false."}
                    },
                    "required": ["path", "content"]
                }),
                risk: RiskLevel::Medium,
            },
            ToolSpec {
                name: "propose_patch".into(),
                description: "Propose a code change inside the active task scope as a SEARCH/REPLACE patch.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "summary": {"type": "string"},
                        "changes": {"type": "array", "items": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"},
                                "search": {"type": "string"},
                                "replace": {"type": "string"}
                            },
                            "required": ["path", "search", "replace"]
                        }}
                    },
                    "required": ["summary", "changes"]
                }),
                risk: RiskLevel::Medium,
            },
            ToolSpec {
                name: "mcp_list_tools".into(),
                description: "List tools exposed by configured MCP servers. Use before mcp_call to inspect names and input schemas.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "server": {"type": "string", "description": "Optional configured MCP server name"}
                    }
                }),
                risk: RiskLevel::Read,
            },
            ToolSpec {
                name: "mcp_call".into(),
                description: "Call a tool on a configured MCP server with a JSON object of arguments.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "server": {"type": "string", "description": "Configured MCP server name"},
                        "tool": {"type": "string", "description": "Tool name exposed by the MCP server"},
                        "arguments": {"type": "object", "description": "Tool arguments matching the MCP input schema"}
                    },
                    "required": ["server", "tool", "arguments"]
                }),
                risk: RiskLevel::Medium,
            },
        ]
    }
}
