//! Compact task brief and user-input cleanup.

use crate::scope::TaskScope;
use dsx_context::AgentContext;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskBriefParts {
    pub goal: String,
    pub done: String,
    pub plan: String,
    pub last_changes: String,
    pub next_step: String,
    pub active_scope: String,
    pub constraints: String,
    pub surface_architecture: String,
}

impl TaskBriefParts {
    pub fn render(&self) -> String {
        format!(
            "Compact task brief:\n\
             Goal:\n{}\n\
             Done:\n{}\n\
             Plan:\n{}\n\
             Last changes:\n{}\n\
             Next step:\n{}\n\
             Active scope:\n{}\n\
             Constraints:\n{}\n\
             Surface architecture:\n{}",
            indent(&self.goal),
            indent(&self.done),
            indent(&self.plan),
            indent(&self.last_changes),
            indent(&self.next_step),
            indent(&self.active_scope),
            indent(&self.constraints),
            indent(&self.surface_architecture)
        )
    }
}

pub fn clean_task_input(task: &str) -> String {
    let lines = task
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !is_ui_noise_line(line))
        .collect::<Vec<_>>();

    let cleaned = lines.join("\n").trim().to_string();
    if cleaned.is_empty() {
        task.trim().to_string()
    } else {
        cleaned
    }
}

pub fn build_task_brief(task: &str, scope: &TaskScope, ctx: &AgentContext) -> String {
    build_task_brief_parts(task, scope, ctx).render()
}

pub fn build_task_brief_parts(task: &str, scope: &TaskScope, ctx: &AgentContext) -> TaskBriefParts {
    let state = ctx.task_summary.as_ref();
    let files = ctx
        .file_tree
        .iter()
        .take(40)
        .map(|file| format!("  - {file}"))
        .collect::<Vec<_>>()
        .join("\n");
    let files = if files.is_empty() {
        "  - (no top-level files found)".to_string()
    } else {
        files
    };

    TaskBriefParts {
        goal: field_or(state.map(|s| s.goal.as_str()), task),
        done: field_or(
            state.map(|s| s.done.as_str()),
            "Nothing in this run yet. Update from tool results only.",
        ),
        plan: field_or(
            state.map(|s| s.plan.as_str()),
            "1. Inspect only the active task scope.\n2. Make the smallest scoped changes.\n3. Verify with focused commands/tests.",
        ),
        last_changes: field_or(
            state.map(|s| s.last_changes.as_str()),
            "Use the git status/diff below; do not infer uninspected file contents.",
        ),
        next_step: field_or(
            state.map(|s| s.next_step.as_str()),
            "Start scoped inspection.",
        ),
        active_scope: field_or(
            state.map(|s| s.active_scope.as_str()),
            &scope.active_root.display().to_string(),
        ),
        constraints: field_or(
            state.map(|s| s.constraints.as_str()),
            "- Active scope is a hard boundary.\n- Keep source files at 300 lines or fewer; split into modules/components.\n- Keep responses compact; do not repeat old conversation.",
        ),
        surface_architecture: field_or(state.map(|s| s.architecture.as_str()), &files),
    }
}

fn field_or(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn indent(value: &str) -> String {
    value
        .lines()
        .map(|line| format!("  {}", line.trim_end()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_ui_noise_line(line: &str) -> bool {
    let total = line.chars().count().max(1);
    let box_chars = line
        .chars()
        .filter(|ch| {
            matches!(
                ch,
                '╭' | '╮' | '╰' | '╯' | '─' | '│' | '├' | '┤' | '┬' | '┴'
            )
        })
        .count();
    if box_chars * 2 >= total {
        return true;
    }

    let lower = line.to_lowercase();
    (lower.contains("tokens:") || lower.contains("cost:")) && lower.contains("ctrl+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::resolve_task_scope;

    #[test]
    fn removes_tui_border_noise() {
        let raw = "╭────╮\nиспользуй /tmp/proj\n│ tokens: 0 cost: $0 Ctrl+S │\n╰────╯";
        let cleaned = clean_task_input(raw);

        assert_eq!(cleaned, "используй /tmp/proj");
    }

    #[test]
    fn brief_contains_required_sections() {
        let root = std::env::temp_dir().join("dsx_brief_scope");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let scope = resolve_task_scope(&root, "build").unwrap();
        let ctx = AgentContext {
            project_root: root.display().to_string(),
            git_status: String::new(),
            git_diff: String::new(),
            file_tree: vec!["Cargo.toml".into()],
            memories: Vec::new(),
            task_summary: None,
            max_tokens: 1000,
        };

        let brief = build_task_brief("build", &scope, &ctx);

        assert!(brief.contains("Goal:"));
        assert!(brief.contains("Done:"));
        assert!(brief.contains("Plan:"));
        assert!(brief.contains("Last changes:"));
        assert!(brief.contains("Active scope:"));
        assert!(brief.contains("300 lines"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn brief_prefers_persisted_task_state() {
        let root = std::env::temp_dir().join("dsx_brief_state");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let scope = resolve_task_scope(&root, "build").unwrap();
        let mut summary = dsx_context::TaskSummary::new(&root.display().to_string());
        summary.goal = "persisted goal".into();
        summary.done = "persisted done".into();
        summary.next_step = "persisted next".into();
        let ctx = AgentContext {
            project_root: root.display().to_string(),
            git_status: String::new(),
            git_diff: String::new(),
            file_tree: vec!["Cargo.toml".into()],
            memories: Vec::new(),
            task_summary: Some(summary),
            max_tokens: 1000,
        };

        let brief = build_task_brief("new task", &scope, &ctx);

        assert!(brief.contains("persisted goal"));
        assert!(brief.contains("persisted done"));
        assert!(brief.contains("persisted next"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn brief_parts_expose_structured_capsule_fields() {
        let root = std::env::temp_dir().join("dsx_brief_parts");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let scope = resolve_task_scope(&root, "build").unwrap();
        let ctx = AgentContext {
            project_root: root.display().to_string(),
            git_status: String::new(),
            git_diff: String::new(),
            file_tree: vec!["src/main.rs".into()],
            memories: Vec::new(),
            task_summary: None,
            max_tokens: 1000,
        };

        let parts = build_task_brief_parts("build", &scope, &ctx);

        assert_eq!(parts.goal, "build");
        assert!(parts.constraints.contains("300 lines"));
        assert!(parts.surface_architecture.contains("src/main.rs"));
        assert!(parts.render().contains("Compact task brief:"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
