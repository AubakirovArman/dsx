//! Workspace task summary notes across launch and child scopes.

use std::path::{Path, PathBuf};

pub(crate) struct WorkspaceNote {
    pub(crate) label: String,
    pub(crate) saved: bool,
    pub(crate) goal: String,
    pub(crate) done: String,
    pub(crate) plan: String,
    pub(crate) last_changes: String,
    pub(crate) next_step: String,
    pub(crate) architecture: String,
}

pub async fn list_workspace_notes(project_root: &Path, limit: u32, all: bool, json: bool) {
    match collect_workspace_notes(project_root, limit, all).await {
        Ok(notes) if notes.is_empty() => println!("No workspace notes yet."),
        Ok(notes) if json => println!("{}", notes_json_value(&notes)),
        Ok(notes) => print_notes(&notes, all),
        Err(e) => println!("Workspace notes error: {e}"),
    }
}

pub(crate) async fn collect_workspace_notes(
    project_root: &Path,
    limit: u32,
    all: bool,
) -> anyhow::Result<Vec<WorkspaceNote>> {
    let scopes = note_scopes(project_root, limit, all)?;
    let mut notes = Vec::new();
    for scope in scopes {
        let summary = load_summary(&scope).await;
        notes.push(note_from_scope(project_root, &scope, summary.as_ref()));
    }
    Ok(notes)
}

fn note_scopes(project_root: &Path, limit: u32, all: bool) -> anyhow::Result<Vec<PathBuf>> {
    let mut scopes = vec![project_root.to_path_buf()];
    if all {
        for entry in direct_child_dirs(project_root)?
            .into_iter()
            .take(limit as usize)
        {
            scopes.push(entry);
        }
    }
    scopes.truncate(limit.max(1) as usize);
    Ok(scopes)
}

fn direct_child_dirs(project_root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = std::fs::read_dir(project_root)?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(note_candidate)
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    dirs.sort();
    Ok(dirs)
}

async fn load_summary(scope: &Path) -> Option<dsx_memory::TaskSummary> {
    let db_path = scope.join(".dsx").join("sessions.db");
    if !db_path.exists() {
        return None;
    }
    let pool = dsx_memory::open(&db_path).await.ok()?;
    let root = scope.display().to_string();
    dsx_memory::load_task_summary(&pool, &root)
        .await
        .ok()
        .flatten()
}

fn note_from_scope(
    project_root: &Path,
    scope: &Path,
    summary: Option<&dsx_memory::TaskSummary>,
) -> WorkspaceNote {
    WorkspaceNote {
        label: scope_label(project_root, scope),
        saved: summary.is_some(),
        goal: field(summary, |s| &s.goal, "No saved goal yet."),
        done: field(summary, |s| &s.done, "No saved completion state yet."),
        plan: field(summary, |s| &s.plan, "No saved plan yet."),
        last_changes: field(summary, |s| &s.last_changes, fallback_last(scope)),
        next_step: field(summary, |s| &s.next_step, "No saved next step yet."),
        architecture: field(summary, |s| &s.architecture, fallback_arch(scope)),
    }
}

fn field(
    summary: Option<&dsx_memory::TaskSummary>,
    pick: impl Fn(&dsx_memory::TaskSummary) -> &String,
    fallback: impl Into<String>,
) -> String {
    summary
        .map(pick)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(short)
        .unwrap_or_else(|| fallback.into())
}

fn print_notes(notes: &[WorkspaceNote], all: bool) {
    println!(
        "Workspace notes{}:",
        if all { " across scopes" } else { "" }
    );
    for note in notes {
        println!(
            "  [{}] {}",
            note.label,
            if note.saved { "saved" } else { "fallback" }
        );
        print_field("goal", &note.goal);
        print_field("done", &note.done);
        print_field("plan", &note.plan);
        print_field("last", &note.last_changes);
        print_field("next", &note.next_step);
        print_field("arch", &note.architecture);
    }
}

fn print_field(label: &str, value: &str) {
    println!("      {label}: {}", value.replace('\n', " | "));
}

pub(crate) fn notes_json_value(notes: &[WorkspaceNote]) -> serde_json::Value {
    serde_json::Value::Array(notes.iter().map(note_json_value).collect())
}

fn note_json_value(note: &WorkspaceNote) -> serde_json::Value {
    serde_json::json!({
        "scope": note.label,
        "saved": note.saved,
        "goal": note.goal,
        "done": note.done,
        "plan": note.plan,
        "last_changes": note.last_changes,
        "next_step": note.next_step,
        "architecture": note.architecture,
    })
}

fn scope_label(project_root: &Path, scope: &Path) -> String {
    scope
        .strip_prefix(project_root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".".into())
}

fn note_candidate(name: &str) -> bool {
    !matches!(name, ".git" | ".dsx" | "target" | "node_modules") && !name.starts_with('.')
}

fn fallback_last(scope: &Path) -> String {
    format!("{} direct item(s) visible.", direct_item_count(scope))
}

fn fallback_arch(scope: &Path) -> String {
    let name = scope
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".");
    format!("{name}/: project folder; open details only when needed")
}

fn direct_item_count(scope: &Path) -> usize {
    std::fs::read_dir(scope).map(|rd| rd.count()).unwrap_or(0)
}

fn short(value: &str) -> String {
    const LIMIT: usize = 220;
    let mut text: String = value.chars().take(LIMIT).collect();
    if value.chars().count() > LIMIT {
        text.push_str("...");
    }
    text
}
