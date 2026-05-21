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
    let mut entries = Vec::new();
    for entry in WalkBuilder::new(dir).max_depth(Some(1)).build() {
        let entry = entry?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if entry.path() == dir {
            continue;
        }
        if let Some(name) = entry.path().file_name() {
            let mut label = name.to_string_lossy().to_string();
            if file_type.is_dir() {
                label.push('/');
            }
            if skip_context_entry(&label) {
                continue;
            }
            if file_type.is_dir() || file_type.is_file() {
                entries.push(label);
            }
        }
    }
    entries.sort();
    Ok(entries)
}

fn skip_context_entry(label: &str) -> bool {
    matches!(
        label,
        ".git/"
            | ".dsx/"
            | "target/"
            | "node_modules/"
            | "vendor/"
            | "dist/"
            | "build/"
            | ".next/"
            | "coverage/"
            | "__pycache__/"
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_files_includes_shallow_directories() {
        let root = temp_root("dsx_fs_shallow_dirs");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("1234/src")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(root.join("1234/src/main.rs"), "fn main() {}\n").unwrap();

        let files = list_files(&root).unwrap();

        assert!(files.contains(&"1234/".into()));
        assert!(files.contains(&"Cargo.toml".into()));
        assert!(!files.contains(&"main.rs".into()));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn list_files_skips_generated_and_vendor_directories() {
        let root = temp_root("dsx_fs_skip_generated");
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["target", "node_modules", "dist", "vendor", ".dsx"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("README.md"), "readme\n").unwrap();

        let files = list_files(&root).unwrap();

        assert!(files.contains(&"src/".into()));
        assert!(files.contains(&"README.md".into()));
        assert!(!files.contains(&"target/".into()));
        assert!(!files.contains(&"node_modules/".into()));
        assert!(!files.contains(&"dist/".into()));
        assert!(!files.contains(&".dsx/".into()));

        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}"))
    }
}
