//! DSX File System — workspace-aware file operations with `.gitignore` support.

use std::path::{Path, PathBuf};
use ignore::WalkBuilder;

/// Find the workspace root by walking up from `start` looking for `.git`.
pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// List files in a directory, respecting `.gitignore`.
pub fn list_files(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in WalkBuilder::new(dir).max_depth(Some(1)).build() {
        let entry = entry?;
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            if let Some(name) = entry.path().file_name() {
                files.push(name.to_string_lossy().to_string());
            }
        }
    }
    Ok(files)
}

/// Read file content as UTF-8 string.
pub fn read_file(path: &Path) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

/// Resolve a relative path against the workspace root, preventing traversal.
pub fn resolve_path(workspace: &Path, relative: &str) -> anyhow::Result<PathBuf> {
    let candidate = workspace.join(relative);
    let canonical = candidate.canonicalize()?;
    let ws_canonical = workspace.canonicalize()?;
    if !canonical.starts_with(&ws_canonical) {
        anyhow::bail!("path traversal blocked: {relative}");
    }
    Ok(canonical)
}
