//! Token and cost guardrails for agent requests.

use dsx_provider::types::ChatRequest;

const DEFAULT_MAX_REQUEST_TOKENS: u64 = 250_000;
const DEFAULT_MAX_RUN_TOKENS: u64 = 750_000;
const DEFAULT_MAX_RUN_COST_USD: f64 = 2.0;
const PRO_INPUT_COST: f64 = 1.74;
const PRO_OUTPUT_COST: f64 = 3.48;
const FLASH_INPUT_COST: f64 = 0.14;
const FLASH_OUTPUT_COST: f64 = 0.28;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetLimits {
    pub max_request_tokens: u64,
    pub max_run_tokens: u64,
    pub max_run_cost_usd: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub reasoning_tokens: u64,
}

impl RunUsage {
    pub fn new(prompt_tokens: u64, completion_tokens: u64, reasoning_tokens: u64) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            reasoning_tokens,
        }
    }

    pub fn total(self) -> u64 {
        self.prompt_tokens + self.completion_tokens + self.reasoning_tokens
    }
}

pub fn current_limits() -> BudgetLimits {
    BudgetLimits {
        max_request_tokens: env_u64("DSX_MAX_REQUEST_TOKENS", DEFAULT_MAX_REQUEST_TOKENS),
        max_run_tokens: env_u64("DSX_MAX_RUN_TOKENS", DEFAULT_MAX_RUN_TOKENS),
        max_run_cost_usd: env_f64("DSX_MAX_RUN_COST_USD", DEFAULT_MAX_RUN_COST_USD),
    }
}

pub fn check_request(request: &ChatRequest) -> anyhow::Result<u64> {
    let limits = current_limits();
    let estimated = check_request_against_limits(request, &limits)?;
    Ok(estimated)
}

pub fn check_request_with_usage(request: &ChatRequest, usage: RunUsage) -> anyhow::Result<u64> {
    let limits = current_limits();
    let estimated = check_request_against_limits(request, &limits)?;
    check_projected_usage(request, usage, estimated, &limits)?;
    Ok(estimated)
}

pub fn check_run_usage(
    model_name: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
    reasoning_tokens: u64,
) -> anyhow::Result<()> {
    let limits = current_limits();
    let usage = RunUsage {
        prompt_tokens,
        completion_tokens,
        reasoning_tokens,
    };
    if usage.total() > limits.max_run_tokens {
        anyhow::bail!(
            "Run token budget exceeded: used {} tokens, limit {}. Stopping to prevent runaway spend.",
            usage.total(),
            limits.max_run_tokens
        );
    }

    let cost = estimate_cost(model_name, prompt_tokens, completion_tokens);
    if cost > limits.max_run_cost_usd {
        anyhow::bail!(
            "Run cost budget exceeded: estimated ${cost:.4}, limit ${:.4}. Stopping to prevent runaway spend.",
            limits.max_run_cost_usd
        );
    }
    Ok(())
}

pub fn format_limits(limits: BudgetLimits) -> String {
    format!(
        "req<={}k run<={}k cost<=${:.2}",
        limits.max_request_tokens / 1_000,
        limits.max_run_tokens / 1_000,
        limits.max_run_cost_usd
    )
}

pub fn estimate_run_cost(model_name: &str, usage: RunUsage) -> f64 {
    estimate_cost(model_name, usage.prompt_tokens, usage.completion_tokens)
}

pub fn estimate_request_tokens(request: &ChatRequest) -> anyhow::Result<u64> {
    let bytes = serde_json::to_vec(request)?.len() as u64;
    Ok((bytes / 4).max(1))
}

fn check_request_against_limits(
    request: &ChatRequest,
    limits: &BudgetLimits,
) -> anyhow::Result<u64> {
    let estimated = estimate_request_tokens(request)?;
    if estimated > limits.max_request_tokens {
        anyhow::bail!(
            "Request token budget exceeded before API call: estimated {estimated} tokens, limit {}. Compact context or narrow the task scope.",
            limits.max_request_tokens
        );
    }
    Ok(estimated)
}

fn check_projected_usage(
    request: &ChatRequest,
    usage: RunUsage,
    estimated_request: u64,
    limits: &BudgetLimits,
) -> anyhow::Result<()> {
    let response_cap = request.max_tokens.unwrap_or(0) as u64;
    let projected = usage.total() + estimated_request + response_cap;
    if projected > limits.max_run_tokens {
        anyhow::bail!(
            "Projected run token budget exceeded before API call: current {} + request estimate {estimated_request} + response cap {response_cap} = {projected}, limit {}. Narrow the task or lower max output.",
            usage.total(),
            limits.max_run_tokens
        );
    }
    Ok(())
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
        let result = check_run_usage("deepseek-v4-pro", 100_000_000, 1, 0);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_absurd_reasoning_usage() {
        let result = check_run_usage("deepseek-v4-pro", 1, 1, 100_000_000);

        assert!(result.is_err());
    }

    #[test]
    fn projected_usage_accounts_for_response_cap() {
        let request = small_request(Some(500));
        let limits = BudgetLimits {
            max_request_tokens: 20_000,
            max_run_tokens: 1_000,
            max_run_cost_usd: 10.0,
        };
        let estimated = check_request_against_limits(&request, &limits).unwrap();
        let result = check_projected_usage(
            &request,
            RunUsage {
                prompt_tokens: 400,
                completion_tokens: 100,
                reasoning_tokens: 0,
            },
            estimated,
            &limits,
        );

        assert!(result.is_err());
    }

    fn small_request(max_tokens: Option<u32>) -> ChatRequest {
        ChatRequest {
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
            max_tokens,
            stream_options: None,
        }
    }
}
