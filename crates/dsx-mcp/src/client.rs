//! MCP stdio client and configured server helpers.

use crate::frame::{read_frame, write_frame};
use crate::types::{McpCallResult, McpServerConfig, McpTool};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

pub struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    next_id: u64,
}

impl McpClient {
    pub async fn connect_stdio(command: &str, args: &[String]) -> anyhow::Result<Self> {
        Self::connect_stdio_with_options(command, args, &BTreeMap::new(), None).await
    }

    pub async fn connect_stdio_with_options(
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
        cwd: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let mut child = spawn_stdio(command, args, env, cwd)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open MCP server stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open MCP server stdout"))?;
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        client.initialize().await?;
        Ok(client)
    }

    pub async fn initialize(&mut self) -> anyhow::Result<Value> {
        let result = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": DEFAULT_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "dsx",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }),
            )
            .await?;
        self.notify("notifications/initialized", json!({})).await?;
        Ok(result)
    }

    pub async fn list_tools(&mut self) -> anyhow::Result<Vec<McpTool>> {
        let result = self.request("tools/list", json!({})).await?;
        let tools = result
            .get("tools")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("MCP tools/list response missing 'tools'"))?;
        Ok(serde_json::from_value(tools)?)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> anyhow::Result<McpCallResult> {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    async fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        write_frame(
            &mut self.stdin,
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
        )
        .await?;

        loop {
            let response = read_frame(&mut self.stdout).await?;
            if response.get("id").and_then(|value| value.as_u64()) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                anyhow::bail!("MCP request '{method}' failed: {error}");
            }
            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> anyhow::Result<()> {
        write_frame(
            &mut self.stdin,
            &json!({ "jsonrpc": "2.0", "method": method, "params": params }),
        )
        .await
    }

    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

pub async fn list_server_tools(server: &McpServerConfig) -> anyhow::Result<Vec<McpTool>> {
    let mut client = connect_configured_server(server).await?;
    let tools = client.list_tools().await;
    let _ = client.shutdown().await;
    tools
}

pub async fn call_server_tool(
    server: &McpServerConfig,
    tool: &str,
    arguments: Value,
) -> anyhow::Result<McpCallResult> {
    let mut client = connect_configured_server(server).await?;
    let result = client.call_tool(tool, arguments).await;
    let _ = client.shutdown().await;
    result
}

async fn connect_configured_server(server: &McpServerConfig) -> anyhow::Result<McpClient> {
    if !server.transport.eq_ignore_ascii_case("stdio") {
        anyhow::bail!(
            "MCP server '{}' uses unsupported transport '{}'",
            server.name,
            server.transport
        );
    }
    let command = server
        .command
        .as_deref()
        .filter(|command| !command.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("MCP server '{}' is missing command", server.name))?;

    McpClient::connect_stdio_with_options(command, &server.args, &server.env, server.cwd.as_deref())
        .await
}

fn spawn_stdio(
    command: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
    cwd: Option<&Path>,
) -> anyhow::Result<Child> {
    let mut process = Command::new(command);
    process
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if !env.is_empty() {
        process.envs(env);
    }
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    Ok(process.spawn()?)
}
