//! DSX Agent — task classifier and model router.

use dsx_core::types::ModelRoute;
use dsx_provider::types::{ChatRequest, Message, ThinkingConfig};

/// Classify a task to determine model routing.
pub async fn classify(task: &str, api_key: &str, api_base: &str) -> anyhow::Result<ModelRoute> {
    if api_key.is_empty() {
        return Ok(heuristic_classify(task));
    }

    let client = dsx_provider::client::DeepSeekClient::new_with_base(
        api_key.to_string(),
        api_base.to_string(),
    );
    let prompt = r#"You are the task classifier for DSX Code. Your job is to classify the user's coding task into one of the following model routing categories.

Categories:
- "Flash": simple explanations, summaries, code translation, or very basic single-file lookups (low complexity).
- "FlashThinking": reviewing code, basic single-file edits, or simple unit test generation.
- "ProHigh": multi-file inspection, complex logic edits, refactorings, or normal debugging.
- "ProMax": security audits, major architecture migrations, extremely deep multi-file debugging, or massive refactoring tasks.

Return JSON format:
{
  "category": "Flash" | "FlashThinking" | "ProHigh" | "ProMax",
  "reason": "short explanation of the decision"
}"#;

    let request = ChatRequest {
        model: "deepseek-v4-flash".to_string(),
        messages: vec![
            Message {
                role: "system".into(),
                content: Some(prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            Message {
                role: "user".into(),
                content: Some(task.to_string()),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ],
        stream: Some(false),
        tools: None,
        thinking: Some(ThinkingConfig {
            type_: "disabled".into(),
        }), // Flash Non-Thinking
        reasoning_effort: None,
        max_tokens: Some(500),
        stream_options: None,
    };
    crate::budget::check_request(&request)?;

    // Fail-safe HTTP request block
    let response_text = match client.chat(&request).await {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!(
                "Task classification API call failed: {e}. Falling back to heuristic routing."
            );
            return Ok(heuristic_classify(task));
        }
    };

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response_text)
        && let Some(choices) = val.get("choices").and_then(|v| v.as_array())
        && let Some(first_choice) = choices.first()
        && let Some(content) = first_choice
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(content)
        && let Some(cat) = json_val.get("category").and_then(|v| v.as_str())
    {
        match cat {
            "Flash" => return Ok(ModelRoute::Flash),
            "FlashThinking" => return Ok(ModelRoute::FlashThinking),
            "ProMax" => return Ok(ModelRoute::ProMax),
            _ => return Ok(ModelRoute::ProHigh),
        }
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response_text)
        && let Some(cat) = val.get("category").and_then(|v| v.as_str())
    {
        match cat {
            "Flash" => return Ok(ModelRoute::Flash),
            "FlashThinking" => return Ok(ModelRoute::FlashThinking),
            "ProMax" => return Ok(ModelRoute::ProMax),
            _ => return Ok(ModelRoute::ProHigh),
        }
    }

    Ok(ModelRoute::ProHigh) // safe fallback
}

pub fn heuristic_classify(task: &str) -> ModelRoute {
    let task_lower = task.to_lowercase();
    if task_lower.contains("security")
        || task_lower.contains("refactor")
        || task_lower.contains("architecture")
        || task_lower.contains("debug")
        || task_lower.contains("multi-file")
    {
        ModelRoute::ProMax
    } else if task_lower.contains("explain")
        || task_lower.contains("summary")
        || task_lower.contains("what is")
    {
        ModelRoute::Flash
    } else if task_lower.contains("review") {
        ModelRoute::FlashThinking
    } else {
        ModelRoute::ProHigh
    }
}
