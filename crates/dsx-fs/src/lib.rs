//! DSX File System — workspace-aware file operations with `.gitignore` support.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

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
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false)
            && let Some(name) = entry.path().file_name()
        {
            files.push(name.to_string_lossy().to_string());
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

/// Resolve a path that may not exist yet, while still preventing traversal.
pub fn resolve_path_allow_missing(workspace: &Path, relative: &str) -> anyhow::Result<PathBuf> {
    if relative.trim().is_empty() {
        anyhow::bail!("path is required");
    }

    let ws_canonical = workspace.canonicalize()?;
    let candidate = workspace.join(relative);
    let parent = candidate
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {relative}"))?;

    let parent_canonical = if parent.exists() {
        parent.canonicalize()?
    } else {
        let mut existing = parent;
        while !existing.exists() {
            existing = existing
                .parent()
                .ok_or_else(|| anyhow::anyhow!("path parent does not exist: {relative}"))?;
        }
        existing.canonicalize()?
    };

    if !parent_canonical.starts_with(&ws_canonical) {
        anyhow::bail!("path traversal blocked: {relative}");
    }

    Ok(candidate)
}
