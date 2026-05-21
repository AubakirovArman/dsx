//! DSX Tools — tool definitions and registry for the agent loop.

use serde::{Deserialize, Serialize};
use dsx_core::types::RiskLevel;

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
                description: "Read a file from the workspace.".into(),
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
                description: "List files in a directory.".into(),
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
                description: "Search for a pattern in workspace files.".into(),
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
                description: "Run a shell command in the workspace (requires approval).".into(),
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
                name: "propose_patch".into(),
                description: "Propose a code change as a SEARCH/REPLACE patch.".into(),
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
        ]
    }
}
