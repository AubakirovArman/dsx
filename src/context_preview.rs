//! Dry-run model context preview without calling the provider.

use std::path::Path;

pub(crate) use crate::context_preview_output::{
    budget_line, enforce_narrow_scope, enforce_request_budget, preview_json,
};

pub(crate) struct ContextPreview {
    pub(crate) task: String,
    pub(crate) clean_task: String,
    pub(crate) launch_scope: String,
    pub(crate) active_scope: String,
    pub(crate) narrowed: bool,
    pub(crate) active_exists: bool,
    pub(crate) system_note: String,
    pub(crate) project_context: String,
    pub(crate) task_brief: String,
    pub(crate) task_parts: dsx_agent::brief::TaskBriefParts,
    pub(crate) project_instructions: Option<String>,
    pub(crate) metrics: ContextMetrics,
}

pub(crate) struct ContextMetrics {
    pub(crate) project_context_chars: usize,
    pub(crate) task_brief_chars: usize,
    pub(crate) project_instructions_chars: usize,
    pub(crate) estimated_request_tokens: u64,
    pub(crate) max_request_tokens: u64,
    pub(crate) response_cap_tokens: u32,
    pub(crate) request_budget_status: String,
}

pub async fn run_context_preview(
    project_root: &Path,
    task: &str,
    json: bool,
    check: bool,
    require_narrow: bool,
) -> anyhow::Result<()> {
    let preview = build_context_preview(project_root, task).await?;
    if json {
        println!("{}", preview_json(&preview));
    } else {
        crate::context_preview_output::print_preview(&preview, !check);
    }
    if require_narrow {
        enforce_narrow_scope(&preview)?;
    }
    if check {
        enforce_request_budget(&preview)?;
    }
    Ok(())
}

pub(crate) async fn build_context_preview(
    project_root: &Path,
    task: &str,
) -> anyhow::Result<ContextPreview> {
    let scope = dsx_agent::scope::resolve_task_scope(project_root, task)?;
    let clean_task = dsx_agent::brief::clean_task_input(task);
    let ctx = collect_preview_context(&scope.active_root).await?;
    let project_context = dsx_context::format_context(&ctx);
    let task_parts = dsx_agent::brief::build_task_brief_parts(&clean_task, &scope, &ctx);
    let task_brief = task_parts.render();
    let project_instructions = scope
        .active_root
        .exists()
        .then(|| dsx_context::load_project_instructions(&scope.active_root))
        .flatten();
    let metrics = crate::context_preview_metrics::context_metrics(
        &scope.system_note(),
        &project_context,
        &task_brief,
        project_instructions.as_deref(),
        &clean_task,
    )?;

    Ok(ContextPreview {
        task: task.into(),
        clean_task,
        launch_scope: scope.launch_root.display().to_string(),
        active_scope: scope.active_root.display().to_string(),
        narrowed: scope.narrowed,
        active_exists: scope.active_root.exists(),
        system_note: scope.system_note(),
        project_context,
        task_brief,
        task_parts,
        project_instructions,
        metrics,
    })
}

async fn collect_preview_context(active_root: &Path) -> anyhow::Result<dsx_context::AgentContext> {
    if active_root.exists() {
        return dsx_context::ContextManager::new()
            .collect(active_root, 250_000)
            .await;
    }

    Ok(dsx_context::AgentContext {
        project_root: active_root.display().to_string(),
        git_status: "active scope does not exist yet".into(),
        git_diff: String::new(),
        file_tree: Vec::new(),
        memories: Vec::new(),
        task_summary: None,
        max_tokens: 250_000,
    })
}
