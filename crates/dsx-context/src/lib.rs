//! DSX Context Manager — assemble context for the agent loop.
//!
//! Gathers: project map, git status/diff, file summaries, memories.

use std::path::Path;

pub use dsx_memory::TaskSummary;

pub struct ContextManager;

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub project_root: String,
    pub git_status: String,
    pub git_diff: String,
    pub file_tree: Vec<String>,
    pub memories: Vec<String>,
    pub task_summary: Option<dsx_memory::TaskSummary>,
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
        let memories = load_memories(project_root).await.unwrap_or_default();
        let task_summary = load_task_summary(project_root).await.unwrap_or_default();

        Ok(AgentContext {
            project_root: project_root.display().to_string(),
            git_status,
            git_diff,
            file_tree,
            memories,
            task_summary,
            max_tokens,
        })
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
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
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(path)
        {
            return Some(content);
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
    if !ctx.memories.is_empty() {
        buf.push_str("Memories:\n");
        for memory in &ctx.memories {
            buf.push_str(&format!("  {memory}\n"));
        }
    }
    if let Some(summary) = &ctx.task_summary {
        let compact = summary.compact_text();
        if !compact.is_empty() {
            buf.push_str("Compact task state:\n");
            for line in compact.lines() {
                buf.push_str(&format!("  {line}\n"));
            }
        }
    }
    buf
}

async fn load_memories(project_root: &Path) -> anyhow::Result<Vec<String>> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let pool = dsx_memory::open(&db_path).await?;
    let project_root_str = project_root.display().to_string();
    let items = dsx_memory::recent_memory_items(&pool, &project_root_str, 20).await?;
    Ok(items
        .into_iter()
        .map(|item| {
            format!(
                "[{}:{} conf={:.2}] {}",
                item.scope, item.type_, item.confidence, item.content
            )
        })
        .collect())
}

async fn load_task_summary(project_root: &Path) -> anyhow::Result<Option<dsx_memory::TaskSummary>> {
    let db_path = project_root.join(".dsx").join("sessions.db");
    if !db_path.exists() {
        return Ok(None);
    }

    let pool = dsx_memory::open(&db_path).await?;
    let project_root_str = project_root.display().to_string();
    dsx_memory::load_task_summary(&pool, &project_root_str).await
}
