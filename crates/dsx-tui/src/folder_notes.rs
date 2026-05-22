//! Compact per-folder note helpers for the workflow panel.

use crate::{App, FolderNote};
use std::path::{Component, Path};

impl App {
    pub fn set_folder_notes(&mut self, notes: Vec<FolderNote>) {
        self.folder_notes = notes;
        self.clamp_folder_note_cursor();
    }

    pub fn upsert_folder_note(&mut self, scope: &str, summary: &str, next_step: &str) {
        let folder = scope_label(scope);
        if let Some((index, note)) = self
            .folder_notes
            .iter_mut()
            .enumerate()
            .find(|(_, note)| note.folder == folder)
        {
            note.summary = summary.into();
            note.next_step = next_step.into();
            self.folder_note_cursor = index;
            return;
        }
        if self.folder_notes.len() >= 12 {
            self.folder_notes.remove(0);
            self.folder_note_cursor = self.folder_note_cursor.saturating_sub(1);
        }
        self.folder_notes.push(FolderNote {
            architecture: folder_architecture(&folder),
            folder,
            summary: summary.into(),
            next_step: next_step.into(),
        });
        self.folder_note_cursor = self.folder_notes.len().saturating_sub(1);
    }

    pub fn select_next_folder_note(&mut self) {
        if self.folder_notes.is_empty() {
            self.folder_note_cursor = 0;
            return;
        }
        self.folder_note_cursor = (self.folder_note_cursor + 1) % self.folder_notes.len();
    }

    pub fn select_previous_folder_note(&mut self) {
        if self.folder_notes.is_empty() {
            self.folder_note_cursor = 0;
            return;
        }
        if self.folder_note_cursor == 0 {
            self.folder_note_cursor = self.folder_notes.len() - 1;
        } else {
            self.folder_note_cursor -= 1;
        }
    }

    pub fn focused_folder_note_index(&self) -> Option<usize> {
        if self.folder_notes.is_empty() {
            None
        } else {
            Some(self.folder_note_cursor.min(self.folder_notes.len() - 1))
        }
    }

    pub fn focused_folder_note(&self) -> Option<&FolderNote> {
        self.focused_folder_note_index()
            .and_then(|index| self.folder_notes.get(index))
    }

    pub fn focused_folder_scope(&self) -> Option<String> {
        let note = self.focused_folder_note()?;
        let launch = non_empty(&self.scope_lock.launch_scope)
            .or_else(|| non_empty(&self.scope_lock.active_scope))?;
        let label = note.folder.trim().trim_end_matches('/');
        if label == "." {
            return Some(launch.to_string());
        }
        let relative = safe_relative_path(label)?;
        Some(Path::new(launch).join(relative).display().to_string())
    }

    pub fn draft_focused_scope_task(&mut self) -> bool {
        let Some(note) = self.focused_folder_note() else {
            return false;
        };
        let label = note.folder.trim().trim_end_matches('/').to_string();
        let Some(scope) = self.focused_folder_scope() else {
            self.add_message(
                "error",
                "Focused folder is not safe for scoped task drafting.",
            );
            return false;
        };
        let prefix = if label == "." {
            "use current workspace only:".to_string()
        } else {
            format!("use folder {label} only:")
        };
        self.input = scoped_task_input(&prefix, &self.input);
        self.cursor_pos = self.input.chars().count();
        self.show_context = false;
        self.add_message(
            "system",
            &format!("Drafted scoped task for focused folder: {scope}"),
        );
        true
    }

    fn clamp_folder_note_cursor(&mut self) {
        if self.folder_notes.is_empty() {
            self.folder_note_cursor = 0;
        } else {
            self.folder_note_cursor = self.folder_note_cursor.min(self.folder_notes.len() - 1);
        }
    }
}

fn scope_label(scope: &str) -> String {
    Path::new(scope)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("{name}/"))
        .unwrap_or_else(|| "./".into())
}

fn folder_architecture(folder: &str) -> String {
    match folder.trim_end_matches('/') {
        "src" => "application source; inspect task-relevant files only",
        "crates" => "workspace crates; drill into the target crate before reading files",
        "docs" => "documentation and user-facing guidance",
        "plan" => "roadmap and architecture notes",
        "." => "launch workspace; choose a child scope when possible",
        _ => "active project folder; load detailed context only when needed",
    }
    .into()
}

fn scoped_task_input(prefix: &str, current: &str) -> String {
    let body = task_body_without_scope_prefix(current);
    if body.is_empty() {
        format!("{prefix} ")
    } else {
        format!("{prefix} {body}")
    }
}

fn task_body_without_scope_prefix(current: &str) -> &str {
    let trimmed = current.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("use current workspace only:") {
        return trimmed["use current workspace only:".len()..].trim_start();
    }
    if let Some(rest) = lower.strip_prefix("use folder ")
        && let Some(index) = rest.find(" only:")
    {
        let body_start = "use folder ".len() + index + " only:".len();
        return trimmed[body_start..].trim_start();
    }
    trimmed
}

fn safe_relative_path(label: &str) -> Option<&Path> {
    let path = Path::new(label);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return None;
    }
    path.components()
        .all(|part| matches!(part, Component::Normal(_)))
        .then_some(path)
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}
