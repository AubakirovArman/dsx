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
    let allow_bare = has_scope_hint(task);
    task.split_whitespace()
        .filter_map(|raw| {
            let cleaned = clean_path_token(raw);
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
            } else if allow_bare {
                bare_child_candidate(launch_root, cleaned, task)?
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

fn bare_child_candidate(launch_root: &Path, token: &str, task: &str) -> Option<PathBuf> {
    if !safe_bare_name(token) {
        return None;
    }
    let candidate = launch_root.join(token);
    if candidate.is_dir() || (has_creation_hint(task) && plausible_missing_project_name(token)) {
        Some(candidate)
    } else {
        None
    }
}

fn plausible_missing_project_name(token: &str) -> bool {
    token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

fn safe_bare_name(token: &str) -> bool {
    if token == "." || token == ".." || scope_noise_word(token) {
        return false;
    }
    token
        .chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_'))
}

fn scope_noise_word(token: &str) -> bool {
    matches!(
        token.to_lowercase().as_str(),
        "folder"
            | "directory"
            | "workspace"
            | "project"
            | "repo"
            | "only"
            | "inside"
            | "in"
            | "to"
            | "for"
            | "use"
            | "в"
            | "во"
            | "для"
            | "папку"
            | "папка"
            | "папке"
            | "директорию"
            | "каталог"
            | "проект"
            | "только"
            | "используй"
            | "создай"
    )
}

fn clean_path_token(raw: &str) -> &str {
    let trimmed = raw
        .trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ')' | '(' | ']' | '[' | '}' | '{'));
    trimmed.trim_end_matches([',', '.', ':', ';'])
}

fn has_creation_hint(task: &str) -> bool {
    let lower = task.to_lowercase();
    ["create", "new", "scaffold", "созд", "нов", "сгенер"]
        .iter()
        .any(|hint| lower.contains(hint))
}

fn has_scope_hint(task: &str) -> bool {
    let lower = task.to_lowercase();
    [
        "folder",
        "directory",
        "workspace",
        "project",
        "repo",
        "only",
        "inside",
        "use",
        "пап",
        "директор",
        "каталог",
        "проект",
        "репозитор",
        "воркспейс",
        "только",
        "использ",
        "внутри",
    ]
    .iter()
    .any(|hint| lower.contains(hint))
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
