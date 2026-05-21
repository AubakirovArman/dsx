//! Layered config file discovery, parsing, and merging.

use crate::layers::{AppConfigLayer, ModelSettingsLayer, ModelSpecLayer};
use crate::settings::{AppConfig, ModelSettings, ModelSpec, PermissionsLayer, ProjectSettings};
use std::path::{Path, PathBuf};

pub fn load() -> anyhow::Result<AppConfig> {
    load_for_project(&std::env::current_dir()?)
}

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
        apply(app.default_mode, &mut config.app.default_mode);
        apply(app.theme, &mut config.app.theme);
    }
    if let Some(paths) = layer.paths {
        apply(paths.global_memory, &mut config.paths.global_memory);
        apply(paths.sessions_dir, &mut config.paths.sessions_dir);
    }
    if let Some(provider) = layer.provider {
        apply(provider.api_key_env, &mut config.provider.api_key_env);
        apply(
            provider.openai_base_url,
            &mut config.provider.openai_base_url,
        );
        apply(
            provider.timeout_seconds,
            &mut config.provider.timeout_seconds,
        );
        apply(provider.max_retries, &mut config.provider.max_retries);
    }
    if let Some(models) = layer.models {
        merge_model_settings(&mut config.models, models);
    }
    if let Some(routing) = layer.routing {
        apply(routing.classifier, &mut config.routing.classifier);
        apply(routing.reviewer, &mut config.routing.reviewer);
        apply(routing.summarizer, &mut config.routing.summarizer);
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
    apply(layer.provider, &mut config.provider);
    apply(layer.model, &mut config.model);
    if let Some(thinking) = layer.thinking {
        config.thinking = Some(thinking);
    }
    if let Some(reasoning_effort) = layer.reasoning_effort {
        config.reasoning_effort = Some(reasoning_effort);
    }
}

fn apply<T>(value: Option<T>, target: &mut T) {
    if let Some(value) = value {
        *target = value;
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
