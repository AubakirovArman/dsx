//! DSX Config — layered config loader (global → project → team).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

fn default_mode() -> String {
    "ask".into()
}
fn default_theme() -> String {
    "dark".into()
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

/// Load config from global + current working directory project files.
pub fn load() -> anyhow::Result<AppConfig> {
    load_for_project(&std::env::current_dir()?)
}

/// Load config from global files first, then project-local files.
pub fn load_for_project(project_root: &Path) -> anyhow::Result<AppConfig> {
    load_from_paths(&config_paths(project_root))
}

fn config_paths(project_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("dsx").join("config.toml"));
        paths.push(config_dir.join("deepseek-code").join("config.toml"));
    }
    paths.push(project_root.join(".deepseek-code").join("project.toml"));
    paths.push(project_root.join(".deepseek-code").join("config.toml"));
    paths.push(project_root.join(".deepseek").join("project.toml"));
    paths.push(project_root.join(".dsx").join("config.toml"));
    paths
}

fn load_from_paths(paths: &[PathBuf]) -> anyhow::Result<AppConfig> {
    let mut config = AppConfig::default();
    for path in paths {
        if !path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        let layer: AppConfigLayer = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse config {}: {e}", path.display()))?;
        merge_layer(&mut config, layer);
    }
    Ok(config)
}

fn merge_layer(config: &mut AppConfig, layer: AppConfigLayer) {
    if let Some(app) = layer.app {
        if let Some(default_mode) = app.default_mode {
            config.app.default_mode = default_mode;
        }
        if let Some(theme) = app.theme {
            config.app.theme = theme;
        }
    }

    if let Some(paths) = layer.paths {
        if let Some(global_memory) = paths.global_memory {
            config.paths.global_memory = global_memory;
        }
        if let Some(sessions_dir) = paths.sessions_dir {
            config.paths.sessions_dir = sessions_dir;
        }
    }

    if let Some(provider) = layer.provider {
        if let Some(api_key_env) = provider.api_key_env {
            config.provider.api_key_env = api_key_env;
        }
        if let Some(openai_base_url) = provider.openai_base_url {
            config.provider.openai_base_url = openai_base_url;
        }
        if let Some(timeout_seconds) = provider.timeout_seconds {
            config.provider.timeout_seconds = timeout_seconds;
        }
        if let Some(max_retries) = provider.max_retries {
            config.provider.max_retries = max_retries;
        }
    }

    if let Some(models) = layer.models {
        merge_model_settings(&mut config.models, models);
    }

    if let Some(routing) = layer.routing {
        if let Some(classifier) = routing.classifier {
            config.routing.classifier = classifier;
        }
        if let Some(reviewer) = routing.reviewer {
            config.routing.reviewer = reviewer;
        }
        if let Some(summarizer) = routing.summarizer {
            config.routing.summarizer = summarizer;
        }
    }

    if let Some(project) = layer.project {
        let target = config.project.get_or_insert_with(ProjectSettings::default);
        if let Some(name) = project.name {
            target.name = Some(name);
        }
    }

    if let Some(permissions) = layer.permissions {
        let target = config
            .permissions
            .get_or_insert_with(PermissionsLayer::default);
        if let Some(mode) = permissions.mode {
            target.mode = Some(mode);
        }
        target.allow.extend(permissions.allow);
        target.deny.extend(permissions.deny);
        target.commands.extend(permissions.commands);
    }
}

fn merge_model_settings(config: &mut ModelSettings, layer: ModelSettingsLayer) {
    if let Some(hard) = layer.hard {
        merge_model_spec(&mut config.hard, hard);
    }
    if let Some(hard_max) = layer.hard_max {
        merge_model_spec(&mut config.hard_max, hard_max);
    }
    if let Some(fast) = layer.fast {
        merge_model_spec(&mut config.fast, fast);
    }
    if let Some(fast_thinking) = layer.fast_thinking {
        merge_model_spec(&mut config.fast_thinking, fast_thinking);
    }
}

fn merge_model_spec(config: &mut ModelSpec, layer: ModelSpecLayer) {
    if let Some(provider) = layer.provider {
        config.provider = provider;
    }
    if let Some(model) = layer.model {
        config.model = model;
    }
    if let Some(thinking) = layer.thinking {
        config.thinking = Some(thinking);
    }
    if let Some(reasoning_effort) = layer.reasoning_effort {
        config.reasoning_effort = Some(reasoning_effort);
    }
}

#[derive(Debug, Default, Deserialize)]
struct AppConfigLayer {
    app: Option<AppSettingsLayer>,
    paths: Option<PathSettingsLayer>,
    provider: Option<ProviderSettingsLayer>,
    models: Option<ModelSettingsLayer>,
    routing: Option<RoutingSettingsLayer>,
    project: Option<ProjectSettingsLayer>,
    permissions: Option<PermissionsLayerPatch>,
}

#[derive(Debug, Default, Deserialize)]
struct AppSettingsLayer {
    default_mode: Option<String>,
    theme: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PathSettingsLayer {
    global_memory: Option<PathBuf>,
    sessions_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct ProviderSettingsLayer {
    api_key_env: Option<String>,
    openai_base_url: Option<String>,
    timeout_seconds: Option<u64>,
    max_retries: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
struct ModelSettingsLayer {
    hard: Option<ModelSpecLayer>,
    hard_max: Option<ModelSpecLayer>,
    fast: Option<ModelSpecLayer>,
    fast_thinking: Option<ModelSpecLayer>,
}

#[derive(Debug, Default, Deserialize)]
struct ModelSpecLayer {
    provider: Option<String>,
    model: Option<String>,
    thinking: Option<String>,
    reasoning_effort: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RoutingSettingsLayer {
    classifier: Option<String>,
    reviewer: Option<String>,
    summarizer: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ProjectSettingsLayer {
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PermissionsLayerPatch {
    mode: Option<String>,
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    deny: Vec<String>,
    #[serde(default)]
    commands: Vec<CommandRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                default_mode: "ask".into(),
                theme: "dark".into(),
            },
            paths: PathSettings {
                global_memory: dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("dsx/memory.sqlite"),
                sessions_dir: dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("dsx/sessions"),
            },
            provider: ProviderSettings {
                api_key_env: "DEEPSEEK_API_KEY".into(),
                openai_base_url: "https://api.deepseek.com".into(),
                timeout_seconds: 600,
                max_retries: 4,
            },
            models: ModelSettings {
                hard: ModelSpec {
                    provider: "deepseek".into(),
                    model: "deepseek-v4-pro".into(),
                    thinking: Some("enabled".into()),
                    reasoning_effort: Some("high".into()),
                },
                hard_max: ModelSpec {
                    provider: "deepseek".into(),
                    model: "deepseek-v4-pro".into(),
                    thinking: Some("enabled".into()),
                    reasoning_effort: Some("max".into()),
                },
                fast: ModelSpec {
                    provider: "deepseek".into(),
                    model: "deepseek-v4-flash".into(),
                    thinking: None,
                    reasoning_effort: None,
                },
                fast_thinking: ModelSpec {
                    provider: "deepseek".into(),
                    model: "deepseek-v4-flash".into(),
                    thinking: Some("enabled".into()),
                    reasoning_effort: Some("high".into()),
                },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_paths_merges_global_and_project_layers() {
        let tmp = std::env::temp_dir().join("dsx_test_config_merge");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("global")).unwrap();
        std::fs::create_dir_all(tmp.join("project/.deepseek-code")).unwrap();
        let global = tmp.join("global/config.toml");
        let project = tmp.join("project/.deepseek-code/project.toml");

        std::fs::write(
            &global,
            r#"
[app]
default_mode = "ask"
theme = "light"

[provider]
api_key_env = "GLOBAL_DEEPSEEK_KEY"
openai_base_url = "https://global.example/v1"

[permissions]
allow = ["git status"]
deny = ["rm -rf *"]
"#,
        )
        .unwrap();
        std::fs::write(
            &project,
            r#"
[app]
default_mode = "auto"

[permissions]
allow = ["cargo test"]
"#,
        )
        .unwrap();

        let config = load_from_paths(&[global, project]).unwrap();

        assert_eq!(config.app.default_mode, "auto");
        assert_eq!(config.app.theme, "light");
        assert_eq!(config.provider.api_key_env, "GLOBAL_DEEPSEEK_KEY");
        let permissions = config.permissions.unwrap();
        assert_eq!(
            permissions.allow,
            vec!["git status".to_string(), "cargo test".to_string()]
        );
        assert_eq!(permissions.deny, vec!["rm -rf *".to_string()]);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
