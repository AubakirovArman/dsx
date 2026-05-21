//! DSX Agent — synchronous block-on runner.

use dsx_provider::types::{ChatRequest, Message, ToolCall, FunctionCall, ThinkingConfig, StreamOptions};
use dsx_provider::streaming::StreamEvent;
use crate::types::{AgentConfig, AgentOutcome, ToolResult};
use crate::build_tool_defs;

// Pricing per 1M tokens (May 2026)
const PRO_INPUT_COST: f64 = 1.74;
const PRO_OUTPUT_COST: f64 = 3.48;
const FLASH_INPUT_COST: f64 = 0.14;
const FLASH_OUTPUT_COST: f64 = 0.28;

/// Execute a natural language task and block until a final answer is returned.
pub async fn run(task: &str, config: &AgentConfig) -> anyhow::Result<AgentOutcome> {
    let project_root = &config.project_root;
    let client = dsx_provider::client::DeepSeekClient::new_with_base(config.api_key.clone(), config.api_base.clone());

    // Step 1: Classify the task
    let route = crate::classify::classify(task, &config.api_key, &config.api_base).await?;
    tracing::info!(route = ?route, "Task classified");

    // Step 2: Collect project context
    let ctx = dsx_context::ContextManager::new()
        .collect(project_root, 250_000)
        .await?;
    let context_str = dsx_context::format_context(&ctx);

    // Step 3: Build system prompt (static)
    let system_prompt = dsx_prompts::lead_agent();

    // Step 4: Set up the conversation
    let tools = build_tool_defs();
    let (model_name, thinking, effort) = dsx_provider::model_config(route);

    let mut messages: Vec<Message> = vec![
        Message {
            role: "system".into(),
            content: Some(system_prompt),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        },
        Message {
            role: "system".into(),
            content: Some(format!("Current workspace project context:\n{}", context_str)),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        },
    ];

    if let Some(instructions) = dsx_context::load_project_instructions(project_root) {
        messages.push(Message {
            role: "system".into(),
            content: Some(format!("Project-specific instructions:\n{}", instructions)),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        });
    }

    messages.push(Message {
        role: "user".into(),
        content: Some(task.to_string()),
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    });

    // Token tracking
    let mut total_prompt_tokens: u64 = 0;
    let mut total_completion_tokens: u64 = 0;
    let mut total_reasoning_tokens: u64 = 0;
    let mut all_tool_results: Vec<ToolResult> = Vec::new();
    let mut final_answer: Option<String> = None;
    let mut iterations: usize = 0;

    // ── Main ReAct loop ──────────────────────────────────────────
    for i in 0..config.max_iterations {
        iterations = i + 1;
        let request = ChatRequest {
            model: model_name.to_string(),
            messages: messages.clone(),
            stream: Some(true),
            tools: Some(tools.clone()),
            thinking: if thinking { Some(ThinkingConfig { type_: "enabled".into() }) } else { None },
            reasoning_effort: effort.map(|e| e.to_string()),
            max_tokens: Some(16384),
            stream_options: Some(StreamOptions { include_usage: true }),
        };

        // Stream and collect events
        let events = client.chat_stream_events(&request).await?;

        // Process events
        let mut reasoning = String::new();
        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut finish_calls: Vec<ToolCall> = Vec::new();
        let mut usage: Option<dsx_provider::streaming::Usage> = None;
        let mut finish_reason = String::new();

        for event in &events {
            match event {
                StreamEvent::Reasoning(r) => reasoning.push_str(r),
                StreamEvent::Content(c) => content.push_str(c),
                StreamEvent::ToolCall(tc) => {
                    let call = ToolCall {
                        id: tc.id.clone(),
                        type_: "function".into(),
                        function: FunctionCall {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    };
                    tool_calls.push(call.clone());
                    finish_calls.push(call);
                }
                StreamEvent::Finish { finish_reason: fr, usage: u } => {
                    finish_reason = fr.clone();
                    // Prefer the last non-None usage we see
                    if u.is_some() || usage.is_none() {
                        usage = u.clone();
                    }
                }
                StreamEvent::Error(err) => {
                    anyhow::bail!("Agent error: {err}");
                }
            }
        }

        // Track tokens
        if let Some(ref u) = usage {
            total_prompt_tokens = u.prompt_tokens as u64;
            total_completion_tokens = u.completion_tokens as u64;
            if let Some(rt) = u.reasoning_tokens {
                total_reasoning_tokens = rt as u64;
            }
        }

        // Commit reasoning/content to conversation history
        messages.push(Message {
            role: "assistant".into(),
            content: if content.is_empty() { None } else { Some(content.clone()) },
            tool_calls: if finish_calls.is_empty() { None } else { Some(finish_calls) },
            tool_call_id: None,
            reasoning_content: if reasoning.is_empty() { None } else { Some(reasoning) },
        });

        // Break if finished without further tool calls
        if tool_calls.is_empty() || finish_reason == "stop" {
            final_answer = Some(content);
            break;
        }

        // Execute each tool call
        let mut tool_msgs: Vec<Message> = Vec::new();
        let tool_ctx = crate::tool_executor::ToolContext {
            workspace: project_root.clone(),
            mode: config.mode,
            approval_tx: config.approval_tx.clone(),
        };
        for tc in &tool_calls {
            let call_ready = dsx_provider::streaming::ToolCallReady { id: tc.id.clone(), name: tc.function.name.clone(), arguments: tc.function.arguments.clone() };
            let result = crate::tool_executor::execute(&call_ready, &tool_ctx).await;
            all_tool_results.push(result.clone());
            tool_msgs.push(Message { role: "tool".into(), content: Some(result.content), tool_calls: None, tool_call_id: Some(tc.id.clone()), reasoning_content: None });
        }
        messages.extend(tool_msgs);
    }

    let is_pro = model_name.contains("pro");
    let (input_cost_per_m, output_cost_per_m) = if is_pro { (PRO_INPUT_COST, PRO_OUTPUT_COST) } else { (FLASH_INPUT_COST, FLASH_OUTPUT_COST) };
    let estimated_cost = (total_prompt_tokens as f64 / 1_000_000.0) * input_cost_per_m
        + (total_completion_tokens as f64 / 1_000_000.0) * output_cost_per_m;

    Ok(AgentOutcome {
        answer: final_answer,
        iterations,
        total_prompt_tokens: total_prompt_tokens,
        total_completion_tokens: total_completion_tokens,
        total_reasoning_tokens: total_reasoning_tokens,
        estimated_cost_usd: estimated_cost,
        tool_results: all_tool_results,
    })
}
