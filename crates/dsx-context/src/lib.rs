//! DSX Context Manager — assemble context for the agent loop.
//!
//! Gathers: project map, git status/diff, file summaries, memories.

use std::path::Path;

pub struct ContextManager;

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub project_root: String,
    pub git_status: String,
    pub git_diff: String,
    pub file_tree: Vec<String>,
    pub memories: Vec<String>,
    pub max_tokens: u64,
}

impl ContextManager {
    pub fn new() -> Self {
        Self
    }

    /// Collect context for a new agent task.
    pub async fn collect(
        &self,
        project_root: &Path,
        max_tokens: u64,
    ) -> anyhow::Result<AgentContext> {
        let git_status = dsx_git::status(project_root).unwrap_or_default();
        let git_diff = dsx_git::diff(project_root).unwrap_or_default();
        let file_tree = dsx_fs::list_files(project_root)?;

        // TODO: load memories from memory store

        Ok(AgentContext {
            project_root: project_root.display().to_string(),
            git_status,
            git_diff,
            file_tree,
            memories: Vec::new(),
            max_tokens,
        })
    }
}

/// Load project-specific instructions from `.deepseek-code/instructions.md`, `CLAUDE.md`, `GEMINI.md`, or `AGENTS.md`.
pub fn load_project_instructions(root: &Path) -> Option<String> {
    let candidates = [
        root.join(".deepseek-code").join("instructions.md"),
        root.join("CLAUDE.md"),
        root.join("GEMINI.md"),
        root.join("AGENTS.md"),
    ];
    for path in &candidates {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                return Some(content);
            }
        }
    }
    None
}

/// Format context as a string to include in the system prompt.
pub fn format_context(ctx: &AgentContext) -> String {
    let mut buf = String::new();
    buf.push_str(&format!("Project: {}\n", ctx.project_root));
    buf.push_str(&format!("Git branch/status: {}\n", ctx.git_status.trim()));
    if !ctx.git_diff.is_empty() {
        buf.push_str(&format!("Git diff (dirty):\n{}", ctx.git_diff));
    }
    buf.push_str("Files:\n");
    for f in &ctx.file_tree {
        buf.push_str(&format!("  {f}\n"));
    }
    buf
}
