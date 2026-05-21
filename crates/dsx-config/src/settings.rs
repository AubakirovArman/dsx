//! Public DSX configuration model and defaults.

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
    pub scope: ScopeSettings,
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
pub struct ScopeSettings {
    #[serde(default)]
    pub allow_wide: bool,
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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                default_mode: "ask".into(),
                theme: "dark".into(),
            },
            paths: PathSettings {
                global_memory: data_path("dsx/memory.sqlite"),
                sessions_dir: data_path("dsx/sessions"),
            },
            provider: ProviderSettings {
                api_key_env: "DEEPSEEK_API_KEY".into(),
                openai_base_url: "https://api.deepseek.com".into(),
                timeout_seconds: 600,
                max_retries: 4,
            },
            models: default_models(),
            routing: RoutingSettings {
                classifier: "fast".into(),
                reviewer: "fast".into(),
                summarizer: "fast".into(),
            },
            scope: ScopeSettings::default(),
            project: None,
            permissions: None,
        }
    }
}

fn default_mode() -> String {
    "ask".into()
}

fn default_theme() -> String {
    "dark".into()
}

fn data_path(path: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(path)
}

fn default_models() -> ModelSettings {
    ModelSettings {
        hard: model("deepseek-v4-pro", Some("enabled"), Some("high")),
        hard_max: model("deepseek-v4-pro", Some("enabled"), Some("max")),
        fast: model("deepseek-v4-flash", None, None),
        fast_thinking: model("deepseek-v4-flash", Some("enabled"), Some("high")),
    }
}

fn model(name: &str, thinking: Option<&str>, effort: Option<&str>) -> ModelSpec {
    ModelSpec {
        provider: "deepseek".into(),
        model: name.into(),
        thinking: thinking.map(str::to_string),
        reasoning_effort: effort.map(str::to_string),
    }
}
