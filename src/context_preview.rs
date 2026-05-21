//! Dry-run model context preview without calling the provider.

use std::path::Path;

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
        print_preview(&preview, !check);
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
    let task_brief = dsx_agent::brief::build_task_brief(&clean_task, &scope, &ctx);
    let project_instructions = scope
        .active_root
        .exists()
        .then(|| dsx_context::load_project_instructions(&scope.active_root))
        .flatten();
    let metrics = context_metrics(
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

fn print_preview(preview: &ContextPreview, show_budget_advice: bool) {
    println!("Context preview:");
    println!("  Task: {}", crate::handlers::task_preview(&preview.task));
    println!(
        "  Clean task: {}",
        crate::handlers::task_preview(&preview.clean_task)
    );
    println!("  Launch: {}", preview.launch_scope);
    println!("  Active: {}", preview.active_scope);
    println!(
        "  Status: {}",
        if preview.narrowed { "NARROWED" } else { "WIDE" }
    );
    println!(
        "  Active exists: {}",
        if preview.active_exists { "yes" } else { "no" }
    );
    println!(
        "  Request estimate: {} tokens / {} ({})",
        preview.metrics.estimated_request_tokens,
        preview.metrics.max_request_tokens,
        preview.metrics.request_budget_status
    );
    println!("  Capsule budget: {}", budget_line(preview));
    println!(
        "  Context chars: project={} brief={} instructions={}",
        preview.metrics.project_context_chars,
        preview.metrics.task_brief_chars,
        preview.metrics.project_instructions_chars
    );
    if show_budget_advice && preview.metrics.request_budget_status == "over" {
        println!(
            "\nBudget advice:\n{}\n",
            crate::context_budget_advice::budget_advice(preview)
        );
    }
    println!("\nSystem scope note:\n{}\n", preview.system_note);
    println!(
        "{}\n",
        dsx_agent::prompt::context_capsule(&preview.task_brief)
    );
    if let Some(instructions) = &preview.project_instructions {
        println!("Project-specific instructions:\n{}\n", instructions);
    }
    println!("Project context:\n{}", preview.project_context);
}

pub(crate) fn preview_json(preview: &ContextPreview) -> serde_json::Value {
    serde_json::json!({
        "task": preview.task,
        "clean_task": preview.clean_task,
        "launch_scope": preview.launch_scope,
        "active_scope": preview.active_scope,
        "narrowed": preview.narrowed,
        "active_exists": preview.active_exists,
        "system_note": preview.system_note,
        "task_brief": preview.task_brief,
        "context_capsule": dsx_agent::prompt::context_capsule(&preview.task_brief),
        "budget_advice": crate::context_budget_advice::budget_advice(preview),
        "project_instructions": preview.project_instructions,
        "project_context": preview.project_context,
        "metrics": {
            "project_context_chars": preview.metrics.project_context_chars,
            "task_brief_chars": preview.metrics.task_brief_chars,
            "project_instructions_chars": preview.metrics.project_instructions_chars,
            "estimated_request_tokens": preview.metrics.estimated_request_tokens,
            "max_request_tokens": preview.metrics.max_request_tokens,
            "response_cap_tokens": preview.metrics.response_cap_tokens,
            "request_budget_status": preview.metrics.request_budget_status,
        },
    })
}

pub(crate) fn enforce_request_budget(preview: &ContextPreview) -> anyhow::Result<()> {
    let estimated = preview.metrics.estimated_request_tokens;
    let limit = preview.metrics.max_request_tokens;
    if estimated > limit {
        anyhow::bail!(
            "{}",
            crate::context_budget_advice::over_budget_error(preview)
        );
    }
    Ok(())
}

pub(crate) fn budget_line(preview: &ContextPreview) -> String {
    format!(
        "capsule request ~{} / {} tokens ({})",
        preview.metrics.estimated_request_tokens,
        preview.metrics.max_request_tokens,
        preview.metrics.request_budget_status
    )
}

pub(crate) fn enforce_narrow_scope(preview: &ContextPreview) -> anyhow::Result<()> {
    if !preview.narrowed {
        anyhow::bail!(
            "Context preview stayed on the launch workspace. Add an explicit child folder like ./1234 or remove --require-narrow for an intentional workspace-wide task."
        );
    }
    Ok(())
}

fn context_metrics(
    system_note: &str,
    project_context: &str,
    task_brief: &str,
    project_instructions: Option<&str>,
    clean_task: &str,
) -> anyhow::Result<ContextMetrics> {
    let response_cap_tokens = 16_384;
    let estimated_request_tokens = estimate_start_request_tokens(
        system_note,
        project_context,
        task_brief,
        project_instructions,
        clean_task,
        response_cap_tokens,
    )?;
    let limits = dsx_agent::budget::current_limits();
    Ok(ContextMetrics {
        project_context_chars: project_context.chars().count(),
        task_brief_chars: task_brief.chars().count(),
        project_instructions_chars: project_instructions
            .map(|value| value.chars().count())
            .unwrap_or(0),
        estimated_request_tokens,
        max_request_tokens: limits.max_request_tokens,
        response_cap_tokens,
        request_budget_status: if estimated_request_tokens <= limits.max_request_tokens {
            "ok".into()
        } else {
            "over".into()
        },
    })
}

fn estimate_start_request_tokens(
    system_note: &str,
    project_context: &str,
    task_brief: &str,
    project_instructions: Option<&str>,
    clean_task: &str,
    response_cap_tokens: u32,
) -> anyhow::Result<u64> {
    use dsx_provider::types::ChatRequest;

    let messages = dsx_agent::prompt::build_start_messages(
        dsx_prompts::lead_agent(),
        system_note,
        project_context,
        task_brief,
        project_instructions,
        clean_task,
    );

    let request = ChatRequest {
        model: "deepseek-v4-pro".into(),
        messages,
        stream: Some(true),
        tools: Some(dsx_agent::build_tool_defs()),
        thinking: None,
        reasoning_effort: None,
        max_tokens: Some(response_cap_tokens),
        stream_options: None,
    };
    dsx_agent::budget::estimate_request_tokens(&request)
}
