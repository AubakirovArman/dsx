//! DSX MCP — Model Context Protocol stdio client.
//!
//! Implements the MCP JSON-RPC transport over `Content-Length` framed stdio
//! messages, plus initialize, tools/list, and tools/call.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_transport() -> String {
    "stdio".into()
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema", default)]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallResult {
    #[serde(default)]
    pub content: Vec<Value>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

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

        let mut child = process.spawn()?;
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
                json!({
                    "name": name,
                    "arguments": arguments,
                }),
            )
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    async fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        write_frame(&mut self.stdin, &payload).await?;

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
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        write_frame(&mut self.stdin, &payload).await
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

pub fn load_config(project_root: &Path) -> anyhow::Result<McpConfig> {
    let mut config = McpConfig::default();
    for path in config_paths(project_root) {
        if !path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        let mut next = parse_config_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse MCP config {}: {e}", path.display()))?;
        let cwd_base = if path.starts_with(project_root) {
            project_root
        } else {
            path.parent().unwrap_or(project_root)
        };
        normalize_relative_cwds(&mut next, cwd_base);
        merge_config(&mut config, next);
    }
    Ok(config)
}

pub fn enabled_servers(config: &McpConfig) -> impl Iterator<Item = &McpServerConfig> {
    config.servers.iter().filter(|server| server.enabled)
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

fn config_paths(project_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("dsx").join("mcp.json"));
    }
    paths.push(project_root.join(".deepseek-code").join("mcp.json"));
    paths.push(project_root.join(".deepseek").join("mcp.json"));
    paths.push(project_root.join(".dsx").join("mcp.json"));
    paths.push(project_root.join("mcp.json"));
    paths
}

fn merge_config(base: &mut McpConfig, next: McpConfig) {
    for server in next.servers {
        if let Some(existing) = base
            .servers
            .iter_mut()
            .find(|existing| existing.name == server.name)
        {
            *existing = server;
        } else {
            base.servers.push(server);
        }
    }
}

fn normalize_relative_cwds(config: &mut McpConfig, base: &Path) {
    for server in &mut config.servers {
        if let Some(cwd) = &server.cwd
            && cwd.is_relative()
        {
            server.cwd = Some(base.join(cwd));
        }
    }
}

fn parse_config_str(content: &str) -> anyhow::Result<McpConfig> {
    parse_config_value(serde_json::from_str(content)?)
}

fn parse_config_value(value: Value) -> anyhow::Result<McpConfig> {
    if value.is_array() {
        return parse_servers_value(&value, None);
    }

    if let Some(servers) = value.get("servers") {
        return parse_servers_value(servers, None);
    }

    if let Some(servers) = value.get("mcpServers") {
        return parse_servers_value(servers, None);
    }

    Ok(serde_json::from_value(value)?)
}

fn parse_servers_value(value: &Value, fallback_name: Option<&str>) -> anyhow::Result<McpConfig> {
    if let Some(array) = value.as_array() {
        let mut servers = Vec::new();
        for item in array {
            let raw: RawServerConfig = serde_json::from_value(item.clone())?;
            servers.push(raw.into_server(fallback_name)?);
        }
        return Ok(McpConfig { servers });
    }

    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("MCP config servers must be an array or object"))?;
    let mut servers = Vec::new();
    for (name, item) in object {
        let raw: RawServerConfig = serde_json::from_value(item.clone())?;
        servers.push(raw.into_server(Some(name))?);
    }
    Ok(McpConfig { servers })
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

#[derive(Debug, Deserialize)]
struct RawServerConfig {
    #[serde(default)]
    name: Option<String>,
    #[serde(default = "default_transport")]
    transport: String,
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default)]
    disabled: bool,
}

impl RawServerConfig {
    fn into_server(self, fallback_name: Option<&str>) -> anyhow::Result<McpServerConfig> {
        let name = self
            .name
            .or_else(|| fallback_name.map(ToOwned::to_owned))
            .ok_or_else(|| anyhow::anyhow!("MCP server config missing name"))?;
        if name.trim().is_empty() {
            anyhow::bail!("MCP server config has empty name");
        }

        Ok(McpServerConfig {
            name,
            transport: self.transport,
            command: self.command,
            args: self.args,
            env: self.env,
            cwd: self.cwd,
            enabled: self.enabled && !self.disabled,
        })
    }
}

pub async fn write_frame<W>(writer: &mut W, value: &Value) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R>(reader: &mut R) -> anyhow::Result<Value>
where
    R: AsyncRead + Unpin,
{
    let mut header = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        let n = reader.read(&mut byte).await?;
        if n == 0 {
            anyhow::bail!("MCP stream closed while reading frame header");
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") || header.ends_with(b"\n\n") {
            break;
        }
        if header.len() > 8192 {
            anyhow::bail!("MCP frame header too large");
        }
    }

    let header_text = String::from_utf8(header)?;
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("MCP frame missing Content-Length header"))?;

    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(serde_json::from_slice(&body)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn frame_round_trip() {
        let (mut client, mut server) = duplex(4096);
        let expected = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
        });

        let writer = tokio::spawn(async move {
            write_frame(&mut client, &expected).await.unwrap();
        });
        let actual = read_frame(&mut server).await.unwrap();
        writer.await.unwrap();

        assert_eq!(actual["method"], "tools/list");
        assert_eq!(actual["id"], 1);
    }

    #[tokio::test]
    async fn stdio_client_lists_and_calls_tools() {
        let script = r#"
import json, sys

def read_frame():
    header = b""
    while not header.endswith(b"\r\n\r\n"):
        b = sys.stdin.buffer.read(1)
        if not b:
            return None
        header += b
    length = 0
    for line in header.decode().splitlines():
        if line.lower().startswith("content-length:"):
            length = int(line.split(":", 1)[1].strip())
    return json.loads(sys.stdin.buffer.read(length))

def write_frame(obj):
    data = json.dumps(obj).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(data)}\r\n\r\n".encode() + data)
    sys.stdout.buffer.flush()

while True:
    msg = read_frame()
    if msg is None:
        break
    method = msg.get("method")
    if method == "initialize":
        write_frame({"jsonrpc":"2.0","id":msg["id"],"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake"}}})
    elif method == "notifications/initialized":
        pass
    elif method == "tools/list":
        write_frame({"jsonrpc":"2.0","id":msg["id"],"result":{"tools":[{"name":"echo","description":"Echo args","inputSchema":{"type":"object"}}]}})
    elif method == "tools/call":
        write_frame({"jsonrpc":"2.0","id":msg["id"],"result":{"content":[{"type":"text","text":json.dumps(msg["params"]["arguments"])}]}})
"#;
        let args = vec!["-c".to_string(), script.to_string()];
        let mut client = McpClient::connect_stdio("python3", &args).await.unwrap();
        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");

        let result = client.call_tool("echo", json!({"value": 7})).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content[0]["type"], "text");

        client.shutdown().await.unwrap();
    }

    #[test]
    fn load_config_reads_project_deepseek_code_mcp() {
        let tmp = std::env::temp_dir().join("dsx_test_mcp_config_project");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".deepseek-code")).unwrap();
        std::fs::write(
            tmp.join(".deepseek-code").join("mcp.json"),
            r#"{
                "servers": [
                    {
                        "name": "local",
                        "command": "python3",
                        "args": ["-c", "print(1)"]
                    }
                ]
            }"#,
        )
        .unwrap();

        let config = load_config(&tmp).unwrap();
        let server = config
            .servers
            .iter()
            .find(|server| server.name == "local")
            .unwrap();
        assert_eq!(server.command.as_deref(), Some("python3"));
        assert!(server.enabled);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_config_supports_mcp_servers_map_and_disabled_flag() {
        let config = parse_config_str(
            r#"{
                "mcpServers": {
                    "echo": {
                        "command": "python3",
                        "args": ["server.py"],
                        "disabled": true
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "echo");
        assert!(!config.servers[0].enabled);
    }
}
