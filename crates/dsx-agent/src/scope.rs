//! Resolve the narrow task workspace from a broader launch workspace.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TaskScope {
    pub launch_root: PathBuf,
    pub active_root: PathBuf,
    pub narrowed: bool,
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

fn explicit_path_candidates(launch_root: &Path, task: &str) -> Vec<PathBuf> {
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
            if path.is_absolute() {
                Some(path)
            } else if cleaned.contains('/') || cleaned.contains('\\') {
                Some(launch_root.join(path))
            } else {
                None
            }
        })
        .collect()
}

fn scope_candidate(launch_root: &Path, candidate: &Path) -> anyhow::Result<PathBuf> {
    let path = if candidate.exists() {
        let canonical = candidate.canonicalize()?;
        if canonical.is_file() {
            canonical
                .parent()
                .ok_or_else(|| anyhow::anyhow!("path has no parent"))?
                .to_path_buf()
        } else {
            canonical
        }
    } else {
        nearest_existing_parent(candidate)?
    };

    if !path.starts_with(launch_root) {
        anyhow::bail!("task path is outside launch workspace: {}", path.display());
    }
    Ok(path)
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
}
