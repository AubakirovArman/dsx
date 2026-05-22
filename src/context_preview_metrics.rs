//! Request-budget metrics for dry-run context previews.

use crate::context_preview::ContextMetrics;

pub(crate) fn context_metrics(
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
