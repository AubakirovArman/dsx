//! Token and cost guardrails for agent requests.

use dsx_provider::types::ChatRequest;

const DEFAULT_MAX_REQUEST_TOKENS: u64 = 250_000;
const DEFAULT_MAX_RUN_TOKENS: u64 = 750_000;
const DEFAULT_MAX_RUN_COST_USD: f64 = 2.0;
const PRO_INPUT_COST: f64 = 1.74;
const PRO_OUTPUT_COST: f64 = 3.48;
const FLASH_INPUT_COST: f64 = 0.14;
const FLASH_OUTPUT_COST: f64 = 0.28;

pub fn check_request(request: &ChatRequest) -> anyhow::Result<u64> {
    let estimated = estimate_request_tokens(request)?;
    let max = env_u64("DSX_MAX_REQUEST_TOKENS", DEFAULT_MAX_REQUEST_TOKENS);
    if estimated > max {
        anyhow::bail!(
            "Request token budget exceeded before API call: estimated {estimated} tokens, limit {max}. Compact context or narrow the task scope."
        );
    }
    Ok(estimated)
}

pub fn check_run_usage(
    model_name: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
) -> anyhow::Result<()> {
    let total = prompt_tokens + completion_tokens;
    let max_tokens = env_u64("DSX_MAX_RUN_TOKENS", DEFAULT_MAX_RUN_TOKENS);
    if total > max_tokens {
        anyhow::bail!(
            "Run token budget exceeded: used {total} tokens, limit {max_tokens}. Stopping to prevent runaway spend."
        );
    }

    let cost = estimate_cost(model_name, prompt_tokens, completion_tokens);
    let max_cost = env_f64("DSX_MAX_RUN_COST_USD", DEFAULT_MAX_RUN_COST_USD);
    if cost > max_cost {
        anyhow::bail!(
            "Run cost budget exceeded: estimated ${cost:.4}, limit ${max_cost:.4}. Stopping to prevent runaway spend."
        );
    }
    Ok(())
}

pub fn estimate_request_tokens(request: &ChatRequest) -> anyhow::Result<u64> {
    let bytes = serde_json::to_vec(request)?.len() as u64;
    Ok((bytes / 4).max(1))
}

fn estimate_cost(model_name: &str, prompt_tokens: u64, completion_tokens: u64) -> f64 {
    let is_pro = model_name.contains("pro");
    let (input_cost, output_cost) = if is_pro {
        (PRO_INPUT_COST, PRO_OUTPUT_COST)
    } else {
        (FLASH_INPUT_COST, FLASH_OUTPUT_COST)
    };
    (prompt_tokens as f64 / 1_000_000.0) * input_cost
        + (completion_tokens as f64 / 1_000_000.0) * output_cost
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsx_provider::types::Message;

    #[test]
    fn estimates_serialized_request_size() {
        let request = ChatRequest {
            model: "deepseek-v4-pro".into(),
            messages: vec![Message {
                role: "user".into(),
                content: Some("hello".into()),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            stream: Some(true),
            tools: None,
            thinking: None,
            reasoning_effort: None,
            max_tokens: Some(32),
            stream_options: None,
        };

        assert!(estimate_request_tokens(&request).unwrap() > 1);
    }

    #[test]
    fn rejects_absurd_reported_usage() {
        let result = check_run_usage("deepseek-v4-pro", 100_000_000, 1);

        assert!(result.is_err());
    }
}
