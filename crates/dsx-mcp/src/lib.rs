//! DSX MCP — Model Context Protocol client.
//!
//! MVP: stdio transport for connecting to MCP servers.
//! v1: HTTP/SSE transport, server-side MCP host.

pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
}

pub struct McpServerConfig {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub enabled: bool,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self { servers: Vec::new() }
    }
}

/// Placeholder: MCP implementation deferred to v1.
pub fn is_enabled() -> bool {
    false
}
