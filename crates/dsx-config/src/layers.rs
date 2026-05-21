//! Partial config layer DTOs for TOML merging.

use crate::settings::CommandRule;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct AppConfigLayer {
    pub(crate) app: Option<AppSettingsLayer>,
    pub(crate) paths: Option<PathSettingsLayer>,
    pub(crate) provider: Option<ProviderSettingsLayer>,
    pub(crate) models: Option<ModelSettingsLayer>,
    pub(crate) routing: Option<RoutingSettingsLayer>,
    pub(crate) project: Option<ProjectSettingsLayer>,
    pub(crate) permissions: Option<PermissionsLayerPatch>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct AppSettingsLayer {
    pub(crate) default_mode: Option<String>,
    pub(crate) theme: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct PathSettingsLayer {
    pub(crate) global_memory: Option<PathBuf>,
    pub(crate) sessions_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderSettingsLayer {
    pub(crate) api_key_env: Option<String>,
    pub(crate) openai_base_url: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) max_retries: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ModelSettingsLayer {
    pub(crate) hard: Option<ModelSpecLayer>,
    pub(crate) hard_max: Option<ModelSpecLayer>,
    pub(crate) fast: Option<ModelSpecLayer>,
    pub(crate) fast_thinking: Option<ModelSpecLayer>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ModelSpecLayer {
    pub(crate) provider: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) thinking: Option<String>,
    pub(crate) reasoning_effort: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RoutingSettingsLayer {
    pub(crate) classifier: Option<String>,
    pub(crate) reviewer: Option<String>,
    pub(crate) summarizer: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProjectSettingsLayer {
    pub(crate) name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct PermissionsLayerPatch {
    pub(crate) mode: Option<String>,
    #[serde(default)]
    pub(crate) allow: Vec<String>,
    #[serde(default)]
    pub(crate) deny: Vec<String>,
    #[serde(default)]
    pub(crate) commands: Vec<CommandRule>,
}
