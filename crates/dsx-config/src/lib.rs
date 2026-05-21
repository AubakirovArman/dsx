//! DSX Config — layered config loader (global → project → team).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub paths: PathSettings,
    pub provider: ProviderSettings,
    pub models: ModelSettings,
    pub routing: RoutingSettings,
    #[serde(default)]
    pub project: Option<ProjectSettings>,
    #[serde(default)]
    pub permissions: Option<PermissionsLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_mode")]
    pub default_mode: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_mode() -> String { "ask".into() }
fn default_theme() -> String { "dark".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSettings {
    pub global_memory: PathBuf,
    pub sessions_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub api_key_env: String,
    pub openai_base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    pub hard: ModelSpec,
    pub hard_max: ModelSpec,
    pub fast: ModelSpec,
    pub fast_thinking: ModelSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub provider: String,
    pub model: String,
    pub thinking: Option<String>,
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSettings {
    pub classifier: String,
    pub reviewer: String,
    pub summarizer: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionsLayer {
    pub mode: Option<String>,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub commands: Vec<CommandRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRule {
    pub pattern: String,
    pub risk: String,
}

/// Load config from global + project + team files.
pub fn load() -> anyhow::Result<AppConfig> {
    // TODO: merge global ~/.config/dsx/config.toml, project .deepseek-code/project.toml, team.toml
    let global_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dsx");
    let config_path = global_dir.join("config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(AppConfig::default())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                default_mode: "ask".into(),
                theme: "dark".into(),
            },
            paths: PathSettings {
                global_memory: dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("dsx/memory.sqlite"),
                sessions_dir: dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("dsx/sessions"),
            },
            provider: ProviderSettings {
                api_key_env: "DEEPSEEK_API_KEY".into(),
                openai_base_url: "https://api.deepseek.com".into(),
                timeout_seconds: 600,
                max_retries: 4,
            },
            models: ModelSettings {
                hard: ModelSpec { provider: "deepseek".into(), model: "deepseek-v4-pro".into(), thinking: Some("enabled".into()), reasoning_effort: Some("high".into()) },
                hard_max: ModelSpec { provider: "deepseek".into(), model: "deepseek-v4-pro".into(), thinking: Some("enabled".into()), reasoning_effort: Some("max".into()) },
                fast: ModelSpec { provider: "deepseek".into(), model: "deepseek-v4-flash".into(), thinking: None, reasoning_effort: None },
                fast_thinking: ModelSpec { provider: "deepseek".into(), model: "deepseek-v4-flash".into(), thinking: Some("enabled".into()), reasoning_effort: Some("high".into()) },
            },
            routing: RoutingSettings {
                classifier: "fast".into(),
                reviewer: "fast".into(),
                summarizer: "fast".into(),
            },
            project: None,
            permissions: None,
        }
    }
}
