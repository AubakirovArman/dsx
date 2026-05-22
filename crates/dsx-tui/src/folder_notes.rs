//! Compact per-folder note helpers for the workflow panel.

use crate::{App, FolderNote};
use std::path::Path;

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
