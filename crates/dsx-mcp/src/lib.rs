//! DSX MCP — Model Context Protocol stdio client.

mod client;
mod config;
mod frame;
mod types;

pub use client::{McpClient, call_server_tool, list_server_tools};
pub use config::{enabled_servers, load_config};
pub use frame::{read_frame, write_frame};
pub use types::{McpCallResult, McpConfig, McpServerConfig, McpTool};

#[cfg(test)]
mod tests;
