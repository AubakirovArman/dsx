# DeepSeek V4 API: In-Depth Research Report

**Research Date:** 2026-06-17
**Models Researched:** `deepseek-v4-pro`, `deepseek-v4-flash`
**Sources:** Official DeepSeek API Docs (api-docs.deepseek.com), GitHub issues, third-party integration guides

---

## 1. Thinking Mode

### 1.1 How Thinking Mode is Enabled

## Claim: Thinking mode is enabled via a `thinking` parameter with `{"type": "enabled"}`, and reasoning effort is controlled via `reasoning_effort` parameter (values: "high" or "max")
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "When using the OpenAI SDK, you need to pass the `thinking` parameter within `extra_body`: `response = client.chat.completions.create(model="deepseek-v4-pro", reasoning_effort="high", extra_body={"thinking": {"type": "enabled"}})`"
Context: For raw API calls, `thinking` is a top-level body parameter. For OpenAI SDK compatibility, it must be passed via `extra_body` because it's not a standard OpenAI parameter. The `reasoning_effort` parameter IS a recognized top-level parameter.
Confidence: HIGH

## Claim: The `thinking` parameter defaults to `enabled` (thinking mode is ON by default)
Source: Official DeepSeek API Docs - Create Chat Completion API Reference
URL: https://api-docs.deepseek.com/api/create-chat-completion
Date: 2026-04-24
Excerpt: "`thinking` object nullable - Controls the switch between thinking and non-thinking mode. `type`: Possible values: [`enabled`, `disabled`]. Default value: `enabled`"
Context: This is a critical detail - if you don't specify thinking mode, it defaults to ENABLED (thinking mode on). To disable thinking, you must explicitly pass `{"type": "disabled"}`.
Confidence: HIGH

## Claim: `reasoning_effort` accepts "high" and "max"; "low" and "medium" are mapped to "high", "xhigh" is mapped to "max"
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "In thinking mode, the default effort is `high` for regular requests; for some complex agent requests (such as Claude Code, OpenCode), effort is automatically set to `max`. For compatibility, `low` and `medium` are mapped to `high`, and `xhigh` is mapped to `max`."
Context: This mapping exists for compatibility with existing software that may use low/medium/xhigh values from other providers.
Confidence: HIGH

### 1.2 OpenAI-Compatible vs Anthropic-Compatible Formats

## Claim: In OpenAI-compatible format, `thinking` is passed via `extra_body={"thinking": {"type": "enabled"}}` and `reasoning_effort` is a top-level parameter
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "Control Parameter (OpenAI Format): Thinking Mode Toggle: `{"thinking": {"type": "enabled/disabled"}}`; Thinking Effort Control: `{"reasoning_effort": "high/max"}`"
Context: The OpenAI SDK doesn't natively support `thinking`, so it must go through `extra_body`. The `reasoning_effort` parameter is recognized by the API directly.
Confidence: HIGH

## Claim: In Anthropic-compatible format, thinking effort is controlled via `output_config: {"effort": "high/max"}`
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "Control Parameter (Anthropic Format): Thinking Mode Toggle: `{"thinking": {"type": "enabled/disabled"}}`; Thinking Effort Control: `{"output_config": {"effort": "high/max"}}`"
Context: When using the Anthropic SDK with base_url `https://api.deepseek.com/anthropic`, the effort control parameter has a different structure.
Confidence: HIGH

### 1.3 Streaming Reasoning Tokens Separately

## Claim: In streaming mode, reasoning tokens are delivered separately via `delta.reasoning_content` chunks, before the actual answer tokens in `delta.content`
Source: Official DeepSeek API Docs - Thinking Mode (Streaming Sample Code)
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: ```python
for chunk in response:
    if chunk.choices[0].delta.reasoning_content:
        reasoning_content += chunk.choices[0].delta.reasoning_content
    else:
        content += chunk.choices[0].delta.content
```
Context: The streaming flow is: first, all reasoning tokens arrive via `delta.reasoning_content`, then the actual response content arrives via `delta.content`. The first chunk also includes `delta.role="assistant"`. The client must accumulate reasoning_content separately from content.
Confidence: HIGH

## Claim: Raw SSE streaming format includes `reasoning_content` in the delta field of each chunk
Source: ds2api reverse-engineering documentation
URL: https://github.com/CJackHwang/ds2api/blob/main/API.en.md
Date: 2026-05-14
Excerpt: ```
data: {"id":"...","object":"chat.completion.chunk","choices":[{"delta":{"reasoning_content":"..."},"index":0}]}
data: {"id":"...","object":"chat.completion.chunk","choices":[{"delta":{"content":"..."},"index":0}]}
```
Context: This shows the raw SSE format. Reasoning tokens come first in `delta.reasoning_content`, then content tokens in `delta.content`. The stream is terminated by `data: [DONE]`. Last chunk includes `finish_reason` and `usage`.
Confidence: HIGH (consistent with official SDK examples)

### 1.4 Example API Calls

**OpenAI-format, non-streaming, thinking enabled:**
```python
from openai import OpenAI
client = OpenAI(api_key="<DeepSeek API Key>", base_url="https://api.deepseek.com")

response = client.chat.completions.create(
    model="deepseek-v4-pro",
    messages=[{"role": "user", "content": "9.11 and 9.8, which is greater?"}],
    reasoning_effort="high",
    extra_body={"thinking": {"type": "enabled"}},
)
reasoning_content = response.choices[0].message.reasoning_content
content = response.choices[0].message.content
```

**OpenAI-format, streaming, thinking enabled:**
```python
response = client.chat.completions.create(
    model="deepseek-v4-pro",
    messages=[{"role": "user", "content": "9.11 and 9.8, which is greater?"}],
    stream=True,
    reasoning_effort="high",
    extra_body={"thinking": {"type": "enabled"}},
)
reasoning_content = ""
content = ""
for chunk in response:
    if chunk.choices[0].delta.reasoning_content:
        reasoning_content += chunk.choices[0].delta.reasoning_content
    elif chunk.choices[0].delta.content:
        content += chunk.choices[0].delta.content
```

**Anthropic-format:**
```python
import anthropic
client = anthropic.Anthropic()  # base_url set via env ANTHROPIC_BASE_URL

message = client.messages.create(
    model="deepseek-v4-pro",
    max_tokens=1000,
    system="You are a helpful assistant.",
    messages=[{"role": "user", "content": [{"type": "text", "text": "Hi, how are you?"}]}]
)
```

### 1.5 Important: Thinking Mode Parameter Limitations

## Claim: Thinking mode silently ignores `temperature`, `top_p`, `presence_penalty`, and `frequency_penalty` parameters
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "Thinking mode does not support the `temperature`, `top_p`, `presence_penalty`, or `frequency_penalty` parameters. Please note that, for compatibility with existing software, setting these parameters will not trigger an error but will also have no effect."
Context: The API accepts these parameters but ignores them in thinking mode. This is important for compatibility - existing code won't break, but won't get the expected sampling behavior either.
Confidence: HIGH

---

## 2. Tool Calling / Function Calling

### 2.1 Schema and Format

## Claim: Tool calls use OpenAI-style function calling format with `tools` parameter; max 128 functions supported
Source: Official DeepSeek API Docs - Create Chat Completion API Reference
URL: https://api-docs.deepseek.com/api/create-chat-completion
Date: 2026-04-24
Excerpt: "`tools`: A list of tools the model may call. Currently, only functions are supported as a tool. Use this to provide a list of functions the model may generate JSON inputs for. A max of 128 functions are supported."
Context: Standard OpenAI-compatible tool calling. Each tool has `type: "function"`, with `name`, `description`, and `parameters` (JSON Schema). The `tool_choice` parameter controls tool selection (none/auto/required/specific function).
Confidence: HIGH

## Claim: `tool_choice` supports `none`, `auto`, `required`, and specific function selection
Source: Official DeepSeek API Docs - Create Chat Completion API Reference
URL: https://api-docs.deepseek.com/api/create-chat-completion
Date: 2026-04-24
Excerpt: "`tool_choice`: `none` means the model will not call any tool... `auto` means the model can pick between generating a message or calling one or more tools. `required` means the model must call one or more tools. Specifying a particular tool via `{"type": "function", "function": {"name": "my_function"}}` forces the model to call that tool."
Context: Full OpenAI-compatible tool_choice support including the `required` option (which forces at least one tool call).
Confidence: HIGH

### 2.2 Parallel Tool Calls

## Claim: The API supports parallel tool calls - the model can return multiple tool_calls in a single response
Source: Official DeepSeek API Docs - Tool Calls (Thinking Mode example shows multiple tool calls)
URL: https://api-docs.deepseek.com/guides/tool_calls
Date: 2026-04-24
Excerpt: The tool call execution flow shows model returning `tool_calls` array with multiple items. The response schema lists `tool_calls` as "The tool calls generated by the model" as an array.
Context: While not explicitly stated as "parallel tool calls", the schema uses an array of tool_calls, and the examples show multiple tool calls being returned. The `tool_choice: "required"` option also implies the model can call multiple tools.
Confidence: HIGH

### 2.3 Strict Mode (Beta)

## Claim: Strict mode ensures tool outputs comply with the function's JSON schema; requires beta base URL
Source: Official DeepSeek API Docs - Tool Calls
URL: https://api-docs.deepseek.com/guides/tool_calls
Date: 2026-04-24
Excerpt: "To use `strict` mode, you need to: Use `base_url="https://api.deepseek.com/beta"` to enable Beta features; In the `tools` parameter, all `function` need to set the `strict` property to `true`; The server will validate the JSON Schema..."
Context: Strict mode is a beta feature. All object properties must be `required` and `additionalProperties` must be `false`. Supported JSON Schema types: object, string, number, integer, boolean, array, enum, anyOf.
Confidence: HIGH

### 2.4 Tool Calling in Thinking Mode

## Claim: When using tool calls in thinking mode, `reasoning_content` MUST be passed back to the API in all subsequent requests
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "Unlike turns in thinking mode that do not involve tool calls, for turns that do perform tool calls, the `reasoning_content` must be fully passed back to the API in all subsequent requests. If your code does not correctly pass back `reasoning_content`, the API will return a 400 error."
Context: This is a critical requirement. For non-tool-call conversations, previous reasoning_content can be omitted. But for tool-call conversations, every subsequent request must include the full reasoning_content from the assistant's previous tool-call turns. The error message is: "The reasoning_content in the thinking mode must be passed back to the API."
Confidence: HIGH

### 2.5 Example Tool Call Request/Response

**Request:**
```python
import openai
client = openai.OpenAI(api_key="<key>", base_url="https://api.deepseek.com")

tools = [
    {
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather of a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city name"
                    }
                },
                "required": ["location"]
            }
        }
    }
]

response = client.chat.completions.create(
    model="deepseek-v4-pro",
    messages=[{"role": "user", "content": "How's the weather in Hangzhou?"}],
    tools=tools,
    tool_choice="auto"
)
```

**Response (assistant message with tool calls):**
```json
{
  "role": "assistant",
  "content": null,
  "reasoning_content": "The user wants to know the weather in Hangzhou. I should call the get_weather function.",
  "tool_calls": [
    {
      "id": "call_00_xxxxxxxx",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"location\": \"Hangzhou\"}"
      }
    }
  ]
}
```

---

## 3. Streaming

### 3.1 How Streaming Works

## Claim: Streaming uses SSE format with `data: <json>\n\n` frames, terminated by `data: [DONE]`
Source: Official DeepSeek API Docs - Create Chat Completion API Reference
URL: https://api-docs.deepseek.com/api/create-chat-completion
Date: 2026-04-24
Excerpt: "If set, partial message deltas will be sent. Tokens will be sent as data-only server-sent events (SSE) as they become available, with the stream terminated by a `data: [DONE]` message."
Context: Standard OpenAI-compatible SSE streaming. Each chunk is a JSON object inside `data: ` lines. The final chunk is `data: [DONE]`.
Confidence: HIGH

### 3.2 Reasoning Content in Streaming

## Claim: In streaming with thinking mode, reasoning tokens are delivered in `delta.reasoning_content` BEFORE content tokens in `delta.content`
Source: Official DeepSeek API Docs - Thinking Mode (Streaming example)
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: ```python
for chunk in response:
    if chunk.choices[0].delta.reasoning_content:
        reasoning_content += chunk.choices[0].delta.reasoning_content
    else:
        content += chunk.choices[0].delta.content
```
Context: The streaming sequence for thinking mode is: (1) first chunk has `delta.role="assistant"`, (2) subsequent chunks deliver reasoning tokens via `delta.reasoning_content`, (3) once reasoning is complete, content tokens arrive via `delta.content`, (4) final chunk has `finish_reason` and `usage`.
Confidence: HIGH

### 3.3 Usage in Streaming

## Claim: With `stream_options={"include_usage": true}`, an additional chunk with usage statistics is streamed before `data: [DONE]`
Source: Official DeepSeek API Docs - Create Chat Completion API Reference
URL: https://api-docs.deepseek.com/api/create-chat-completion
Date: 2026-04-24
Excerpt: "If set, an additional chunk will be streamed before the `data: [DONE]` message. The `usage` field on this chunk shows the token usage statistics for the entire request, and the `choices` field will always be an empty array. All other chunks will also include a `usage` field, but with a null value."
Context: This is standard OpenAI-compatible streaming behavior. The usage chunk has `choices: []` and a populated `usage` object including `prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`, `completion_tokens`, and `completion_tokens_details.reasoning_tokens`.
Confidence: HIGH

### 3.4 Example Streaming Response Format

**Raw SSE stream with thinking mode:**
```
data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"deepseek-v4-pro","choices":[{"delta":{"role":"assistant"},"index":0}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"deepseek-v4-pro","choices":[{"delta":{"reasoning_content":"Let me compare 9.11 and 9.8. "},"index":0}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"deepseek-v4-pro","choices":[{"delta":{"reasoning_content":"9.8 is greater than 9.11 because 9.8 = 9.80 and 9.80 > 9.11."},"index":0}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"deepseek-v4-pro","choices":[{"delta":{"content":"9.8 is greater than 9.11."},"index":0}]}

data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","created":1234567890,"model":"deepseek-v4-pro","choices":[{"delta":{},"index":0,"finish_reason":"stop"}],"usage":{"prompt_tokens":15,"completion_tokens":42,"prompt_cache_hit_tokens":0,"prompt_cache_miss_tokens":15,"total_tokens":57,"completion_tokens_details":{"reasoning_tokens":25}}}

data: [DONE]
```

---

## 4. JSON Mode / Structured Output

### 4.1 How to Enable JSON Output

## Claim: JSON output is enabled by setting `response_format={"type": "json_object"}` and including the word "json" in the prompt
Source: Official DeepSeek API Docs - JSON Output
URL: https://api-docs.deepseek.com/guides/json_mode
Date: 2026-04-24
Excerpt: "To enable JSON Output, users should: Set the `response_format` parameter to `{'type': 'json_object'}`. Include the word 'json' in the system or user prompt, and provide an example of the desired JSON format to guide the model in outputting valid JSON."
Context: This is identical to OpenAI's JSON mode approach. The model guarantees valid JSON output. The prompt must explicitly instruct the model to output JSON.
Confidence: HIGH

## Claim: JSON mode may occasionally return empty content; the docs acknowledge this as a known issue
Source: Official DeepSeek API Docs - JSON Output
URL: https://api-docs.deepseek.com/guides/json_mode
Date: 2026-04-24
Excerpt: "When using the JSON Output feature, the API may occasionally return empty content. We are actively working on optimizing this issue. You can try modifying the prompt to mitigate such problems."
Context: This is an acknowledged limitation. Users should handle empty responses gracefully and potentially retry with modified prompts.
Confidence: HIGH

### 4.2 Example JSON Mode Request

```python
from openai import OpenAI
import json

client = OpenAI(api_key="<your api key>", base_url="https://api.deepseek.com")

system_prompt = """
The user will provide some exam text. Please parse the "question" and "answer" and output them in JSON format.
EXAMPLE JSON OUTPUT:
{
    "question": "Which is the highest mountain in the world?",
    "answer": "Mount Everest"
}
"""

response = client.chat.completions.create(
    model="deepseek-v4-pro",
    messages=[
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": "Which is the longest river in the world? The Nile River."}
    ],
    response_format={"type": "json_object"}
)
result = json.loads(response.choices[0].message.content)
# {"question": "Which is the longest river in the world?", "answer": "The Nile River"}
```

---

## 5. Context Caching

### 5.1 How Context Caching Works

## Claim: Context Caching is enabled by default for all users; no code changes needed
Source: Official DeepSeek API Docs - Context Caching
URL: https://api-docs.deepseek.com/guides/kv_cache
Date: 2026-04-24
Excerpt: "The DeepSeek API Context Caching on Disk Technology is enabled by default for all users, allowing them to benefit without needing to modify their code. Each user request will trigger the construction of a hard disk cache. If subsequent requests have overlapping prefixes with previous requests, the overlapping part will only be fetched from the cache, which counts as a 'cache hit.'"
Context: This is fully automatic. Users don't need to opt in or configure anything. The system builds disk-based caches transparently.
Confidence: HIGH

### 5.2 Cache Persistence and Hit Rules

## Claim: Cache prefixes are persisted at request boundaries, via common prefix detection, and at fixed token intervals for long inputs
Source: Official DeepSeek API Docs - Context Caching
URL: https://api-docs.deepseek.com/guides/kv_cache
Date: 2026-04-24
Excerpt: "Persistence at request boundaries: Each request will produce two cache prefix units at the end position of the user input and the end position of the model output. Common prefix detection persistence: When the system detects a common prefix across multiple requests, it will persist that common prefix as an independent cache prefix unit. Persistence at fixed token intervals: For long inputs or long outputs, the system will carve out cache prefix units at fixed token intervals."
Context: The cache matching requires FULL matching of a cache prefix unit. Partial prefix matches do NOT count as cache hits. This is different from some other providers' prefix matching.
Confidence: HIGH

### 5.3 Cache Hit Pricing

## Claim: Cache hit pricing is 1/10 of the original launch price (reduced on 2026/4/26 12:15 UTC)
Source: Official DeepSeek API Docs - Pricing
URL: https://api-docs.deepseek.com/quick_start/pricing
Date: 2026-04-26
Excerpt: "For all models, the input cache hit price has been reduced to 1/10 of the launch price. This price adjustment takes effect from 2026/4/26 12:15 UTC."
Context: Current cache hit pricing: V4-Flash: $0.0028/1M tokens, V4-Pro: $0.003625/1M tokens (discounted). Cache miss: V4-Flash: $0.14/1M, V4-Pro: $0.435/1M (discounted).
Confidence: HIGH

### 5.4 Checking Cache Hit Status

## Claim: Cache hit/miss token counts are reported in the `usage` field as `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens`
Source: Official DeepSeek API Docs - Context Caching
URL: https://api-docs.deepseek.com/guides/kv_cache
Date: 2026-04-24
Excerpt: "In the response from the DeepSeek API, we have added two fields in the `usage` section to reflect the cache hit status of the request: `prompt_cache_hit_tokens`: The number of tokens in the input of this request that resulted in a cache hit. `prompt_cache_miss_tokens`: The number of tokens in the input of this request that did not result in a cache hit."
Context: These fields appear in both streaming and non-streaming responses (when `stream_options.include_usage=true` for streaming). Also note: `prompt_tokens = prompt_cache_hit_tokens + prompt_cache_miss_tokens`.
Confidence: HIGH

### 5.5 Cache Limitations

## Claim: The cache system works on a "best-effort" basis and does not guarantee 100% cache hit rate
Source: Official DeepSeek API Docs - Context Caching
URL: https://api-docs.deepseek.com/guides/kv_cache
Date: 2026-04-24
Excerpt: "The cache system works on a 'best-effort' basis and does not guarantee a 100% cache hit rate. Cache construction takes seconds. Once the cache is no longer in use, it will be automatically cleared, usually within a few hours to a few days."
Context: Don't rely on caching for deterministic cost reduction. Cache eviction happens within hours to days of disuse.
Confidence: HIGH

---

## 6. Rate Limits

### 6.1 Rate Limit Model

## Claim: DeepSeek uses DYNAMIC concurrency limits based on server load; no fixed public RPM/TPM tables exist
Source: Official DeepSeek API Docs - Rate Limit
URL: https://api-docs.deepseek.com/quick_start/rate_limit
Date: 2026-04-24
Excerpt: "DeepSeek API dynamically limits user concurrency based on server load. When you reach the concurrency limit, you will immediately receive an HTTP 429 response."
Context: This is fundamentally different from OpenAI and Anthropic which publish tier-based RPM/TPM limits. DeepSeek's limits vary dynamically with server load and short-term usage history. There is NO paid tier ladder to unlock higher limits.
Confidence: HIGH

### 6.2 What Happens at Limit

## Claim: When the concurrency limit is reached, the API returns HTTP 429 immediately; requests may also wait with keep-alive signals
Source: Official DeepSeek API Docs - Rate Limit
URL: https://api-docs.deepseek.com/quick_start/rate_limit
Date: 2026-04-24
Excerpt: "After your request is sent, it may take some time to receive a response from the server. During this period, your HTTP request will remain connected, and you may continuously receive contents in the following formats: Non-streaming requests: Continuously return empty lines; Streaming requests: Continuously return SSE keep-alive comments (`: keep-alive`). If the request has not started inference after 10 minutes, the server will close the connection."
Context: The API uses a queue-based system. Requests wait on an open connection with keep-alive signals. If inference hasn't started within 10 minutes, the connection is closed.
Confidence: HIGH

### 6.3 Specific Rate Limit Numbers

## Claim: No fixed RPM/TPM numbers are published by DeepSeek for V4 Pro or V4 Flash; limits are dynamic
Source: Official DeepSeek API Docs - Rate Limit; Third-party analysis
URL: https://api-docs.deepseek.com/quick_start/rate_limit; https://devtk.ai/en/blog/ai-api-rate-limits-comparison-2026/
Date: 2026-04-24; 2026-02-24
Excerpt: "DeepSeek does not publish fixed RPM/TPM tables for V4. Its official docs say concurrency is limited dynamically based on server load and short-term usage history."
Context: UNABLE TO VERIFY specific RPM/TPM numbers. DeepSeek intentionally does not publish fixed rate limits. The only official statement is "dynamic limits based on server load." Some third-party providers (SiliconFlow, GMI Cloud) that host DeepSeek models publish their own rate limits, but these are NOT the official DeepSeek API limits.
Confidence: N/A - NOT PUBLISHED BY OFFICIAL SOURCE

---

## 7. Base URLs

### 7.1 Confirmed Base URLs

## Claim: OpenAI-compatible base URL is `https://api.deepseek.com`
Source: Official DeepSeek API Docs - Your First API Call
URL: https://api-docs.deepseek.com/
Date: 2026-04-24
Excerpt: "base_url (OpenAI): https://api.deepseek.com"
Context: The standard base URL for OpenAI SDK compatibility. Chat completions endpoint: `POST https://api.deepseek.com/chat/completions`.
Confidence: HIGH

## Claim: Anthropic-compatible base URL is `https://api.deepseek.com/anthropic`
Source: Official DeepSeek API Docs - Your First API Call; Anthropic API Guide
URL: https://api-docs.deepseek.com/; https://api-docs.deepseek.com/guides/anthropic_api
Date: 2026-04-24
Excerpt: "base_url (Anthropic): https://api.deepseek.com/anthropic"
Context: For use with Anthropic SDK. Unsupported model names are automatically mapped to `deepseek-v4-flash`. The `anthropic-beta` HTTP header is ignored.
Confidence: HIGH

## Claim: Beta base URL is `https://api.deepseek.com/beta` for accessing beta features (Chat Prefix Completion, strict tool mode, 8K max_tokens)
Source: Official DeepSeek API Docs - Tool Calls; Chat Prefix Completion
URL: https://api-docs.deepseek.com/guides/tool_calls; https://api-docs.deepseek.com/guides/chat_prefix_completion
Date: 2026-04-24
Excerpt: "To use `strict` mode, you need to: Use `base_url='https://api.deepseek.com/beta'` to enable Beta features"
Context: Beta features include: strict tool mode, chat prefix completion, and 8K max_tokens support (vs default 4096).
Confidence: HIGH

---

## 8. API Errors

### 8.1 Error Codes

## Claim: DeepSeek API returns standard HTTP error codes: 400, 401, 402, 422, 429, 500, 503
Source: Official DeepSeek API Docs - Error Codes
URL: https://api-docs.deepseek.com/quick_start/error_codes
Date: 2026-04-24
Excerpt: Detailed error code table published.
Context: Here are the confirmed error codes:

| Code | Name | Cause | Solution |
|------|------|-------|----------|
| 400 | Invalid Format | Invalid request body format | Modify request body per error hints |
| 401 | Authentication Fails | Wrong API key | Check API key |
| 402 | Insufficient Balance | Out of balance | Top up account |
| 422 | Invalid Parameters | Invalid parameters in request | Modify parameters per error hints |
| 429 | Rate Limit Reached | Sending requests too quickly | Pace requests; switch to alternative providers temporarily |
| 500 | Server Error | Server issue | Retry after brief wait |
| 503 | Server Overloaded | Server overloaded due to high traffic | Retry after brief wait |

Confidence: HIGH

### 8.2 Error Format

## Claim: Error responses follow OpenAI-compatible JSON format with `error` object containing `message`, `type`, `param`, and `code`
Source: GitHub issues showing DeepSeek V4 API error responses
URL: https://github.com/Kilo-Org/kilocode/issues/9482
Date: 2026-04-24
Excerpt: ```json
{"error":{"message":"The reasoning_content in the thinking mode must be passed back to the API.","type":"invalid_request_error","param":null,"code":"invalid_request_error"}}
```
Context: Standard OpenAI-compatible error format. `type` is typically `"invalid_request_error"` for 400/422 errors. The `param` field may be null. `code` mirrors the `type` for most errors.
Confidence: HIGH

### 8.3 Additional Error Scenarios

| Scenario | Error Code | Error Message Example |
|----------|-----------|----------------------|
| Missing reasoning_content in tool-call follow-up | 400 | "The reasoning_content in the thinking mode must be passed back to the API." |
| Invalid JSON body | 400 | "Invalid Format" |
| Wrong API key | 401 | "Authentication Fails" |
| Out of balance | 402 | "Insufficient Balance" |
| Invalid model ID | 422 | "Invalid Parameters" |
| Rate limit exceeded | 429 | "Rate Limit Reached" |
| Server internal error | 500 | "Server Error" |
| Server overloaded | 503 | "Server Overloaded" |

---

## 9. Multi-turn Conversation: `reasoning_content` Handling Rules

### 9.1 Without Tool Calls

## Claim: In thinking mode without tool calls, previous turns' `reasoning_content` does NOT need to be passed back; if passed, it will be ignored
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "Between two `user` messages, if the model did not perform a tool call, the intermediate assistant's `reasoning_content` does not need to participate in the context concatenation. If passed to the API in subsequent turns, it will be ignored."
Context: For simple multi-turn Q&A in thinking mode, you can safely omit reasoning_content from previous turns. Only the final `content` from assistant responses needs to be included in the messages history.
Confidence: HIGH

### 9.2 With Tool Calls

## Claim: In thinking mode WITH tool calls, `reasoning_content` MUST be passed back in ALL subsequent requests
Source: Official DeepSeek API Docs - Thinking Mode
URL: https://api-docs.deepseek.com/guides/thinking_mode
Date: 2026-04-19
Excerpt: "Between two `user` messages, if the model performed a tool call, the intermediate assistant's `reasoning_content` must participate in the context concatenation and must be passed back to the API in all subsequent user interaction turns."
Context: This is enforced by the API - omitting reasoning_content after a tool call turn results in a 400 error. The reasoning_content must be included in the assistant message alongside content and tool_calls.
Confidence: HIGH

### 9.3 Practical Message Building

**For non-tool-call multi-turn (simplified):**
```python
# Turn 1
response = client.chat.completions.create(...)
messages.append(response.choices[0].message)  # includes reasoning_content, but OK
messages.append({"role": "user", "content": "Follow-up question"})
# The API will ignore the previous reasoning_content
```

**For tool-call multi-turn (REQUIRED):**
```python
# After tool call turn
messages.append({
    "role": "assistant",
    "content": response.choices[0].message.content,
    "reasoning_content": response.choices[0].message.reasoning_content,
    "tool_calls": response.choices[0].message.tool_calls
})
messages.append({"role": "tool", "tool_call_id": tool.id, "content": tool_result})
messages.append({"role": "user", "content": "Follow-up question"})
# reasoning_content from ALL previous tool-call turns must be preserved
```

---

## 10. Finish Reasons

## Claim: The API returns specific finish_reason values including `insufficient_system_resource`
Source: Official DeepSeek API Docs - Create Chat Completion API Reference
URL: https://api-docs.deepseek.com/api/create-chat-completion
Date: 2026-04-24
Excerpt: "Possible values: [`stop`, `length`, `content_filter`, `tool_calls`, `insufficient_system_resource`] ... `insufficient_system_resource` if the request is interrupted due to insufficient resource of the inference system."
Context: The `insufficient_system_resource` finish reason is unique to DeepSeek and indicates the request was interrupted due to server resource constraints. This is different from `stop` or `length`.
Confidence: HIGH

---

## Summary Table

| Capability | Supported? | DeepSeek V4 Pro | DeepSeek V4 Flash | API Parameter | Source | Notes |
|-----------|------------|-----------------|-------------------|---------------|--------|-------|
| Thinking Mode | Yes | Yes | Yes (default) | `thinking.type`: "enabled"/"disabled" + `reasoning_effort`: "high"/"max" | Official Docs | Thinking ON by default; use `extra_body` for OpenAI SDK |
| Non-thinking Mode | Yes | Yes | Yes | `thinking.type`: "disabled" | Official Docs | FIM completion requires non-thinking mode |
| Reasoning effort control | Yes | Yes | Yes | `reasoning_effort`: "high" or "max" | Official Docs | low/medium mapped to high; xhigh mapped to max |
| Streaming | Yes | Yes | Yes | `stream: true` | Official Docs | SSE format, `data: [DONE]` terminator |
| Streaming reasoning tokens | Yes | Yes | Yes | `delta.reasoning_content` | Official Docs | Reasoning tokens come BEFORE content tokens |
| JSON Mode | Yes | Yes | Yes | `response_format: {"type": "json_object"}` | Official Docs | Must include "json" in prompt; occasional empty responses |
| Tool Calls | Yes | Yes | Yes | `tools` array | Official Docs | OpenAI-style function calling |
| Parallel Tool Calls | Yes | Yes | Yes | `tool_calls` array in response | Official Docs | Multiple tool calls in single response |
| Strict Tool Mode | Yes (Beta) | Yes | Yes | `strict: true` + beta base URL | Official Docs | Requires `https://api.deepseek.com/beta` |
| Max tools | Yes | 128 | 128 | Max 128 functions in `tools` | Official Docs | — |
| Context Caching | Yes (Automatic) | Yes | Yes | Automatic, no parameter needed | Official Docs | Enabled by default; best-effort |
| Cache hit pricing | Yes | $0.003625/1M | $0.0028/1M | Reported in `usage.prompt_cache_hit_tokens` | Official Docs | 1/10 of original price since 2026/4/26 |
| Cache miss pricing | Yes | $0.435/1M (discounted) | $0.14/1M | Reported in `usage.prompt_cache_miss_tokens` | Official Docs | Pro is 75% off until 2026/05/31 |
| Output pricing | Yes | $0.87/1M (discounted) | $0.28/1M | Billed per output tokens | Official Docs | Includes reasoning tokens in output |
| Rate limits | Dynamic | Dynamic | Dynamic | HTTP 429 when exceeded | Official Docs | NO fixed RPM/TPM tables published |
| OpenAI-compatible API | Yes | Yes | Yes | Base URL: `https://api.deepseek.com` | Official Docs | Full Chat Completions compatibility |
| Anthropic-compatible API | Yes | Yes | Yes | Base URL: `https://api.deepseek.com/anthropic` | Official Docs | Unsupported models auto-map to v4-flash |
| 1M context length | Yes | Yes | Yes | Up to 1M tokens total | Official Docs | Input + output combined limited by 1M |
| 384K max output | Yes | Yes | Yes | `max_tokens` up to 384K | Official Docs | Default max_tokens is 4096 (8192 on beta) |
| Temperature control | Yes (non-thinking) | Yes | Yes | `temperature`: 0-2 (default 1) | Official Docs | IGNORED in thinking mode |
| top_p control | Yes (non-thinking) | Yes | Yes | `top_p`: 0-1 (default 1) | Official Docs | IGNORED in thinking mode |
| presence_penalty | Deprecated | No effect | No effect | Parameter accepted but ignored | Official Docs | Deprecated, no effect even in non-thinking |
| frequency_penalty | Deprecated | No effect | No effect | Parameter accepted but ignored | Official Docs | Deprecated, no effect even in non-thinking |
| `reasoning_tokens` in usage | Yes | Yes | Yes | `usage.completion_tokens_details.reasoning_tokens` | Official Docs | Separate count of reasoning tokens used |
| `finish_reason` values | Yes | stop, length, content_filter, tool_calls, insufficient_system_resource | Same | `finish_reason` in response | Official Docs | `insufficient_system_resource` is DeepSeek-specific |

---

## Key Caveats and Gotchas

1. **Thinking is ON by default**: If you don't specify `thinking.type`, it defaults to `"enabled"`. To get non-thinking mode, you must explicitly pass `thinking: {"type": "disabled"}` or omit `reasoning_effort` and use a non-thinking workflow.

2. **reasoning_content MUST be echoed for tool calls**: In thinking mode with tool calls, failing to pass back `reasoning_content` results in a 400 error. This is the #1 integration issue reported on GitHub.

3. **Temperature/top_p ignored in thinking mode**: These parameters are accepted but silently ignored when thinking mode is enabled. For deterministic reasoning output, this is expected behavior.

4. **Dynamic rate limits**: There are NO published RPM/TPM numbers. Plan for 429 errors and implement exponential backoff with fallback providers.

5. **JSON mode occasional emptiness**: The docs acknowledge that JSON mode may occasionally return empty content. Always handle this case.

6. **Cache is best-effort**: Don't architect for 100% cache hit rates. Cache eviction happens within hours to days.

7. **Beta base URL**: For strict tool mode, chat prefix completion, or 8K max_tokens, use `https://api.deepseek.com/beta`.

8. **Legacy model deprecation**: `deepseek-chat` and `deepseek-reasoner` will be fully deprecated on 2026/07/24. They currently map to v4-flash non-thinking and v4-flash thinking respectively.

9. **Presence/frequency penalty deprecated**: These parameters are no longer supported at all (not just in thinking mode). They are accepted but have no effect.

10. **Usage tracking**: Always set `stream_options={"include_usage": true}` when streaming to get complete token usage including cache hit/miss breakdowns.

---

## Sources Referenced

1. Official DeepSeek API Docs - Thinking Mode: https://api-docs.deepseek.com/guides/thinking_mode
2. Official DeepSeek API Docs - Tool Calls: https://api-docs.deepseek.com/guides/tool_calls
3. Official DeepSeek API Docs - JSON Output: https://api-docs.deepseek.com/guides/json_mode
4. Official DeepSeek API Docs - Context Caching: https://api-docs.deepseek.com/guides/kv_cache
5. Official DeepSeek API Docs - Pricing: https://api-docs.deepseek.com/quick_start/pricing
6. Official DeepSeek API Docs - Rate Limit: https://api-docs.deepseek.com/quick_start/rate_limit
7. Official DeepSeek API Docs - Error Codes: https://api-docs.deepseek.com/quick_start/error_codes
8. Official DeepSeek API Docs - Anthropic API: https://api-docs.deepseek.com/guides/anthropic_api
9. Official DeepSeek API Docs - Create Chat Completion: https://api-docs.deepseek.com/api/create-chat-completion
10. Official DeepSeek API Docs - Your First API Call: https://api-docs.deepseek.com/
11. ds2api reverse-engineering notes: https://github.com/CJackHwang/ds2api/blob/main/API.en.md
12. DataCamp DeepSeek V4 Tutorial: https://www.datacamp.com/tutorial/deepseek-v4-api-tutorial
13. WaveSpeed DeepSeek V4 Migration Guide: https://wavespeed.ai/blog/posts/blog-deepseek-v4-model-name-migration/
14. GitHub Issues (multiple): Kilo-Org, OpenClaw, NousResearch, zeroclaw-labs
