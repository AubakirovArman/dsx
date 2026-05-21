//! Resolve the narrow task workspace from a broader launch workspace.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TaskScope {
    pub launch_root: PathBuf,
    pub active_root: PathBuf,
    pub narrowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExplicitPath {
    path: PathBuf,
    directory_hint: bool,
}

impl TaskScope {
    pub fn system_note(&self) -> String {
        if self.narrowed {
            format!(
                "Launch workspace: {}\nActive task scope: {}\nOnly inspect, edit, and run commands inside the active task scope unless the user explicitly changes scope.",
                self.launch_root.display(),
                self.active_root.display()
            )
        } else {
            format!(
                "Active task scope: {}\nOnly inspect, edit, and run commands inside this workspace.",
                self.active_root.display()
            )
        }
    }
}

pub fn resolve_task_scope(launch_root: &Path, task: &str) -> anyhow::Result<TaskScope> {
    let launch_root = launch_root.canonicalize()?;
    let mut candidates = explicit_path_candidates(&launch_root, task);
    candidates.sort();
    candidates.dedup();

    let active_root = candidates
        .into_iter()
        .filter_map(|candidate| scope_candidate(&launch_root, &candidate).ok())
        .max_by_key(|path| path.components().count())
        .unwrap_or_else(|| launch_root.clone());
    let narrowed = active_root != launch_root;

    Ok(TaskScope {
        launch_root,
        active_root,
        narrowed,
    })
}

pub fn ensure_active_root(scope: &TaskScope) -> anyhow::Result<()> {
    if scope.narrowed && !scope.active_root.exists() {
        std::fs::create_dir_all(&scope.active_root)?;
    }
    Ok(())
}

fn explicit_path_candidates(launch_root: &Path, task: &str) -> Vec<ExplicitPath> {
    task.split_whitespace()
        .filter_map(|raw| {
            let cleaned = raw.trim_matches(|c: char| {
                matches!(
                    c,
                    '`' | '"' | '\'' | ',' | '.' | ':' | ';' | ')' | '(' | ']' | '[' | '}' | '{'
                )
            });
            if cleaned.is_empty() {
                return None;
            }
            let path = PathBuf::from(cleaned);
            let directory_hint =
                cleaned.ends_with('/') || cleaned.ends_with('\\') || path.extension().is_none();
            let path = if path.is_absolute() {
                path
            } else if cleaned.contains('/') || cleaned.contains('\\') {
                launch_root.join(path)
            } else {
                return None;
            };
            Some(ExplicitPath {
                path,
                directory_hint,
            })
        })
        .collect()
}

fn scope_candidate(launch_root: &Path, candidate: &ExplicitPath) -> anyhow::Result<PathBuf> {
    let path = if candidate.path.exists() {
        let canonical = candidate.path.canonicalize()?;
        if canonical.is_file() {
            canonical
                .parent()
                .ok_or_else(|| anyhow::anyhow!("path has no parent"))?
                .to_path_buf()
        } else {
            canonical
        }
    } else if candidate.directory_hint {
        missing_path_under_launch(launch_root, &candidate.path)?
    } else {
        let parent = candidate
            .path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("path has no parent"))?;
        missing_path_under_launch(launch_root, parent)?
    };

    if !path.starts_with(launch_root) {
        anyhow::bail!("task path is outside launch workspace: {}", path.display());
    }
    Ok(path)
}

fn missing_path_under_launch(launch_root: &Path, path: &Path) -> anyhow::Result<PathBuf> {
    let existing = nearest_existing_parent(path)?;
    let canonical = existing.canonicalize()?;
    if !canonical.starts_with(launch_root) {
        anyhow::bail!("task path is outside launch workspace: {}", path.display());
    }
    let tail = path.strip_prefix(existing)?;
    if tail.components().any(|part| {
        matches!(
            part,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        anyhow::bail!("task path contains unsafe traversal: {}", path.display());
    }
    Ok(canonical.join(tail))
}

fn nearest_existing_parent(path: &Path) -> anyhow::Result<PathBuf> {
    let mut current = path;
    while !current.exists() {
        current = current
            .parent()
            .ok_or_else(|| anyhow::anyhow!("path has no existing parent"))?;
    }
    Ok(current.canonicalize()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrows_to_absolute_subdirectory() {
        let root = std::env::temp_dir().join("dsx_scope_abs");
        let target = root.join("sites/1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target).unwrap();

        let task = format!("создай проект только в {}", target.display());
        let scope = resolve_task_scope(&root, &task).unwrap();

        assert_eq!(scope.active_root, target.canonicalize().unwrap());
        assert!(scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ignores_paths_outside_launch_workspace() {
        let root = std::env::temp_dir().join("dsx_scope_inside");
        let outside = std::env::temp_dir().join("dsx_scope_outside");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let task = format!("используй {}", outside.display());
        let scope = resolve_task_scope(&root, &task).unwrap();

        assert_eq!(scope.active_root, root.canonicalize().unwrap());
        assert!(!scope.narrowed);

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn narrows_to_missing_directory_inside_launch_workspace() {
        let root = std::env::temp_dir().join("dsx_scope_missing_dir");
        let target = root.join("sites/1234");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sites")).unwrap();

        let task = format!("создай проект только в {}", target.display());
        let scope = resolve_task_scope(&root, &task).unwrap();

        assert_eq!(scope.active_root, target);
        assert!(scope.narrowed);
        assert!(!scope.active_root.exists());
        ensure_active_root(&scope).unwrap();
        assert!(scope.active_root.is_dir());

        let _ = std::fs::remove_dir_all(&root);
    }
}
