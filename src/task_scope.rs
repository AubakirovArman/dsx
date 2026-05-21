//! App-level helpers for resolving and displaying active task scope.

use std::path::{Path, PathBuf};

pub(crate) struct ResolvedTaskScope {
    pub(crate) active_root: PathBuf,
    pub(crate) launch_label: String,
    pub(crate) active_label: String,
    pub(crate) narrowed: bool,
}

pub(crate) fn resolve_task_scope(project_root: &Path, task: &str) -> ResolvedTaskScope {
    let scope = dsx_agent::scope::resolve_task_scope(project_root, task).ok();
    let active_root = scope
        .as_ref()
        .map(|scope| scope.active_root.clone())
        .unwrap_or_else(|| project_root.to_path_buf());
    let narrowed = scope.as_ref().map(|scope| scope.narrowed).unwrap_or(false);
    ResolvedTaskScope {
        launch_label: project_root.display().to_string(),
        active_label: active_root.display().to_string(),
        active_root,
        narrowed,
    }
}
