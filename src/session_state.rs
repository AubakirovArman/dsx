//! Helpers for compact persisted task state and bounded history loading.

use std::path::Path;

pub fn history_excerpt(content: &str) -> String {
    const LIMIT: usize = 1_200;
    let cleaned = dsx_agent::brief::clean_task_input(content);
    let mut excerpt: String = cleaned.chars().take(LIMIT).collect();
    if cleaned.chars().count() > LIMIT {
        excerpt.push_str("...");
    }
    excerpt
}

pub async fn record_task_started(active_root: &Path, task: &str) -> anyhow::Result<()> {
    let db_path = active_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    let project_root = active_root.display().to_string();
    let mut summary = dsx_memory::load_task_summary(&pool, &project_root)
        .await?
        .unwrap_or_else(|| dsx_memory::TaskSummary::new(&project_root));

    summary.goal = history_excerpt(task);
    summary.done = "Task accepted; no tool result yet.".into();
    summary.plan = default_plan();
    summary.last_changes = "Starting a new scoped run.".into();
    summary.next_step = "Collect active-scope context and execute the first safe step.".into();
    summary.active_scope = project_root;
    summary.constraints = default_constraints();
    summary.architecture = architecture_outline(active_root);
    summary.scope_violations = 0;
    summary.last_scope_violation.clear();
    summary.updated_at.clear();

    dsx_memory::upsert_task_summary(&pool, &summary).await
}

pub async fn record_task_finished(
    active_root: &Path,
    brief: &dsx_tui::TaskBriefPanel,
    tools: &[dsx_tui::ToolTimelineEntry],
    scope_violations: u64,
    last_scope_violation: &str,
) -> anyhow::Result<()> {
    let db_path = active_root.join(".dsx").join("sessions.db");
    let pool = dsx_memory::open(&db_path).await?;
    let project_root = active_root.display().to_string();
    let mut summary = dsx_memory::load_task_summary(&pool, &project_root)
        .await?
        .unwrap_or_else(|| dsx_memory::TaskSummary::new(&project_root));

    summary.goal = brief.goal.clone();
    summary.done = brief.done.clone();
    summary.plan = brief.plan.clone();
    summary.last_changes = latest_tool_summary(brief, tools);
    summary.next_step = brief.next_step.clone();
    summary.active_scope = brief.active_scope.clone();
    summary.constraints = default_constraints();
    summary.architecture = architecture_outline(active_root);
    summary.scope_violations = scope_violations;
    summary.last_scope_violation = last_scope_violation.to_string();
    summary.updated_at.clear();

    dsx_memory::upsert_task_summary(&pool, &summary).await
}

pub async fn load_folder_notes(launch_root: &Path) -> Vec<dsx_tui::FolderNote> {
    let Ok(entries) = std::fs::read_dir(launch_root) else {
        return Vec::new();
    };

    let mut dirs = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            note_candidate(&name).then_some((name, entry.path()))
        })
        .collect::<Vec<_>>();
    dirs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut notes = Vec::new();
    for (name, path) in dirs.into_iter().take(12) {
        let summary = load_child_summary(&path).await;
        notes.push(folder_note(&name, summary.as_ref()));
    }
    notes
}

async fn load_child_summary(path: &Path) -> Option<dsx_memory::TaskSummary> {
    let db_path = path.join(".dsx").join("sessions.db");
    if !db_path.exists() {
        return None;
    }
    let pool = dsx_memory::open(&db_path).await.ok()?;
    let project_root = path.display().to_string();
    dsx_memory::load_task_summary(&pool, &project_root)
        .await
        .ok()
        .flatten()
}

fn folder_note(name: &str, summary: Option<&dsx_memory::TaskSummary>) -> dsx_tui::FolderNote {
    dsx_tui::FolderNote {
        folder: format!("{name}/"),
        summary: truncate_note(summary_text(summary).unwrap_or_else(|| describe_dir(name).into())),
        next_step: truncate_note(
            summary
                .and_then(|summary| non_empty(&summary.next_step))
                .unwrap_or("No saved task state yet."),
        ),
    }
}

fn summary_text(summary: Option<&dsx_memory::TaskSummary>) -> Option<String> {
    let summary = summary?;
    if summary.scope_violations > 0 {
        return Some(format!(
            "Scope guard blocked {} escape(s): {}",
            summary.scope_violations, summary.last_scope_violation
        ));
    }
    non_empty(&summary.last_changes)
        .or_else(|| non_empty(&summary.done))
        .or_else(|| non_empty(&summary.goal))
        .map(ToOwned::to_owned)
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn note_candidate(name: &str) -> bool {
    !matches!(name, ".git" | ".dsx" | "target") && !name.starts_with('.')
}

fn latest_tool_summary(
    brief: &dsx_tui::TaskBriefPanel,
    tools: &[dsx_tui::ToolTimelineEntry],
) -> String {
    if tools.is_empty() {
        return brief.last_changes.clone();
    }

    tools
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|tool| format!("{}={}: {}", tool.name, tool.status, tool.summary))
        .collect::<Vec<_>>()
        .join("\n")
}

fn default_plan() -> String {
    "1. Stay inside active scope\n2. Inspect only needed files\n3. Apply scoped changes\n4. Verify with focused commands/tests".into()
}

fn default_constraints() -> String {
    "- Active scope is a hard boundary\n- Source files should stay at 300 lines or fewer\n- Split large code into modules/components\n- Send compact state, not full chat history".into()
}

fn architecture_outline(root: &Path) -> String {
    let Ok(entries) = std::fs::read_dir(root) else {
        return "Architecture unavailable until the active scope is readable.".into();
    };

    let mut lines = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && name != ".dsx" {
                return None;
            }
            let count = std::fs::read_dir(entry.path())
                .map(|rd| rd.count())
                .unwrap_or(0);
            Some(format!(
                "- {name}/: {}; {count} direct item(s)",
                describe_dir(&name)
            ))
        })
        .take(12)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        lines.push("- ./: active project root; inspect files only when needed".into());
    }
    lines.join("\n")
}

fn describe_dir(name: &str) -> &'static str {
    match name {
        "src" => "application entrypoints and UI/event code",
        "crates" => "workspace modules and reusable Rust crates",
        "docs" => "user-facing documentation",
        "plan" => "architecture notes, research, and roadmap material",
        "tests" => "test fixtures and integration checks",
        ".dsx" => "local DSX memory, sessions, and indexes",
        "target" => "build artifacts; normally ignored",
        _ => "project folder; open only for task-relevant details",
    }
}

fn truncate_note(value: impl AsRef<str>) -> String {
    const LIMIT: usize = 140;
    let value = value.as_ref();
    let mut text: String = value.chars().take(LIMIT).collect();
    if value.chars().count() > LIMIT {
        text.push_str("...");
    }
    text
}
