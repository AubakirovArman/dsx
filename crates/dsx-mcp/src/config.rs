//! MCP configuration loading and normalization.

use crate::types::{McpConfig, McpServerConfig, default_enabled, default_transport};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

pub(crate) fn parse_config_str(content: &str) -> anyhow::Result<McpConfig> {
    parse_config_value(serde_json::from_str(content)?)
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
        let servers = array
            .iter()
            .map(|item| parse_server(item, fallback_name))
            .collect::<anyhow::Result<Vec<_>>>()?;
        return Ok(McpConfig { servers });
    }

    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("MCP config servers must be an array or object"))?;
    let servers = object
        .iter()
        .map(|(name, item)| parse_server(item, Some(name)))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(McpConfig { servers })
}

fn parse_server(value: &Value, fallback_name: Option<&str>) -> anyhow::Result<McpServerConfig> {
    let raw: RawServerConfig = serde_json::from_value(value.clone())?;
    raw.into_server(fallback_name)
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
