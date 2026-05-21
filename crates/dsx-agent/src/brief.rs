//! Compact task brief and user-input cleanup.

use crate::scope::TaskScope;
use dsx_context::AgentContext;

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

    format!(
        "Compact task brief:\n\
         Goal:\n  {task}\n\
         Done:\n  Nothing in this run yet. Update from tool results only.\n\
         Plan:\n  1. Inspect only the active task scope.\n  2. Make the smallest scoped changes.\n  3. Verify with focused commands/tests.\n\
         Last changes:\n  Use the git status/diff below; do not infer uninspected file contents.\n\
         Active scope:\n  {}\n\
         Constraints:\n  - Active scope is a hard boundary.\n  - Keep source files at 300 lines or fewer; split into modules/components.\n  - Keep responses compact; do not repeat old conversation.\n\
         Surface architecture:\n{}",
        scope.active_root.display(),
        files
    )
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
}
