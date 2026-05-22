//! Text, JSON, and enforcement helpers for dry-run context previews.

use crate::context_preview::ContextPreview;

pub(crate) fn print_preview(preview: &ContextPreview, show_budget_advice: bool) {
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
    println!("  Scope contract: tools locked to active scope");
    if !preview.narrowed {
        println!("  Scope warning: workspace-wide until a child folder is selected");
    }
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
        "scope_contract": scope_contract_json(preview),
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

fn scope_contract_json(preview: &ContextPreview) -> serde_json::Value {
    serde_json::json!({
        "launch_scope": preview.launch_scope,
        "active_scope": preview.active_scope,
        "tool_root": preview.active_scope,
        "status": if preview.narrowed { "narrowed" } else { "wide" },
        "active_exists": preview.active_exists,
        "rule": "read/write/commands are locked to active_scope",
        "warning": if preview.narrowed { "" } else { "workspace-wide until a child folder is selected" },
    })
}
