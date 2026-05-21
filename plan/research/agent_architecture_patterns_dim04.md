# Architecture Patterns in Production AI Coding Agents

## Research Findings: Practical Implementation Details

**Date**: 2026-06-28
**Sources**: Claude Code v2.1.88 source analysis (VILA Lab paper), Anthropic documentation, OpenClaw architecture, Aider documentation, production SDKs, community analyses

---

## Table of Contents

1. [Agent Loop Patterns](#1-agent-loop-patterns)
2. [Tool Use Protocols](#2-tool-use-protocols)
3. [Permission Systems](#3-permission-systems)
4. [Context Management](#4-context-management)
5. [Subagent Orchestration](#5-subagent-orchestration)
6. [Memory Systems](#6-memory-systems)
7. [Prompt Engineering for Coding Agents](#7-prompt-engineering-for-coding-agents)

---

## 1. Agent Loop Patterns

### 1.1 ReAct Pattern (Reasoning + Acting)

The ReAct pattern (Reasoning + Acting), formalized by Yao et al. (2022), is the foundational pattern for virtually all production coding agents. It interleaves reasoning traces ("Thoughts") with action steps (tool calls) in a closed loop:

```
User Input -> Thought -> Action -> Observation -> Thought -> Action -> ... -> Answer
```

**Core implementation structure (Python pseudocode):**

```python
class ReActAgent:
    def __init__(self, tools: Dict[str, Callable], max_iterations: int = 10):
        self.tools = tools
        self.max_iterations = max_iterations
        self.trajectory: List[Dict[str, str]] = []

    def run(self, question: str, llm: Callable[[str], str]) -> Dict[str, Any]:
        self.trajectory = []
        for i in range(self.max_iterations):
            # 1. Format prompt with full trajectory
            prompt = self.format_prompt(question)
            # 2. Call LLM
            response = llm(prompt)
            # 3. Parse thought and action
            thought, action, params = self.parse_response(response)
            # 4. Execute tool
            if action == "finish":
                return {"success": True, "answer": params}
            observation = self.tools[action](params)
            # 5. Record in trajectory
            self.trajectory.append({
                "thought": thought,
                "action": f"{action}[{params}]",
                "observation": str(observation)
            })
        return {"success": False, "reason": "Max iterations reached"}
```

**Production adaptations of ReAct:**

Modern coding agents have evolved beyond the original paper's format. Key differences in production:

| Aspect | Academic ReAct | Production Coding Agents |
|--------|---------------|--------------------------|
| Thought format | Free text in response | Structured `thinking` blocks or content block separation |
| Action format | Custom text parsing (`Action: tool[params]`) | Native `tool_use`/`function_call` API blocks with JSON schemas |
| Observation format | Plain text | Structured `tool_result` messages with `tool_use_id` pairing |
| Loop control | Fixed max iterations | Multiple stop conditions (no tool use, max turns, context overflow, hook intervention) |
| Error handling | Retry with same prompt | Multi-layer recovery ladder (compaction, token escalation, fallback model) |

**Key insight**: Only ~1.6% of Claude Code's codebase is AI decision logic; the remaining 98.4% is operational infrastructure around the loop.

---

### 1.2 Claude Code's Core Loop (`queryLoop`)

Claude Code's entire system is powered by a single `async generator` function called `queryLoop()` in `query.ts`. This is the architectural center: all interfaces (CLI, SDK, IDE) converge on this same loop.

**9-step pipeline per turn:**

```
1. Settings resolution     - Destructure immutable params (system prompt, permission callback, model config)
2. Mutable state init      - Single State object stores all mutable state across iterations
3. Context assembly        - getMessagesAfterCompactBoundary(): get messages since last compaction
4. Pre-model shapers (5)   - Budget reduction -> Snip -> Microcompact -> Context Collapse -> Auto-Compact
5. Model call              - for await loop over deps.callModel() streams the response
6. Tool-use dispatch       - StreamingToolExecutor or fallback runTools()
7. Permission gate         - Deny -> Ask -> Allow rule evaluation
8. Tool execution          - Results added as tool_result messages, loop continues
9. Stop condition          - No tool_use blocks? Turn complete.
```

**Key implementation details:**

```typescript
// Pseudocode representation of queryLoop
async function* queryLoop(params: QueryParams): AsyncGenerator<StreamEvent> {
  let state: State = initializeState();
  
  while (true) {
    // Step 1-2: Settings + state
    const { systemPrompt, permissionCallback, modelConfig } = params;
    
    // Step 3: Context assembly
    let messages = getMessagesAfterCompactBoundary(state.messages);
    
    // Step 4: Pre-model shapers (5-layer compaction)
    messages = applyToolResultBudget(messages, state.toolBudgetState);
    messages = maybeHistorySnip(messages);
    messages = microcompact(messages);
    messages = maybeContextCollapseProjection(messages);
    
    if (shouldAutoCompact(messages, modelConfig, state)) {
      const compacted = await fullCompaction(messages, state, modelConfig);
      if (compacted) {
        messages = buildPostCompactMessages(compacted);
      }
    }
    
    // Step 5: Model call (streaming)
    yield { type: "stream_request_start" };
    const response = await deps.callModel(messages, systemPrompt, tools, modelConfig);
    
    // Step 6-7: Parse tool_use blocks and check permissions
    const toolUses = parseToolUseBlocks(response);
    if (toolUses.length === 0) {
      // Step 9: Stop condition met
      yield { type: "result", content: response.text };
      break;
    }
    
    // Step 8: Execute tools (streaming executor for latency)
    for await (const result of streamingToolExecutor.execute(toolUses)) {
      yield { type: "tool_result", toolUseId: result.toolUseId, content: result.output };
      state.messages.push(createToolResultMessage(result));
    }
    
    // Check max turns, context overflow, hook intervention
    if (state.turnCount >= MAX_TURNS || state.shouldStop) {
      yield { type: "stop", reason: state.stopReason };
      break;
    }
  }
}
```

**Two execution paths for tools:**

| Path | When Used | How It Works |
|------|-----------|--------------|
| `StreamingToolExecutor` | Primary (latency optimization) | Begins executing tools as they stream in from the model response |
| `runTools()` | Fallback | Iterates over partitions from `partitionToolCalls()`, classifies tools as concurrent-safe or exclusive |

**5 stop conditions:**

1. **No tool use** - Response contains only text (finish_reason: stop)
2. **Max turns** - Configurable per-session turn limit reached
3. **Context overflow** - Context window exhausted even after compaction
4. **Hook intervention** - PreToolUse or stop hook blocks execution
5. **Explicit abort** - User cancels via signal

---

### 1.3 Aider's Loop

Aider uses a fundamentally different architecture. While Claude Code is a full agent runtime with subagents, MCP, and persistent memory, Aider is a focused code editor built around git.

**Aider architecture:**

```
User Instruction -> Coder -> LLM -> Edit Blocks -> File Apply -> Git Commit -> Response
```

**Core components:**

| Component | Purpose |
|-----------|---------|
| `Coder` | Central orchestrator. Multiple variants: `EditBlockCoder`, `WholeFileCoder`, `UnifiedDiffCoder`, `ArchitectCoder` |
| Model layer | LiteLLM-based routing to 100+ providers |
| `RepoMap` | Tree-sitter-powered ranked graph of symbols. Provides intelligent context without loading entire files |
| IO system | Handles interactive and headless modes |
| Git integration | Auto-commits each change with conventional-commit messages |

**Aider's loop is simpler than Claude Code's:**

```python
# Simplified pseudocode of Aider's core loop
class EditBlockCoder:
    def run(self, user_message: str) -> str:
        # 1. Build repo map for context
        repo_context = self.repo_map.get_ranked_tags_map(
            self.abs_fnames,  # files in chat
            self.abs_read_only_fnames,
            max_tokens=self.max_map_tokens
        )
        
        # 2. Format messages with edit format instructions
        messages = self.format_messages(user_message, repo_context)
        
        # 3. Send to LLM
        response = self.llm.complete(messages)
        
        # 4. Parse edit blocks from response
        edits = self.parse_edit_blocks(response)
        
        # 5. Apply edits to files
        for edit in edits:
            self.apply_edit(edit)
        
        # 6. Git commit
        if edits:
            self.git_commit(f"aider: {user_message[:50]}")
        
        return response
```

**Key differences from Claude Code:**

- **No subagent delegation** - Single-threaded, no child agents
- **No permission system** - Relies on git as safety net (all changes reversible)
- **No persistent memory across sessions** - Starts fresh each time (though supports conventions files)
- **Model-agnostic** - Works with any OpenAI-compatible API via LiteLLM
- **RepoMap for context** - Instead of loading full files, builds a ranked symbol map
- **Architect mode** - Two-pass: architect model plans, editor model implements

---

### 1.4 Streaming Tool Calls (`tool_use` Deltas)

Modern agents use streaming to reduce perceived latency. The model's response arrives token-by-token, and the agent begins executing tools before the full response is received.

**Streaming event flow (Claude Code):**

```typescript
// Track streaming state
let inTool = false;

for await (const message of query({ prompt, options })) {
  if (message.type === "stream_event") {
    const event = message.event;

    switch (event.type) {
      case "content_block_start":
        if (event.content_block.type === "tool_use") {
          // Tool call is starting - show status
          console.log(`\n[Using ${event.content_block.name}...]`);
          inTool = true;
        }
        break;

      case "content_block_delta":
        if (event.delta.type === "text_delta" && !inTool) {
          // Stream text to user when NOT in a tool call
          process.stdout.write(event.delta.text);
        }
        // Tool call arguments accumulate internally
        break;

      case "content_block_stop":
        if (inTool) {
          // Tool call finished - arguments complete
          console.log(" done");
          inTool = false;
        }
        break;
    }
  } else if (message.type === "result") {
    // Agent finished all work
    console.log("\n\n--- Complete ---");
  }
}
```

**Streaming tool execution optimization:**

Claude Code's `StreamingToolExecutor` begins executing tools as their `tool_use` blocks are fully parsed from the stream. This means if the model emits 3 tool calls:

```
tool_use: Read(file1.ts)  <-- start executing as soon as arguments are parsed
text: "Let me check..."
tool_use: Read(file2.ts)  <-- start executing as soon as arguments are parsed  
tool_use: Bash(npm test)  <-- start executing as soon as arguments are parsed
```

The first Read can begin while the second Read and Bash are still streaming. This parallelizes execution and significantly reduces end-to-end latency.

---

### 1.5 Error Handling and Retry Logic

Production agents need robust error recovery. Claude Code implements an "error recovery ladder" - escalating recovery strategies rather than simple retry.

**Recovery ladder (least to most aggressive):**

| Trigger | Step 1 | Step 2 | Step 3 |
|---------|--------|--------|--------|
| `prompt_too_long` (413) | Drain staged collapse summaries | Reactive compact | Surface to user |
| `max_output_tokens` | Escalate cap 8K -> 64K | Multi-turn recovery (<=3 attempts) | Surface |
| `media_size_error` | Reactive compact | -- | Surface |

**Key recovery principles:**

1. **Continue states instead of returns** - The loop uses named continue points (`collapse_drain_retry`, `reactive_compact_retry`, `max_output_tokens_escalate`, etc.) rather than returns. This makes the loop testable - every test asserts which transition fired.

2. **Guards prevent infinite loops** - `hasAttemptedReactiveCompact` one-shot flags, hard caps on recovery attempts, circuit breakers.

3. **Never run stop hooks on error responses** - This prevents "error -> hook blocks -> retry -> error" spirals.

4. **Cancellation handling** - Aborts can hit during streaming or tool execution. The executor drains remaining requests by emitting synthetic `tool_result` blocks for queued/running tools. The Anthropic API rejects an assistant message containing `tool_use` without matching `tool_result`.

5. **Tool_use / tool_result invariant** - Every `tool_use` must have a paired `tool_result` in message history before the next API call. This is enforced as an invariant, not a hope.

**Aider's error handling** (simpler, git-based):

```python
# Aider uses git for recovery
class Coder:
    def apply_edit(self, edit):
        try:
            # Apply the edit
            file_content = edit.apply(file_content)
            # Write file
            write_file(edit.filename, file_content)
            # Git commit
            self.repo.git.commit("-m", f"aider: {edit.description}")
        except Exception as e:
            # On failure, user can /undo to revert
            self.io.tool_error(f"Error applying edit: {e}")
```

---

## 2. Tool Use Protocols

### 2.1 OpenAI-Style Function Calling

OpenAI's function calling API is the de facto standard. Most agents and frameworks use this format.

**Request format (tools definition):**

```json
{
  "model": "gpt-4o",
  "messages": [
    {"role": "user", "content": "What's the weather in San Francisco?"}
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "Get weather for a location. Only call this if the user supplies a location.",
        "parameters": {
          "type": "object",
          "properties": {
            "location": {
              "type": "string",
              "description": "City and state, e.g. San Francisco, CA"
            }
          },
          "required": ["location"]
        }
      }
    }
  ]
}
```

**Response format (assistant requests tool call):**

```json
{
  "role": "assistant",
  "content": null,
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"location\":\"San Francisco, CA\"}"
      }
    }
  ]
}
```

**Tool result format (must be provided in next request):**

```json
{
  "role": "tool",
  "tool_call_id": "call_abc123",
  "content": "72 degrees Fahrenheit, sunny"
}
```

**Critical rule**: The `tool_call_id` in the tool result MUST match the `id` from the assistant's `tool_calls`. The API enforces this pairing.

---

### 2.2 Claude's Tool Use Format

Claude uses a content block-based format that is semantically equivalent to OpenAI's but structurally different.

**Tool definition format (same as OpenAI):**

```json
{
  "tools": [
    {
      "name": "get_weather",
      "description": "Get weather for a location",
      "input_schema": {
        "type": "object",
        "properties": {
          "location": {"type": "string", "description": "City and state"}
        },
        "required": ["location"]
      }
    }
  ]
}
```

**Assistant requests tool use:**

```json
{
  "role": "assistant",
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_01A3B",
      "name": "get_weather",
      "input": {
        "location": "San Francisco, CA"
      }
    }
  ]
}
```

**Tool result format:**

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_01A3B",
      "content": "72 degrees Fahrenheit, sunny",
      "is_error": false
    }
  ]
}
```

**Key differences from OpenAI:**

| Aspect | OpenAI | Claude (Anthropic) |
|--------|--------|-------------------|
| Schema key | `parameters` | `input_schema` |
| Tool call key | `tool_calls` array | `content` array with `type: "tool_use"` |
| Arguments key | `arguments` (JSON string) | `input` (JSON object) |
| Tool call ID | `id` | `id` |
| Result role | `role: "tool"` | `role: "user"` with `type: "tool_result"` |
| Result ID key | `tool_call_id` | `tool_use_id` |
| Error indication | Content text | `is_error: true` boolean |

---

### 2.3 DeepSeek's Tool Calling Format

DeepSeek is fully OpenAI-compatible. It uses the same function calling format.

```python
from openai import OpenAI

client = OpenAI(
    api_key="<your api key>",
    base_url="https://api.deepseek.com",
)

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
                        "description": "City and state"
                    }
                },
                "required": ["location"]
            }
        }
    }
]

# Works identically to OpenAI
response = client.chat.completions.create(
    model="deepseek-v4-pro",
    messages=messages,
    tools=tools
)
```

The execution flow: user asks question -> model returns function call -> user calls function and provides result -> model returns natural language answer.

---

### 2.4 Streaming Tool Calls (Handling Partial Tool Calls)

Streaming function calling requires accumulating partial data across chunks. Tool call fields (name, arguments) may be split across multiple deltas.

**OpenAI streaming format:**

```
Chunk 1: {"tool_calls": [{"index": 0, "id": "call_123", "function": {"arguments": ""}}]}
Chunk 2: {"tool_calls": [{"index": 0, "function": {"name": "bash", "arguments": "{"}}]}
Chunk 3: {"tool_calls": [{"index": 0, "function": {"arguments": "\"command\""}}]}
Chunk 4: {"tool_calls": [{"index": 0, "function": {"arguments": ": \"ls -la\"}"}}]}
```

**Implementation pattern for accumulating tool calls:**

```python
from dataclasses import dataclass, field
from typing import Dict

@dataclass
class StreamingState:
    """Tracks partial tool call data across chunks."""
    function_calls: Dict[int, dict] = field(default_factory=dict)
    
    def process_chunk(self, chunk):
        delta = chunk.choices[0].delta
        
        if delta.tool_calls:
            for tc_delta in delta.tool_calls:
                index = tc_delta.index
                
                # Initialize new tool call
                if index not in self.function_calls:
                    self.function_calls[index] = {
                        "index": index,
                        "id": tc_delta.id,
                        "type": tc_delta.type,
                        "function": {"name": "", "arguments": ""}
                    }
                
                # Accumulate name and arguments incrementally
                existing = self.function_calls[index]
                if tc_delta.function.name:
                    existing["function"]["name"] += tc_delta.function.name
                if tc_delta.function.arguments:
                    existing["function"]["arguments"] += tc_delta.function.arguments
        
        # Tool call is complete when finish_reason == "tool_calls"
        if chunk.choices[0].finish_reason == "tool_calls":
            return self._finalize_tool_calls()
        
        return None
    
    def _finalize_tool_calls(self):
        """Parse accumulated JSON arguments and return complete tool calls."""
        results = []
        for tc in self.function_calls.values():
            try:
                tc["function"]["arguments"] = json.loads(
                    tc["function"]["arguments"]
                )
            except json.JSONDecodeError:
                tc["function"]["arguments"] = {}  # Handle malformed JSON
            results.append(tc)
        return results
```

**Common streaming pitfalls:**

1. **Partial name/arguments** - `function.name` may arrive in a later chunk than the first `tool_calls` delta. Must accumulate, not validate too early.
2. **Missing `index` key** - Some OpenAI-compatible providers (Gemini) omit the `index` within each tool_call object. Need defensive handling.
3. **JSON arguments streaming** - Arguments arrive as partial JSON strings. Must buffer until complete before parsing.
4. **Fake IDs** - Chat Completions doesn't provide response IDs like the Responses API. Need placeholder IDs for event lifecycle tracking.

---

### 2.5 Tool Result Formatting

**Best practices for tool results:**

1. **Always pair tool_use with tool_result** - Every `tool_use` block in the assistant's message must have a corresponding `tool_result` in the next user message. The API rejects mismatched pairs.

2. **Include `is_error` for failures** - Claude's format supports `is_error: true` to indicate tool failures. This helps the model distinguish between successful empty results and errors.

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01A3B",
  "is_error": true,
  "content": "Command failed: npm test exited with code 1. 3 tests failed."
}
```

3. **Truncate long results** - Individual tool results should be capped at a configurable size. Claude Code applies per-tool-result budgets before every model call.

4. **Structured content when possible** - Use typed content blocks instead of raw strings when the result has structure.

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01A3B",
  "content": [
    {"type": "text", "text": "Files found:"},
    {"type": "json", "json": ["src/main.ts", "src/lib.ts"]}
  ]
}
```

---

### 2.6 Multi-Turn Tool Execution

Multiple tool calls can be made in parallel or serially.

**Parallel execution** (for independent read-only tools):

```json
// Assistant requests multiple tools at once
{
  "role": "assistant",
  "content": [
    {"type": "tool_use", "id": "toolu_1", "name": "Read", "input": {"file": "src/auth.ts"}},
    {"type": "tool_use", "id": "toolu_2", "name": "Read", "input": {"file": "src/db.ts"}},
    {"type": "tool_use", "id": "toolu_3", "name": "Read", "input": {"file": "src/api.ts"}}
  ]
}

// All results returned in a single message
{
  "role": "user",
  "content": [
    {"type": "tool_result", "tool_use_id": "toolu_1", "content": "...auth code..."},
    {"type": "tool_result", "tool_use_id": "toolu_2", "content": "...db code..."},
    {"type": "tool_result", "tool_use_id": "toolu_3", "content": "...api code..."}
  ]
}
```

**Serial execution** (for dependent or state-mutating tools):

Claude Code's `partitionToolCalls()` classifies tools as concurrent-safe or exclusive:
- **Concurrent-safe**: Read-only operations (`Read`, `Grep`, `Glob`) execute in parallel
- **Exclusive**: State-mutating operations (`Write`, `Edit`, `Bash`) execute serially

---

## 3. Permission Systems

### 3.1 Claude Code's 7-Mode Permission System

Claude Code implements the most sophisticated permission system in any production coding agent. It has **7 permission modes** spanning a graduated autonomy spectrum.

**The 7 permission modes:**

| Mode | Description | Use Case |
|------|-------------|----------|
| `plan` | Model creates plan; execution proceeds only after user approval | High-risk environments, audit requirements |
| `default` | Standard interactive; most operations require user approval | Daily development work |
| `acceptEdits` | File edits and basic fs commands auto-approved; shell commands still ask | Trusted projects |
| `auto` | ML-based classifier evaluates requests not passing fast-path checks | Power users (gated by `TRANSCRIPT_CLASSIFIER` flag) |
| `dontAsk` | No prompting, but deny rules still enforced | Automation/CI |
| `bypassPermissions` | Skips most prompts; safety-critical checks still apply | Isolated environments only |
| `bubble` | Internal mode for subagent permission escalation to parent | Subagent delegation only |

**Why 7 modes?** Anthropic's auto-mode analysis found that users approve ~93% of permission prompts, making interactive confirmation behaviorally unreliable as a sole safety mechanism. The system must maintain safety independently of human vigilance.

---

### 3.2 Deny-First Rule Evaluation

Permission rules are evaluated in **strict order**: deny -> ask -> allow. The first match wins.

```
Deny rules (always block) -> Ask rules (prompt user) -> Allow rules (auto-approve) -> Mode default
```

**Critical property**: A deny rule always takes precedence over an allow rule, even when the allow rule is more specific. A broad deny ("deny all shell commands") cannot be overridden by a narrow allow ("allow npm test").

**Rule syntax:**

```
Tool(specifier)
```

Examples:
- `Bash` - matches ALL bash commands
- `Bash(npm run *)` - matches commands starting with "npm run"
- `Bash(rm -rf *)` - matches rm -rf commands
- `Read(./.env)` - matches reading .env file
- `Edit(/src/**)` - edits any file under src/ (recursive)
- `WebFetch(domain:example.com)` - fetch from specific domain
- `mcp__notion__*` - any tool from Notion MCP server

**4-level permission hierarchy (highest scope wins):**

| Level | Location | Scope |
|-------|----------|-------|
| 1 (Highest) | Managed policy (`/etc/claude-code/`) | Organization-wide enforcement |
| 2 | `~/.claude/settings.json` | User personal defaults |
| 3 | `.claude/settings.json` | Project-level (committed to git) |
| 4 (Lowest) | `.claude/settings.local.json` | Local overrides (git-ignored) |

Array settings like `permissions.allow` **merge across scopes**, they don't replace each other.

**Example permission configuration:**

```json
{
  "permissions": {
    "deny": [
      "Read(**/.env*)",
      "Read(**/secrets/**)",
      "Read(**/*.pem)",
      "Read(**/*.key)",
      "Read(~/.ssh/**)",
      "Read(~/.aws/**)"
    ],
    "ask": [
      "Bash(rm -rf *)",
      "Bash(git push --force *)",
      "Bash(git reset --hard *)",
      "Bash(sudo *)",
      "Bash(chmod 777 *)"
    ],
    "allow": [
      "Bash(npm *)",
      "Bash(git commit *)",
      "Read(*)",
      "Edit(*)",
      "Write(*)"
    ]
  }
}
```

---

### 3.3 The Authorization Pipeline (7 Layers)

Every tool invocation passes through multiple independent safety layers:

```
1. Pre-filtering        - Content-based deny rules evaluated first
2. Mode check           - Current permission mode applied
3. Rule evaluation      - Deny -> Ask -> Allow chain
4. ML classifier        - Auto mode: transcript classifier evaluates risk
5. PreToolUse hooks     - Programmable interception (shell commands, security checks)
6. PermissionRequest    - Async user dialog or coordinator-mode resolution
7. Post-execution hooks - Verification after tool runs
```

**PreToolUse hooks** can modify permission decisions. Example: a security check hook that scans shell commands before execution.

```typescript
hooks: {
  PreToolUse: [
    {
      matcher: "Bash",
      hooks: [
        { type: "command", command: "./scripts/security-check.sh" }
      ]
    }
  ]
}
```

**27 hook events** across 5 categories (PreToolUse, PostToolUse, PrePermissionRequest, PermissionResponse, Stop) with 4 execution types: shell, LLM-evaluated, webhook, subagent verifier.

---

### 3.4 Comparison: How Other Agents Handle Safety

| Agent | Safety Model | Key Mechanism |
|-------|-------------|---------------|
| **Claude Code** | Per-action deny-first rules + ML classifier + optional sandboxing | Fine-grained control over individual actions |
| **SWE-Agent** | Docker container isolation | Sandboxes entire execution environment |
| **OpenHands** | Docker container isolation | Sandboxes entire execution environment |
| **Aider** | Git-based rollback | All changes reversible through version control |
| **OpenClaw** | Perimeter-level access control | Single trusted operator, DM pairing, opt-in sandboxing |

---

### 3.5 Permission Systems in Practice

**Progressive trust pattern**: The agent starts with minimal autonomy; users expand it by approving tool invocations that become permanent rules. This "ask once, remember forever" approach balances safety with convenience.

**Practical permission setup for a team:**

```json
// .claude/settings.json (committed to git - team shared)
{
  "permissions": {
    "allow": [
      "Bash(npm run *)",
      "Bash(git commit *)",
      "Edit(/src/**)",
      "Read(*)",
      "Glob(*)"
    ],
    "deny": [
      "Bash(git push *)",
      "Read(.env)"
    ],
    "ask": [
      "Bash(rm -rf *)",
      "Bash(git push --force *)"
    ]
  }
}
```

```json
// .claude/settings.local.json (git-ignored - personal overrides)
{
  "permissions": {
    "allow": [
      "Bash(git push origin main)"
    ],
    "mode": "acceptEdits"
  }
}
```

---

## 4. Context Management

### 4.1 The Context Window Problem

Even with 1M token context windows, long-running coding agents need context management. Why?

1. **Quality degradation** - Models perform worse on content in the "middle" of long contexts
2. **Cost** - More tokens = higher API costs
3. **Latency** - Longer prompts = slower responses
4. **Cache invalidation** - Changing context breaks prompt caching
5. **Compounding errors** - Early mistakes propagate through full history

---

### 4.2 Claude Code's 5-Layer Compaction Pipeline

Claude Code implements a **graduated lazy-degradation** pipeline: apply the least disruptive compression first, escalating only when cheaper strategies prove insufficient.

**Pipeline execution order (cheapest first):**

```
1. Budget Reduction  (always active)      - Per-tool-result size limits
2. Snip              (HISTORY_SNIP flag)   - Lightweight older-history trimming
3. Microcompact      (CACHED_MICROCOMPACT) - Fine-grained cache-aware compression
4. Context Collapse  (CONTEXT_COLLAPSE)    - Read-time virtual projection over history
5. Auto-Compact      (user-configurable)   - Full model-generated summary (last resort)
```

**Per-turn context pipeline (TypeScript pseudocode):**

```typescript
async function runQueryTurn(
  history: Message[],
  model: ModelInfo,
  source: QuerySource,
  state: RuntimeState
): Promise<Message[]> {
  // Step 1: Slice messages after last compact boundary
  let msgs = getMessagesAfterCompactBoundary(history);
  
  // Step 2: Apply tool result budget (ALWAYS)
  msgs = await applyToolResultBudget(msgs, state.toolBudgetState);
  
  // Step 3: Optionally snip history
  msgs = maybeHistorySnip(msgs);
  
  // Step 4: Microcompact (cache-aware)
  msgs = microcompact(msgs);
  
  // Step 5: Context collapse projection
  msgs = maybeContextCollapseProjection(msgs);
  
  // Step 6: Auto-compact if still too large
  if (shouldAutoCompact(msgs, model, source, state)) {
    const compacted = await fullCompaction(msgs, state, model);
    if (compacted) {
      msgs = buildPostCompactMessages(compacted);
    }
  }
  
  // Step 7: Call model
  const response = await callModel(msgs, model);
  return appendToHistory(history, msgs, response);
}
```

---

### 4.3 Each Compaction Layer Explained

**Layer 1: Budget Reduction (always active)**

Individual tool results are capped at a configurable size. A single verbose output (e.g., `npm test` with full coverage report) cannot consume disproportionate context.

```typescript
// Per-tool-result size limit
const TOOL_RESULT_BUDGET = 8192; // characters per result

function applyToolResultBudget(msgs: Message[], state: BudgetState): Message[] {
  return msgs.map(msg => {
    if (msg.kind === "tool_result" && msg.content.length > TOOL_RESULT_BUDGET) {
      // Truncate with indicator
      return {
        ...msg,
        content: msg.content.slice(0, TOOL_RESULT_BUDGET) + 
                 "\n... [output truncated, N chars hidden]"
      };
    }
    return msg;
  });
}
```

**Layer 2: Snip (HISTORY_SNIP)**

Lightweight trimming of older history. Drops messages that are both old and low-value (e.g., tool results from 20 turns ago that aren't referenced).

**Layer 3: Microcompact (CACHED_MICROCOMPACT)**

Fine-grained cache-aware compression. Instead of eagerly rewriting everything, tracks compactable `tool_result` blocks and edits them in a way that preserves prompt cache reuse.

Two variants:
- **Cached microcompact**: Tracks cacheable blocks, edits them to preserve cache hits
- **Time-based microcompact**: Removes stale low-value content with very low latency

**Layer 4: Context Collapse (CONTEXT_COLLAPSE)**

A read-time virtual projection over history. The full transcript is preserved on disk, but a "collapsed" view is presented to the model. This is non-destructive - the original history can be reconstructed.

```typescript
// buildPostCompactMessages returns:
// [boundaryMarker, ...summaryMessages, ...messagesToKeep, ...attachments, ...hookResults]

function buildPostCompactMessages(compacted: CompactResult): Message[] {
  const boundaryMarker = {
    type: "boundary",
    content: "--- Prior conversation summarized ---",
    metadata: {
      headUuid: compacted.headUuid,
      anchorUuid: compacted.anchorUuid,  
      tailUuid: compacted.tailUuid
    }
  };
  
  return [
    boundaryMarker,
    ...compacted.summaryMessages,    // Model-generated summary
    ...compacted.messagesToKeep       // Recent messages kept verbatim
  ];
}
```

**Layer 5: Auto-Compact (last resort)**

Full model-generated summary of old conversation. Triggered only when cheaper layers are insufficient.

```typescript
// Trigger threshold calculation
function shouldAutoCompact(msgs: Message[], model: ModelInfo, state: RuntimeState): boolean {
  // Reserve space for model output
  const effectiveContext = model.contextWindow - Math.min(model.maxOutputTokens, 20000);
  
  // Additional buffer for compaction overhead
  const autoCompactThreshold = effectiveContext - 13000;
  
  const currentTokens = countTokens(msgs);
  return currentTokens > autoCompactThreshold && 
         !state.hasAttemptedReactiveCompact &&
         state.userConfig.autoCompactEnabled;
}
```

---

### 4.4 Sliding Window vs Summarization vs Checkpoint

| Strategy | How It Works | Pros | Cons |
|----------|-------------|------|------|
| **Sliding Window** | Keep only N most recent messages | Simple, predictable | Loses important older context |
| **Head+Tail** | Split budget between head (system+task) and tail (recent work) | Preserves task definition AND recent progress | Drops middle context |
| **Tool Result Clearing** | Keep message structure but clear raw tool results deep in history | Lightest touch, preserves conversation shape | Loses specific tool outputs |
| **Summarization** | Compress old messages into summary using cheaper model | Preserves information | Extra LLM call cost, may lose details |
| **Semantic Selection** | Use embeddings to select contextually relevant messages | Retains relevant older info | More expensive, requires embeddings |
| **Graduated Compaction** (Claude Code) | Apply 5 layers of increasing aggressiveness | Best information preservation | Complex, hard to fully predict |

**Context Engineering Strategies Benchmark:**

A benchmark described in the literature identifies a failure mode called "thrashing" - the agent repeats steps because the compaction strategy dropped information it needed, causing it to re-read files or re-run tools.

---

### 4.5 How Claude Code Handles Long Conversations

**Effective context window calculation:**

```
effectiveContext = contextWindow - min(modelMaxOutput, 20k reserve)
autoCompactThreshold = effectiveContext - 13k buffer
```

The 13K buffer matters because:
- Compaction itself needs space to generate summaries
- Recovery paths need space for retry attempts
- The model needs room to respond

**9 ordered context sources** (in priority order):

1. System prompt (static)
2. CLAUDE.md hierarchy (lazy-loaded)
3. Auto memory (recent observations)
4. Tool pool (available tools)
5. Subagent/MCP state
6. User context (current message)
7. Conversation history (compacted)
8. File attachments
9. Hook results

**Append-oriented design**: Compaction never modifies or deletes previously written transcript lines. It only appends new boundary and summary events. This makes the system:
- Resumable by design
- Debuggable (full history on disk)
- Safe (no destructive operations)

---

## 5. Subagent Orchestration

### 5.1 Parent-Child Agent Patterns

Claude Code implements subagent delegation via the `Task` tool (or `AgentTool`). The core pattern is simple:

```
Parent Agent -> detects subtask -> spawns Child Agent with isolated context
  -> Child works independently -> returns summary to Parent
```

**Key property: prompt self-containment.** The child cannot see anything the parent sees. Everything it needs must be in the prompt. This is a feature, not a bug - it forces clear task decomposition.

**Two execution flavors:**

| Flavor | How It Works | When to Use |
|--------|-------------|-------------|
| **Synchronous** | Parent blocks and waits for result | Sequential subtasks where order matters |
| **Asynchronous** | Parent gets ID immediately, checks later | Parallel independent subtasks |

**When to delegate vs. handle directly:**

**Delegate when:**
- Task is focused and well-scoped (one objective, self-sufficient)
- Task would pollute parent context with raw output (e.g., "search all test files and summarize coverage")
- Task can run in parallel with other work
- Task requires different tools or permissions than the parent

**Handle directly when:**
- Quick targeted edits (subagent startup overhead not worth it)
- Task requires continuous context from parent
- Task is tightly coupled with parent's current state

---

### 5.2 Context Isolation Between Agents

Each subagent gets:

1. **Fresh context window** - No parent conversation history
2. **Independent tool pool** - Can be restricted with `tools` or `disallowedTools`
3. **Custom system prompt** - Defined in YAML frontmatter
4. **Optional worktree isolation** - Git worktree for file-level isolation
5. **Own permission mode** - Can have different safety settings
6. **Own max turns** - Prevents runaway subagents

**Subagent definition format:**

```yaml
---
name: code-reviewer
description: Code review specialist. Invoke for thorough code review of any changes.
tools: Read, Grep, Glob           # Only these tools available
disallowedTools: Bash, Write      # Explicitly blocked
model: sonnet                     # Can use different model
permissionMode: default           # Can have different safety level
maxTurns: 20                      # Limit subagent turns
skills: review-skill              # Preload specific skills
memory: user                      # Which memory scopes to load
background: false                 # Run sync or async
isolation: worktree               # Git worktree isolation
initialPrompt: "Start by analyzing the changed files"
---

Your subagent's system prompt goes here. This defines the subagent's role,
capabilities, and approach to solving problems.
```

---

### 5.3 Result Aggregation

Subagents return only **summary text** to the parent (1,000-2,000 tokens), not their full conversation history (which may be 10,000+ tokens).

```
Parent context impact:
  Full subagent work: 10,000+ tokens (conversation history)
  Summary only:       1,000-2,000 tokens (condensed result)
  Savings:            ~80-90% of context preserved
```

**Background subagents** run concurrently with the main conversation (`Ctrl+B`). The parent gets back an ID and can check in later via `/tasks`.

---

### 5.4 Parallel Subagent Execution

Three approaches to parallel work in Claude Code:

| Approach | Coordination | Communication | File Isolation |
|----------|-------------|---------------|----------------|
| **Subagents** | Parent delegates and collects | Return summary to parent | Optional worktree isolation |
| **Agent View** | User coordinates manually | Report to user only | Automatic worktree per session |
| **Agent Teams** (experimental) | Team lead assigns work | Inbox-based messaging between peers | No automatic isolation - partition by file ownership |

**Worktree isolation for parallel development:**

```
Main Working Tree
  |
  +-- Subagent A (worktree: /tmp/wt-a, branch: claude/feature-a)
  +-- Subagent B (worktree: /tmp/wt-b, branch: claude/feature-b)
  +-- Subagent C (worktree: /tmp/wt-c, branch: claude/refactor-c)
```

Each subagent:
- Gets its own git worktree on a separate branch
- Makes changes independently without affecting main working tree
- If no changes made, worktree auto-cleaned
- If changes exist, worktree path and branch returned to parent for review/merge

---

### 5.5 Subagent Limitations

- **No nested subagents** - Subagents cannot spawn other subagents
- **Latency hit** - Subagents start fresh and need time to gather context
- **Each invocation is a fresh instance** - Memory is opt-in via `memory` frontmatter field
- **Confirmation overhead at scale** - Many subagents returning detailed results still consume parent context
- **Plugin subagents drop hooks, mcpServers, and permissionMode** - If you need those, copy to `.claude/agents/`

---

## 6. Memory Systems

### 6.1 Session Memory (Conversation History)

Session memory is the conversation transcript. It persists for the duration of a session and is lost when the session ends.

**Claude Code storage:**
- Append-oriented JSONL transcript files
- Never modifies previous lines (append-only)
- Compaction adds summary boundaries, doesn't delete history
- Runs are resumable by design via `session_id`

**Key events in session transcript:**
- User messages
- Assistant messages (text + tool_use blocks)
- Tool results
- Boundary markers (compaction events)
- System events (permission decisions, hook results)

---

### 6.2 Project Memory (Persistent Across Sessions)

Claude Code uses a **file-based memory hierarchy** (no vector DB). Memory is fully inspectable, editable, and version-controllable.

**4-level CLAUDE.md hierarchy** (in precedence order):

| Level | Location | Scope | Committed to Git? |
|-------|----------|-------|-------------------|
| Managed | `/etc/claude-code/CLAUDE.md` | Organization-wide | Yes (IT managed) |
| User | `~/.claude/CLAUDE.md` | Personal, all projects | No |
| Project | `./CLAUDE.md` or `./.claude/CLAUDE.md` | Team-shared per repo | Yes |
| Local | `./CLAUDE.local.md` | Personal, this repo only | No (git-ignored) |

**Memory discovery behavior:**
- Claude traverses upward from CWD to discover CLAUDE.md files
- Subtree files discovered contextually when accessing directories
- Higher-precedence files can override lower-precedence instructions

**Modular rules** (`.claude/rules/*.md`):

```yaml
---
name: testing-standards
description: Rules for writing tests
paths:
  - "**/*.test.ts"
  - "**/*.spec.ts"
---

Always use vitest for testing.
Mock external API calls with MSW.
Use factory functions for test data, not fixtures.
```

**Import syntax** for referencing external documentation:

```markdown
# Project Documentation
See @README.md for project overview
See @package.json for available npm commands
See @docs/architecture.md for system design
```

Supports up to 5 levels of recursive nesting. First-time external imports trigger an approval dialog.

---

### 6.3 Auto Memory (Agent Self-Learning)

Claude Code v2.1.59+ includes auto memory - Claude writes and updates its own memory based on observations.

**Storage**: `~/.claude/projects/<project>/memory/`

**How it works:**
```
Session: Agent writes code, runs tests, fixes bugs
  |
  +-- Observations silently captured via PostToolUse hooks
  |
  +-- Compressed into structured memory at session end
  |
  +-- Injected into next session's context
```

**Auto memory enables:**
- Project knowledge accumulation (architecture, conventions, preferences)
- Error pattern learning (common mistakes, fixes that worked)
- Decision logging (why certain choices were made)
- File relationship tracking (which files commonly change together)

**Controlling auto memory:**
```bash
# Disable for a session
CLAUDE_CODE_DISABLE_AUTO_MEMORY=1 claude

# Force on
CLAUDE_CODE_DISABLE_AUTO_MEMORY=0 claude
```

---

### 6.4 Memory Retrieval

Claude Code uses an **LLM-based scan** of memory-file headers to select up to 5 relevant files. No embeddings, no vector similarity.

**Memory retrieval process:**
1. List all memory files in the hierarchy
2. Read file headers (first N lines)
3. Ask LLM: "Which of these files are relevant to the current task?"
4. Inject selected files into context

**Alternative: agentmemory (external tool)**

For more sophisticated memory, tools like `agentmemory` provide:
- BM25 + Vector + Graph search (RRF fusion)
- Auto-capture via 12 hooks (zero manual effort)
- 95.2% R@5 on LongMemEval-S benchmark
- Multi-agent memory sharing via MCP
- Token savings: ~170K tokens/year vs ~19.5M+ for full context

**agentmemory architecture:**
```
┌─────────────────────────────────────────────┐
│  Agent (Claude Code / Cursor / Aider)       │
│       │                                     │
│       ▼                                     │
│  Hooks (PreToolUse, PostToolUse, etc.)      │
│       │                                     │
│       ▼                                     │
│  MCP Server ────REST───> Memory Engine       │
│  (local, SQLite + embeddings)               │
└─────────────────────────────────────────────┘
```

---

### 6.5 Decision Logging and Error Pattern Memory

**The `learnings.md` pattern** (project memory layer 4):

```markdown
## 2026-04-18 — Auth Refactor Run

**What worked:**
- Using jose instead of jsonwebtoken for Edge compatibility
- Centralizing auth middleware in src/middleware/auth.ts
- Integration tests caught 3 edge cases unit tests missed

**What to avoid:**
- Don't modify JWT payload structure without updating all consumers
- The legacy auth endpoint can't handle concurrent refresh requests

**Updated rules:**
- Added Edge compatibility requirement to auth.md
- Notified team about legacy endpoint limitation
```

**Task history pattern**:

```markdown
## Session 2026-04-17

**Completed:**
- Migrated 12 API endpoints to new auth pattern
- Updated test suite, all passing

**In progress:**
- Rate limiting middleware (pending: 3 endpoints)

**Decisions:**
- Chose sliding window rate limit over token bucket (simpler for our use case)
- Deferred Redis caching to v2 (PostgreSQL sufficient for current load)
```

**Key principle**: Memory files should focus on **coordination** (how the agent should operate), not personality (how it should sound). Short, well-structured memory files outperform verbose ones.

---

## 7. Prompt Engineering for Coding Agents

### 7.1 System Prompt Patterns

**Claude Code's system prompt structure:**

The system prompt is assembled from multiple sources before every model call:

```
Base system prompt (Anthropic-provided)
  + CLAUDE.md hierarchy (user + project instructions)
  + Tool descriptions (available tools)
  + Dynamic context (current directory, git status)
  = Final system prompt
```

**Key insight from Claude Code's architecture**: CLAUDE.md instructions are delivered as **user context** (probabilistic compliance), not as part of the system prompt (deterministic compliance). This means the model treats them as strong suggestions, not hard rules.

**Effective system prompt patterns:**

1. **Explicit knowledge boundaries** - Define what the agent knows and doesn't know

```
You have access to project documentation dated January 2024.
If asked about features added after this date, explicitly state you don't have
that information rather than speculating.
```

2. **Source citation requirements** - Force the model to cite reasoning sources

```
When providing factual information, always indicate your source:
[from training data], [from provided context], or [inference].
If uncertain, say "I'm not certain, but based on [reasoning]..."
```

3. **Constraints over capabilities** - Define what NOT to do

```
Do NOT mention features unless explicitly listed in documentation.
Do NOT infer features based on similar products.
Do NOT speculate about future capabilities.
```

4. **Structured self-validation** - Build in self-check mechanisms

```
Before providing your final answer:
1. List the key facts your response relies on
2. Rate your confidence in each fact (High/Medium/Low)
3. If any fact is Medium or Low confidence, either remove it or caveat it
```

---

### 7.2 Tool Descriptions That Reduce Hallucination

**The tool description problem**: More tools = more inappropriate selections. Research shows tool-calling hallucinations increase with tool count:

1. **Function selection errors** - Calling non-existent tools
2. **Function appropriateness errors** - Choosing semantically wrong tools
3. **Parameter errors** - Malformed or invalid arguments
4. **Completeness errors** - Missing required parameters
5. **Tool bypass behavior** - Generating outputs instead of calling tools

**Solutions used in production:**

**Semantic tool filtering** (Anthropic Tool Search):

```
Instead of loading all tool definitions upfront:
- Mark tools with defer_loading: true
- Include a "tool search" tool
- Agent searches for relevant tools when needed
- Only matching tools get loaded into context
```

This prevents 50+ tool definitions from consuming thousands of tokens per query.

**Writing effective tool descriptions:**

```json
{
  "name": "Read",
  "description": "Read the contents of a file. Use this when you need to examine code, configuration, or documentation. Prefer reading specific sections over entire large files. Always check if a file exists before reading it.",
  "input_schema": {
    "type": "object",
    "properties": {
      "file": {
        "type": "string",
        "description": "Absolute or relative path to the file to read"
      },
      "offset": {
        "type": "integer",
        "description": "Line number to start reading from (1-indexed). Use this to read specific sections."
      },
      "limit": {
        "type": "integer",
        "description": "Maximum number of lines to read. Use this to avoid reading entire large files."
      }
    },
    "required": ["file"]
  }
}
```

Key principles:
- **Describe WHEN to use the tool**, not just what it does
- **Include negative examples** ("Do NOT use this for...")
- **Specify defaults and constraints** clearly
- **Add inline guidance** in parameter descriptions

---

### 7.3 How to Prompt for File Editing

**Aider's edit format approach** (12 formats, auto-selected):

| Format | Structure | Best For |
|--------|-----------|----------|
| `diff` (default) | SEARCH/REPLACE blocks | Most models, targeted edits |
| `whole` | Full file rewrite | Smaller files, weaker models |
| `udiff` | Unified diff format | Models familiar with patches |
| `architect` | Two-pass: plan -> implement | Complex refactoring |

**SEARCH/REPLACE block format** (Aider's default):

```
<<<<<<< SEARCH
  const x = 1;
  const y = 2;
=======
  const x = 1;
  const y = 2;
  const z = 3;
>>>>>>> REPLACE
```

**Claude Code's approach** (native tool_use):

```json
{
  "type": "tool_use",
  "name": "Edit",
  "input": {
    "file": "src/auth.ts",
    "old_string": "  const x = 1;\n  const y = 2;",
    "new_string": "  const x = 1;\n  const y = 2;\n  const z = 3;"
  }
}
```

**Best practices for edit prompting:**

1. **Use search/replace, not line numbers** - Line numbers drift as files change. Search strings are more robust.
2. **Keep search strings unique** - The search string should match exactly one location in the file
3. **Include enough context** - 2-3 lines of surrounding context in the search string helps disambiguate
4. **One logical change per edit** - Don't combine unrelated changes in one edit block
5. **Verify before applying** - Read the file first, then edit (not the other way around)

**Line numbers vs search/replace tradeoff:**

| Approach | Pros | Cons |
|----------|------|------|
| Line numbers | Precise, unambiguous | Breaks when lines shift |
| Search/replace | Robust to line shifts | Ambiguous if search string appears multiple times |
| Unified diff | Standard format | Harder to apply correctly |
| Full rewrite | Simplest to generate | Expensive for large files, loses unrelated changes |

---

### 7.4 Planning Prompts vs Execution Prompts

**Claude Code Architect mode** (similar to Aider's `--architect`):

The architect model reasons about overall approach without being constrained by file-by-file editing. The editor model takes the plan and makes actual file changes.

```
Phase 1: Planning (reasoning-heavy model)
  "Design the solution. Output a detailed plan with:
   - Files to modify and why
   - New files to create
   - Changes per file (described, not implemented)
   - Test strategy"

Phase 2: Execution (cheaper/faster model)  
  "Implement the following plan. For each file:
   - Read the current content
   - Apply the specified changes using Edit tool
   - Verify the result"
```

**Benefits of plan-then-execute:**
- Complex tasks get coherent global design before implementation
- Avoids inconsistent decisions across files
- Can use expensive model for planning, cheap model for execution
- Plan serves as audit trail

**When to use planning:**
- Multi-file changes (5+ files)
- Architecture decisions (new patterns, migrations)
- Complex refactoring (renaming across codebase)

**When to skip planning:**
- Single-file changes
- Bug fixes with clear scope
- Additive changes (new feature in existing structure)

---

### 7.5 Practical Prompt Engineering Summary

**What works in production:**

| Technique | Effectiveness | Cost |
|-----------|--------------|------|
| Explicit knowledge boundaries | High | Free (prompt engineering) |
| Source citation requirements | High | Free |
| Constraints over capabilities | Very High | Free |
| Semantic tool filtering | High | Low (embedding cost) |
| Search/replace edit format | Very High | Free |
| Plan-then-execute | High | 2x model calls |
| Self-validation steps | Medium | Higher token usage |
| Few-shot examples | High | Higher token usage |

**The #1 rule**: The best prompt engineering for coding agents is giving the agent the **right tools** with **clear descriptions**, not writing clever prompts. A well-designed `Edit` tool with good examples outperforms a 500-word prompt about how to edit files.

---

## Appendix: Architecture Comparison Summary

| Dimension | Claude Code | Aider | OpenClaw |
|-----------|-------------|-------|----------|
| **Loop type** | ReAct while-loop async generator | Single-turn request-response | Pi-agent embedded runner |
| **Subagents** | Task delegation, worktree isolation | None | Configurable nesting (max 5) |
| **Permission system** | 7 modes, deny-first, ML classifier | Git-based rollback | Perimeter-level access control |
| **Context management** | 5-layer compaction pipeline | RepoMap (symbol ranking) | Pluggable compaction providers |
| **Memory** | CLAUDE.md 4-level hierarchy, auto memory | Conventions file (session-only) | MEMORY.md, daily notes, dreaming |
| **Extensibility** | MCP, plugins, skills, hooks | None (focused tool) | 12-capability plugin system |
| **Safety model** | Deny-first + sandboxing + hooks | Git rollback | Gateway auth + sandboxing |
| **Session model** | Resumable by design, append-only | Single-shot | Persistent daemon, multi-channel |
| **Model support** | Anthropic only | 100+ via LiteLLM | Anthropic, OpenAI, Google, Local |

---

## Key Takeaways

1. **The core loop is simple; the infrastructure around it is complex.** Claude Code's `queryLoop()` is a basic while-loop. The sophistication is in the 5-layer compaction, 7-mode permission system, and subagent orchestration.

2. **Safety requires defense in depth.** No single mechanism is sufficient. Claude Code combines deny-first rules, ML classifiers, PreToolUse hooks, optional sandboxing, and graduated trust modes.

3. **Context management is the binding constraint.** Even 1M token windows need compaction. Claude Code's 5-layer graduated pipeline preserves the most information at the lowest cost.

4. **Subagents are about context isolation, not parallelism.** The primary benefit is keeping the parent context clean. Parallelism is a secondary benefit.

5. **Memory should be file-based and human-editable.** CLAUDE.md's markdown hierarchy makes memory inspectable, version-controllable, and easy to update. No opaque databases.

6. **Tool descriptions matter more than system prompts.** Clear `when to use this tool` guidance in tool definitions reduces hallucination more effectively than elaborate system prompts.

7. **The best agents are 1.6% AI, 98.4% infrastructure.** The model provides reasoning. The harness provides safety, context management, extensibility, and reliable execution.

---

## References

1. Liu et al., "Dive into Claude Code: The Design Space of Today's and Future AI Agent Systems," arXiv:2604.14228v1, 2026.
2. Yao et al., "ReAct: Synergizing Reasoning and Acting in Language Models," ICLR 2023.
3. Anthropic, "Claude Code Documentation," https://code.claude.com/docs, 2026.
4. Gauthier, "Aider: AI Pair Programming in Your Terminal," https://aider.chat, 2026.
5. Steinberger, "OpenClaw: Personal AI Infrastructure," https://openclaw.ai, 2026.
6. Anthropic, "Building Effective Agents," https://anthropic.com/engineering, 2025.
7. VILA Lab GitHub: https://github.com/VILA-Lab/Dive-into-Claude-Code
