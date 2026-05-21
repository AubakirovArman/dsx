//! DSX Agent — ReAct-based orchestration and streaming execution.

pub mod types;
pub mod classify;
pub mod tool_executor;
pub mod tool_implementations;
pub mod tool_executor_tests;
pub mod runner_sync;

pub use types::{ToolResult, AgentConfig, AgentOutcome, ApprovalRequest};
pub use classify::{classify, heuristic_classify};
pub use runner_sync::run;

use tokio::sync::mpsc;
use dsx_provider::types::{ChatRequest, Message, ToolCall, FunctionCall, ToolDef, FunctionDef, ThinkingConfig, StreamOptions};
use dsx_provider::streaming::StreamEvent;

// Pricing per 1M tokens (May 2026)
const PRO_INPUT_COST: f64 = 1.74;
const PRO_OUTPUT_COST: f64 = 3.48;
const FLASH_INPUT_COST: f64 = 0.14;
const FLASH_OUTPUT_COST: f64 = 0.28;

/// ReAct agent loop with streaming events sent to the TUI via `tx`.
/// Wraps execution to capture and send any startup or API errors to the TUI defensively.
pub async fn run_streaming(
    task: &str,
    config: &AgentConfig,
    tx: mpsc::UnboundedSender<StreamEvent>,
) -> anyhow::Result<AgentOutcome> {
    let result = run_streaming_internal(task, config, tx.clone()).await;
    match result {
        Ok(ref outcome) => {
            let _ = tx.send(StreamEvent::Done {
                answer: outcome.answer.clone().unwrap_or_default(),
                iterations: outcome.iterations,
                tokens: outcome.total_prompt_tokens + outcome.total_completion_tokens,
                cost: outcome.estimated_cost_usd,
            });
        }
        Err(ref e) => {
            let _ = tx.send(StreamEvent::Error(e.to_string()));
        }
    }
    result
}

async fn run_streaming_internal(
    task: &str,
    config: &AgentConfig,
    tx: mpsc::UnboundedSender<StreamEvent>,
) -> anyhow::Result<AgentOutcome> {
    let project_root = &config.project_root;
    let client = dsx_provider::client::DeepSeekClient::new_with_base(config.api_key.clone(), config.api_base.clone());
    let route = classify(task, &config.api_key, &config.api_base).await?;
    let ctx = dsx_context::ContextManager::new().collect(project_root, 250_000).await?;
    let context_str = dsx_context::format_context(&ctx);
    let system_prompt = dsx_prompts::lead_agent();
    let tools = build_tool_defs();
    let (model_name, thinking, effort) = dsx_provider::model_config(route);

    let mut messages: Vec<Message> = vec![
        Message { role: "system".into(), content: Some(system_prompt), tool_calls: None, tool_call_id: None, reasoning_content: None },
        Message { role: "system".into(), content: Some(format!("Current workspace project context:\n{}", context_str)), tool_calls: None, tool_call_id: None, reasoning_content: None },
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

    messages.push(Message { role: "user".into(), content: Some(task.to_string()), tool_calls: None, tool_call_id: None, reasoning_content: None });

    let mut total_prompt: u64 = 0;
    let mut total_completion: u64 = 0;
    let mut total_reasoning: u64 = 0;
    let mut all_tool_results: Vec<ToolResult> = Vec::new();
    let mut final_answer: Option<String> = None;
    let mut iterations: usize = 0;

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

        // Use callback-based streaming — events go to TUI in real-time
        let tx_clone = tx.clone();
        let mut events_buf: Vec<StreamEvent> = Vec::new();
        client.chat_stream_callback(&request, |ev| {
            let _ = tx_clone.send(ev.clone());
            events_buf.push(ev);
        }).await?;
        let events = events_buf;

        // Process (same logic as run())
        let mut reasoning = String::new();
        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut finish_calls: Vec<ToolCall> = Vec::new();
        let mut usage: Option<dsx_provider::streaming::Usage> = None;

        for event in &events {
            match event {
                StreamEvent::Reasoning(r) => reasoning.push_str(r),
                StreamEvent::Content(c) => content.push_str(c),
                StreamEvent::ToolCall(tc) => {
                    let call = ToolCall { id: tc.id.clone(), type_: "function".into(), function: FunctionCall { name: tc.name.clone(), arguments: tc.arguments.clone() } };
                    tool_calls.push(call.clone());
                    finish_calls.push(call);
                }
                StreamEvent::Finish { finish_reason: _, usage: u } => {
                    if u.is_some() || usage.is_none() { usage = u.clone(); }
                }
                StreamEvent::Error(err) => {
                    anyhow::bail!("Agent error: {err}");
                }
                StreamEvent::Done { .. } => {}
            }
        }

        if let Some(ref u) = usage {
            total_prompt += u.prompt_tokens as u64;
            total_completion += u.completion_tokens as u64;
            if let Some(rt) = u.reasoning_tokens {
                total_reasoning = rt as u64;
            }
        }

        messages.push(Message {
            role: "assistant".into(),
            content: if content.is_empty() { None } else { Some(content.clone()) },
            tool_calls: if finish_calls.is_empty() { None } else { Some(finish_calls) },
            tool_call_id: None,
            reasoning_content: if reasoning.is_empty() { None } else { Some(reasoning) },
        });

        let is_last = i + 1 >= config.max_iterations;
        if tool_calls.is_empty() {
            final_answer = Some(content);
            break;
        }

        let tool_ctx = tool_executor::ToolContext {
            workspace: project_root.clone(),
            mode: config.mode,
            approval_tx: config.approval_tx.clone(),
        };
        let mut tool_msgs: Vec<Message> = Vec::new();
        for tc in &tool_calls {
            let call_ready = dsx_provider::streaming::ToolCallReady { id: tc.id.clone(), name: tc.function.name.clone(), arguments: tc.function.arguments.clone() };
            let result = tool_executor::execute(&call_ready, &tool_ctx).await;
            all_tool_results.push(result.clone());
            if !is_last {
                tool_msgs.push(Message { role: "tool".into(), content: Some(result.content), tool_calls: None, tool_call_id: Some(tc.id.clone()), reasoning_content: None });
            }
        }
        if is_last { break; }
        messages.extend(tool_msgs);
    }

    let is_pro = model_name.contains("pro");
    let (input_cost_per_m, output_cost_per_m) = if is_pro { (PRO_INPUT_COST, PRO_OUTPUT_COST) } else { (FLASH_INPUT_COST, FLASH_OUTPUT_COST) };
    let estimated_cost = (total_prompt as f64 / 1_000_000.0) * input_cost_per_m
        + (total_completion as f64 / 1_000_000.0) * output_cost_per_m;

    Ok(AgentOutcome {
        answer: final_answer,
        iterations,
        total_prompt_tokens: total_prompt,
        total_completion_tokens: total_completion,
        total_reasoning_tokens: total_reasoning,
        estimated_cost_usd: estimated_cost,
        tool_results: all_tool_results,
    })
}

pub fn build_tool_defs() -> Vec<ToolDef> {
    dsx_tools::ToolRegistry::builtin_specs()
        .into_iter()
        .map(|spec| ToolDef {
            type_: "function".into(),
            function: FunctionDef {
                name: spec.name.clone(),
                description: spec.description.clone(),
                parameters: spec.parameters.clone(),
            },
        })
        .collect()
}
