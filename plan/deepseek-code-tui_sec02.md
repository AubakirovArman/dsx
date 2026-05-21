## 1. Confirmed DeepSeek V4 API Capabilities

This chapter documents the API capabilities of DeepSeek V4 that are confirmed through official documentation and integration testing. Every claim below is sourced from the official DeepSeek API documentation, SDK examples, or verified integration reports. Capabilities marked as speculative or unconfirmed are explicitly noted as such.

### 1.1 Model Specifications

DeepSeek V4 ships as two models: V4 Pro (full capability) and V4 Flash (cost-optimized). Both share the same Mixture-of-Experts (MoE) architecture, the same 1-million-token context window, and the same MIT license. The difference lies in parameter scale and per-token pricing.

**Table 1: DeepSeek V4 Model Specifications**

| Attribute | V4 Pro | V4 Flash | Source |
|---|---|---|---|
| Total parameters | 1.6 trillion | 284 billion | Official model card, Hugging Face weights [^1^] |
| Active parameters (per forward pass) | 49 billion | 13 billion | Official model card [^1^] |
| Context window | 1,048,576 tokens | 1,048,576 tokens | API reference, verified by integration tests [^2^] |
| Max output tokens | 384,000 (default 4,096; 8,096 on beta endpoint) | Same as Pro | API reference [^2^] |
| Input price (per 1M tokens, list) | $1.74 | $0.14 | Official pricing page [^3^] |
| Output price (per 1M tokens, list) | $3.48 | $0.28 | Official pricing page [^3^] |
| Cache hit price (per 1M tokens) | $0.003625 | $0.0028 | Pricing page, post-2026-04-26 reduction [^4^] |
| License | MIT | MIT | Hugging Face repository [^1^] |
| Weights availability | Hugging Face (full) | Hugging Face (full) | Hugging Face [^1^] |
| Architecture | Mixture-of-Experts (MoE) | Mixture-of-Experts (MoE) | Model card [^1^] |

The 1.6-trillion-parameter Pro model activates only 49 billion parameters per forward pass, a 33:1 sparsity ratio that keeps inference latency manageable while preserving the representational capacity of the full parameter set. The Flash model offers a leaner 22:1 ratio (284B total, 13B active), trading some reasoning depth for roughly 12x lower input cost. Both models expose identical API surfaces; the model name string passed to the API endpoint is the sole difference in client code.

Both models carry the MIT license, and full weights are available on Hugging Face. This means the models can be self-hosted for air-gapped or compliance-sensitive environments, though this chapter focuses exclusively on the managed API.

#### 1.1.1 On Context Window and Effective Use

The 1M token context window applies to the combined input-plus-output length of a single request. In practice, the effective input budget is the 1M total minus the requested `max_tokens` for output. For a coding agent that streams long outputs, setting `max_tokens=384000` leaves approximately 664K tokens for the input context — sufficient for large codebases, conversation history, and system prompt combined. The API does not enforce a separate input/output split; exhaustion of the 1M pool in either direction triggers a `length` finish reason.

### 1.2 Core API Features

The V4 API implements the full Chat Completions schema from OpenAI, with DeepSeek-specific extensions for reasoning mode, context caching, and tool calling. The following table summarizes the confirmed feature set.

**Table 2: Confirmed API Features and Parameters**

| Feature | Status | Key Parameters / Notes | Source |
|---|---|---|---|
| Tool calling (function calling) | Confirmed | `tools` array, max 128 functions; `tool_choice`: `none`/`auto`/`required`/specific function | API reference [^2^] |
| Parallel tool calls | Confirmed | Model returns `tool_calls` array with multiple items in a single response | Tool calling guide [^5^] |
| Strict tool mode (Beta) | Confirmed | `strict: true` on each function; requires `base_url: https://api.deepseek.com/beta` | Tool calling guide [^5^] |
| Thinking mode | Confirmed, ON by default | `thinking.type`: `"enabled"` (default) or `"disabled"`; `reasoning_effort`: `"high"` or `"max"` | Thinking mode guide [^6^] |
| Streaming | Confirmed | SSE format, `data: [DONE]` terminator; reasoning tokens in `delta.reasoning_content` before `delta.content` | API reference, SDK examples [^2^] [^6^] |
| Usage in streaming | Confirmed | `stream_options={"include_usage": true}` adds usage chunk before `[DONE]` | API reference [^2^] |
| JSON mode | Confirmed | `response_format={"type": "json_object"}`; prompt must contain the word "json" | JSON mode guide [^7^] |
| Context caching | Confirmed, automatic | No client parameter required; `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens` in usage | Caching guide [^4^] |
| OpenAI-compatible endpoint | Confirmed | `POST https://api.deepseek.com/chat/completions` | Quickstart [^8^] |
| Anthropic-compatible endpoint | Confirmed | `POST https://api.deepseek.com/anthropic` | Anthropic API guide [^9^] |
| Temperature / top_p | Accepted but ignored in thinking mode | `temperature`: 0–2 (default 1); `top_p`: 0–1 (default 1) | Thinking mode guide [^6^] |
| Presence / frequency penalty | Deprecated, no effect | Parameters accepted for backward compatibility | API reference [^2^] |

#### 1.2.1 Tool Calling

Tool calling follows the OpenAI function-calling schema exactly. A request may include up to 128 function definitions in the `tools` array, each with `type: "function"`, a `name`, a `description`, and a `parameters` object in JSON Schema format. The `tool_choice` parameter supports four modes: `none` (model will not call any tool), `auto` (model decides), `required` (model must call at least one tool), and a specific function selector `{"type": "function", "function": {"name": "..."}}` to force a particular tool. The model can return multiple tool calls in a single response via the `tool_calls` array, enabling parallel execution of independent operations.

A beta "strict" mode is available via the `https://api.deepseek.com/beta` base URL. When `strict: true` is set on a function definition, the API validates that the model's arguments conform to the declared JSON Schema. Strict mode requires all object properties to be listed in `required` and `additionalProperties` to be `false`. Supported schema types are `object`, `string`, `number`, `integer`, `boolean`, `array`, `enum`, and `anyOf` [^5^].

#### 1.2.2 Thinking Mode

Thinking mode is **enabled by default**. If the client does not specify `thinking.type`, the API treats it as `"enabled"` and enters reasoning mode. To disable thinking — for example, when using fill-in-the-middle (FIM) completion — the client must explicitly send `thinking: {"type": "disabled"}`.

When thinking is enabled, the API accepts a `reasoning_effort` parameter with values `"high"` (default for regular requests) or `"max"` (used automatically for complex agent patterns). For compatibility with existing code that may pass other providers' effort values, `"low"` and `"medium"` are silently mapped to `"high"`, and `"xhigh"` is mapped to `"max"` [^6^].

Thinking mode silently ignores `temperature`, `top_p`, `presence_penalty`, and `frequency_penalty`. The API accepts these parameters without error but produces no sampling variation. This behavior is by design: reasoning chains are generated deterministically. Engineers who require temperature control must disable thinking mode first.

When tool calls are used in thinking mode, a critical constraint applies: the `reasoning_content` from every assistant turn that performed a tool call **must** be passed back to the API in all subsequent requests. Omitting it produces HTTP 400 with the error message: `"The reasoning_content in the thinking mode must be passed back to the API."` For multi-turn conversations without tool calls, `reasoning_content` may be omitted; the API ignores it [^6^].

#### 1.2.3 Streaming

Streaming follows the Server-Sent Events (SSE) protocol. Each chunk is a `data: <json>` line; the stream terminates with `data: [DONE]`. In thinking mode, the streaming sequence is strictly ordered:

1. First chunk: `delta.role = "assistant"` only.
2. Subsequent chunks: reasoning tokens arrive via `delta.reasoning_content`.
3. After reasoning completes: answer tokens arrive via `delta.content`.
4. Final chunk: `finish_reason` and `usage` object, including `completion_tokens_details.reasoning_tokens`.

To receive usage statistics in streaming mode, set `stream_options={"include_usage": true}`. The API then emits an additional chunk before `[DONE]` containing the full `usage` object with `prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`, and the reasoning-token breakdown [^2^].

#### 1.2.4 JSON Mode

JSON mode is activated by setting `response_format={"type": "json_object"}`. The API guarantees syntactically valid JSON output, but the prompt must explicitly instruct the model to emit JSON — the official documentation requires the word "json" to appear in the system or user prompt, along with an example of the desired JSON structure [^7^]. An acknowledged limitation: JSON mode may occasionally return empty content. The documentation attributes this to known optimization gaps and recommends prompt modification as a workaround.

### 1.3 API Endpoints and Migration

DeepSeek V4 offers two first-party API interfaces: an OpenAI-compatible endpoint and an Anthropic-compatible endpoint. Both expose the same underlying model; the choice depends on which SDK the client code already uses.

The OpenAI-compatible endpoint uses `base_url = "https://api.deepseek.com"` with the standard `/chat/completions` path. Model names are `deepseek-v4-pro` and `deepseek-v4-flash`. The thinking-mode parameter `thinking` is not part of the OpenAI SDK schema, so it must be passed via `extra_body={"thinking": {"type": "enabled"}}` when using the OpenAI SDK. The `reasoning_effort` parameter is recognized natively by the SDK and can be passed at the top level [^8^] [^6^].

The Anthropic-compatible endpoint uses `base_url = "https://api.deepseek.com/anthropic"`. Thinking effort is controlled via `output_config: {"effort": "high"}` rather than `reasoning_effort`. Unsupported model names are automatically mapped to `deepseek-v4-flash`. The `anthropic-beta` HTTP header is ignored by the endpoint [^9^].

A third base URL, `https://api.deepseek.com/beta`, enables beta features: strict tool mode, chat prefix completion, and an increased `max_tokens` limit of 8,192 (versus the default 4,096). Beta features are not available on the standard endpoint [^5^].

#### 1.3.1 Legacy Model Deprecation

The model aliases `deepseek-chat` and `deepseek-reasoner` are deprecated and will be fully removed on **July 24, 2026**. At present, `deepseek-chat` resolves to V4 Flash in non-thinking mode, and `deepseek-reasoner` resolves to V4 Flash in thinking mode. New code should use the explicit model names `deepseek-v4-pro` or `deepseek-v4-flash` [^8^].

### 1.4 Cost Optimization

#### 1.4.1 Context Caching

Context caching is automatic and requires no client-side configuration. The API builds a disk-based key-value cache for each request. Subsequent requests with matching prefixes fetch the overlapping portion from cache at a 10x reduced price [^4^].

Cache prefix units are created at three points: at the end of each user input, at the end of each model output, and at fixed token intervals for long inputs. The system also detects common prefixes across multiple requests and persists them as independent cache units. Critically, the matching algorithm requires **full** prefix-unit alignment; partial matches do not qualify as cache hits [^4^].

Cache status is reported in the response `usage` field as two counters: `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens`. Their sum equals `prompt_tokens`. The cache operates on a best-effort basis — DeepSeek does not guarantee a 100% hit rate, and unused cache entries are evicted within hours to days [^4^].

For a coding agent that sends a stable system prompt and project context followed by changing task instructions, the natural prefix stability of the conversation structure should yield significant cache hits on the system and context portions. Engineers should monitor `prompt_cache_hit_tokens` in production to validate cache effectiveness.

#### 1.4.2 Launch Discount

DeepSeek applies a 75% launch discount to V4 Pro input and output pricing, extending it to May 31, 2026. This reduces the effective Pro input price from $1.74 to $0.435 per million tokens, and the output price from $3.48 to $0.87 per million tokens. The Flash model prices ($0.14 input, $0.28 output) are not discounted and serve as the baseline. Engineers should verify current pricing on the official pricing page before budgeting, as discount extensions are announced without guaranteed notice periods [^3^].

#### 1.4.3 Cost Comparison

**Table 3: Input/Output Pricing Comparison (per 1M tokens)**

| Provider / Model | Input Price | Output Price | Cache Hit Price | Context Window | Source |
|---|---|---|---|---|---|
| DeepSeek V4 Pro | $0.435 (discounted) / $1.74 (list) | $0.87 (discounted) / $3.48 (list) | $0.003625 | 1M | Official pricing [^3^] [^4^] |
| DeepSeek V4 Flash | $0.14 | $0.28 | $0.0028 | 1M | Official pricing [^3^] [^4^] |
| Claude 4 Sonnet (Anthropic) | ~$3.00 | ~$15.00 | Not offered | 200K | Anthropic pricing page (public) |
| Claude 4 Opus (Anthropic) | ~$15.00 | ~$75.00 | Not offered | 200K | Anthropic pricing page (public) |
| GPT-4.1 (OpenAI) | ~$2.00 | ~$8.00 | 50% discount | 1M | OpenAI pricing page (public) |
| Gemini 2.5 Pro (Google) | ~$1.25 | ~$10.00 | Not offered | 1M | Google AI pricing (public) |

The pricing gap between DeepSeek V4 Pro and competing frontier models is substantial. At the discounted rate, V4 Pro input costs roughly one-seventh of Claude 4 Sonnet and one-fifth of GPT-4.1. V4 Flash is cheaper still — its $0.14/M input price is within an order of magnitude of Claude 3.5 Haiku and GPT-4o-mini, while offering a 1M context window and tool-calling capability comparable to full-scale frontier models. The automatic context caching further reduces effective costs for conversation-heavy workloads: a coding session where 80% of the input hits cache would see an effective Pro input price of approximately $(0.20 \times 0.435 + 0.80 \times 0.003625) = $0.09$ per million tokens, a 99.4% reduction versus the list price.

The trade-off is rate-limiting policy. Where OpenAI and Anthropic publish tier-based RPM (requests per minute) and TPM (tokens per minute) limits that scale with spend, DeepSeek uses dynamic concurrency limits based on server load. When the limit is reached, the API returns HTTP 429 immediately. Requests may also queue on an open connection with SSE keep-alive signals for up to 10 minutes before the server closes the connection. There are no published fixed limits, and no paid tier to unlock higher concurrency [^10^]. Engineers should implement exponential backoff with jitter and consider fallback to alternative providers for high-availability workloads.

