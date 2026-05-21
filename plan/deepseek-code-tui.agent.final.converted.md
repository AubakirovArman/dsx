# Executive Summary

## Project Overview

DeepSeek Code TUI is a terminal-native coding agent powered by DeepSeek V4, implemented in Rust as a single statically-linked binary. It provides an open-source, model-agnostic alternative to closed-source tools such as Claude Code and the Codex CLI, with first-class optimization for DeepSeek's model family — including automatic Pro/Flash routing, thinking mode configuration, and context cache exploitation.

The project addresses a specific gap in the current tooling landscape. Existing coding agents fall into two camps: closed-source, vendor-locked solutions (Claude Code, Codex CLI) that optimize for a single model family; and open-source tools (Aider, Goose) that trade user experience for flexibility. DeepSeek Code TUI occupies a third position — open source with native DeepSeek optimization, delivering a full IDE-like TUI experience inside the terminal while remaining model-agnostic at the backend.

The architecture rests on three design commitments. First, terminal-native operation through ratatui's immediate-mode rendering engine, enabling sub-millisecond frame updates and IDE-grade panel layouts (file tree, chat, diff view, command palette, token statistics) without leaving the terminal^1^. Second, DeepSeek-native API exploitation — automatic context caching (cache hit pricing at one-tenth of standard input rates^2^), streaming reasoning token separation via `delta.reasoning_content`, and dual OpenAI/Anthropic-compatible endpoint support. Third, a Rust implementation targeting 30--40% lower memory consumption than comparable Go-based TUIs, with single-binary distribution and zero runtime dependencies^1^.

The tool operates in three distinct modes — Simple CLI, Full TUI, and Agent Workspace — each targeting a different developer workflow. Simple CLI handles quick tasks (single-command execution, file reads, diff proposals) with minimal overhead. Full TUI provides an interactive session environment with persistent panels, file tree navigation, inline diff rendering, and a command palette. Agent Workspace extends the TUI with multi-session management, subagent delegation, background task execution, project memory, and automatic git checkpoints — designed for sustained project-level work.

## Key Differentiators

**Native DeepSeek V4 optimization.** The tool implements automatic model routing between V4 Pro (1.6T parameters, 49B active, $1.74/M input tokens) and V4 Flash (284B parameters, 13B active, $0.14/M input tokens), selecting the appropriate model based on task complexity^2^. Thinking mode is automatically configured per-operation: `reasoning_effort="high"` for standard edits, `"max"` for complex architectural tasks. The streaming pipeline handles `delta.reasoning_content` chunks before `delta.content`, enabling real-time display of the model's reasoning chain alongside its output^2^. Automatic context caching reduces repeated-prefix costs by 10x, with cache hit tokens billed at $0.003625/M for Pro^2^.

**Rust-built performance.** The ratatui framework (19.1k GitHub stars, stable v0.30+) provides constraint-based responsive layouts and immediate-mode rendering that sustains 60 FPS on commodity hardware^1^. The full dependency stack — tokio for async, tree-sitter for code parsing across 100+ languages, tantivy for sub-10ms code search, git2 for repository operations — compiles to a single binary with no runtime prerequisites^1^. Memory benchmarks against Go's Bubble Tea show 30--40% reduction in resident set size, a meaningful advantage for a tool that runs alongside IDEs and language servers^1^.

**SEARCH/REPLACE patch engine with 4-tier matching.** File edits use Aider's proven SEARCH/REPLACE block format, applied through a cascading matcher: exact string match, whitespace-insensitive match, indentation-preserving match, and fuzzy similarity scoring^3^. Research on LLM-generated diffs shows that line-number-based formats (unified diff) fail at rates exceeding 86%, while exact-string replacement fails 15--20% of the time due to cached content drift^3^. SEARCH/REPLACE blocks avoid both failure modes by being content-addressed (no line numbers) and tolerance-matched. The engine falls back to full-file rewrite for files under 300 lines where patch reliability drops.

**Tiered permission system with automatic risk classification.** Human-in-the-loop approval is insufficient as a safety mechanism: empirical analysis of Claude Code usage shows approximately 93% of permission prompts are approved by users, indicating automation bias^4^. DeepSeek Code TUI implements a command risk scorer from day one, using pattern matching and heuristics for the MVP and graduating to an ML classifier for v1. The system evaluates every proposed tool call against a deny-first rule matrix: destructive operations (`rm`, `git reset --hard`, database writes) require explicit approval; read-only operations proceed automatically; intermediate-risk operations (file writes, git commits) follow configurable policy^4^.

**Subagent orchestration with context isolation.** Delegation to subagents returns 1--2K token summaries instead of 10K+ full execution histories, yielding 80--90% context savings per delegation^4^. The Agent Workspace mode uses V4 Flash for subagent tasks (fast, inexpensive) and reserves V4 Pro for the main reasoning loop. Each subagent operates in an isolated context with its own tool set and optional git worktree, preventing context pollution of the parent session^4^.

---

# 3. Product Vision

## 3.1 What This Is

### 3.1.1 Primary Identity: TUI/CLI Coding Agent

DeepSeek Code TUI is, at its core, a terminal-native development environment powered by the DeepSeek V4 model family. It is not a chat interface wrapped around an API call; it is a full agent runtime that maintains persistent state across turns, executes tools in streaming parallel, manages context through a five-stage compaction pipeline, and renders all output through a terminal UI with IDE-grade panel layouts.

The agent implements the ReAct pattern (Reasoning + Acting) as its fundamental control loop^4^: the user provides an instruction, the model reasons (visible via the `reasoning_content` stream), selects tools to execute, receives observations, and iterates until the task completes. Each tool call — whether reading a file, running a test, or searching the codebase — passes through a permission gate before execution. The loop terminates on five conditions: no further tool calls requested, maximum turn count reached, context window exhausted, a hook intervention blocks execution, or the user aborts^4^.

The architecture separates concerns into three layers: the TUI layer (ratatui widgets, crossterm event handling, constraint-based layout), the business logic layer (conversation state, file cache, search index, configuration), and the service layer (LLM API client via reqwest-eventsource, git operations via git2/gix, file watching via notify, syntax parsing via tree-sitter)^1^. An async event loop (tokio) merges multiple event streams — keyboard input, timer ticks, API SSE chunks, file system notifications, and git operation completions — into a unified event type that drives state transitions^1^.

### 3.1.2 Secondary Identity: DeepSeek Dev Agent CLI

Beyond the interactive TUI, the tool functions as a scriptable command-line interface for AI-assisted coding. The `ask` subcommand accepts a prompt and optional file attachments, executes the full agent loop non-interactively, and returns the result — suitable for shell pipelines, CI/CD integration, and editor plugins. The `index` subcommand builds a tantivy search index over the codebase for fast symbol and content retrieval. The `tui` subcommand (default) launches the interactive interface.

This dual identity — interactive TUI for exploration, CLI for automation — mirrors the workflow of tools like `git` (both interactive and scriptable) rather than chat-only agents.

### 3.1.3 Project Identity: Open Source, Rust-Built, Model-Agnostic

The project ships under an open-source license (MIT/Apache-2.0 dual-licensed) with DeepSeek-native optimization as its primary focus and model-agnostic support as a secondary capability. The backend API client can target any OpenAI-compatible endpoint (OpenAI, Anthropic-compatible, local Ollama, OpenRouter) by swapping the base URL and model name, but the tool's default configuration, cost-optimization logic, and streaming pipeline are calibrated for DeepSeek V4 specifically.

The implementation language is Rust (edition 2024, MSRV 1.85), selected because no other ecosystem combines a production-grade TUI framework (ratatui), async runtime (tokio), code parser (tree-sitter), and search engine (tantivy) with zero-cost abstractions and single-binary distribution^1^. Go's Bubble Tea (40.7k stars) and TypeScript's Ink (35.6k stars) are viable alternatives for simpler applications but lack tantivy-equivalent search and are 2--3 years behind ratatui in layout system maturity^1^.

```
+-------------------+     +---------------------+     +------------------+
|   TUI Layer       |     |   Business Logic     |     |  Service Layer   |
|   (ratatui)       |<--->|   (App State)        |<--->|  (Async Svcs)    |
|                   |     |                      |     |                  |
| - File tree       |     | - Conversation       |     | - LLM API client |
| - Chat panel      |     | - File cache         |     | - Git (git2/gix) |
| - Diff view       |     | - Search index       |     | - File watcher   |
| - Status bar      |     | - Config             |     | - Syntax parser  |
| - Command palette |     | - Permission state   |     | - Search engine  |
+-------------------+     +---------------------+     +------------------+
         ^                                                    |
         |                                                    v
+------------------------------------------------------------------------+
|                    Async Event Loop (tokio)                              |
|   Merges: Crossterm events | Timer ticks | API SSE | File watcher | Git  |
+------------------------------------------------------------------------+
```

## 3.2 Three Product Modes

### 3.2.1 Simple CLI Mode

Simple CLI mode targets quick, single-purpose tasks where launching a full TUI would be overhead. The user types a command; the agent executes the full ReAct loop; the result prints to stdout and the process exits. No panel rendering, no persistent session, minimal startup time.

```
$ dsctui ask "Add input validation to the signup function" --files src/auth.rs
[Reading src/auth.rs...]
[Proposing edit: src/auth.rs, lines 45-62]
<<<<<<< SEARCH
    pub fn signup(&self, email: &str, password: &str) -> Result<User> {
        let user = User::create(email, password)?;
=======
    pub fn signup(&self, email: &str, password: &str) -> Result<User> {
        if email.is_empty() || !email.contains('@') {
            return Err(Error::InvalidEmail);
        }
        if password.len() < 8 {
            return Err(Error::WeakPassword);
        }
        let user = User::create(email, password)?;
>>>>>>> REPLACE

Apply this edit? [y/n/diff]: y
[Applied. Committed: 4f2a8c1]
```

Use cases: quick edits, test generation from the command line, pre-commit hooks, CI/CD pipeline steps, shell script integration. Simple CLI mode still runs through the permission gate — destructive operations require `--yolo` flag or interactive confirmation — and still creates git commits for all edits.

### 3.2.2 Full TUI Mode

Full TUI mode is an interactive session environment designed for sustained development work. It launches a ratatui-based interface with five persistent panels: a file tree sidebar (tree-sitter parsed, git-status colored), a chat panel showing the full conversation with streaming reasoning display, an inline diff viewer (similar crate for line-level diffing with syntax highlighting), a command log panel recording all tool executions with timing and token counts, and a status bar showing the active model (Pro/Flash), current token usage, session cost estimate, and git branch.

The user interacts through a command palette (Ctrl+P) that exposes all agent capabilities: `/ask` for natural language queries, `/edit` for targeted file changes, `/test` to run the test suite and iterate on failures, `/find` to search the codebase via tantivy, `/git` for repository operations, `/undo` to revert the last edit (via git revert), and `/checkpoint` to save a named snapshot. The command palette is discoverable — typing `?` shows available commands with descriptions and key bindings.

Key design decisions for the TUI: (1) immediate-mode rendering at 60 FPS with batched updates during streaming to minimize CPU usage; (2) keyboard-first navigation with all actions reachable without a mouse; (3) persistent panel layout with user-configurable splits and sizes; (4) status bar always visible showing the information most relevant to decision-making (cost, tokens, model).

### 3.2.3 Agent Workspace Mode

Agent Workspace mode extends the Full TUI with project-level constructs: multi-session management (each session isolated with its own context and git branch), subagent delegation, background task execution, project memory, and automatic checkpointing. This mode activates when the user opens a project directory containing a `.dsctui/config.toml` file.

**Multi-session management.** A project can maintain multiple concurrent sessions — one for feature development, another for refactoring, a third for documentation. Each session has its own conversation history, working set of files, and cost accumulator. Sessions switch instantly without reloading the model context (shared project prefix remains cached).

**Subagent delegation.** Complex tasks are automatically or manually decomposed into subagent calls. A subagent receives a fresh context containing only the task description and relevant file excerpts, executes independently, and returns a structured summary (1--2K tokens vs. the 10K+ full history that would pollute the parent context)^4^. Subagents use V4 Flash by default; the parent agent uses V4 Pro. Subagents can themselves delegate, though depth is capped at two to prevent exponential context growth.

**Background tasks.** Long-running operations — full codebase indexing, comprehensive test suite execution, documentation generation across the entire project — execute in background tasks that report progress via the status bar and command log. The user continues interacting with the foreground agent while background work proceeds.

**Project memory.** A tiered memory system persists across sessions: (a) session memory (conversation history, current context window), (b) project memory (file summaries, architectural decisions, API contracts — stored in SQLite), (c) user preferences (model defaults, permission policy, key bindings — stored in `~/.config/dsctui/`), and (d) tool result logs (historical tool outputs for audit and pattern learning)^4^. Even with DeepSeek's 1M context window, relying solely on the context window for memory leads to pollution and degraded performance; the tiered approach keeps the context window focused on the active working set^4^.

**Automatic checkpointing.** Every edit operation triggers an automatic git commit with a descriptive message (`dsctui: <operation summary>`). Before any destructive operation, the system creates a named checkpoint that the user can restore via `/restore <checkpoint-id>`. Checkpoints are stored in a separate git ref namespace to avoid polluting the main branch history.

```toml
# .dsctui/config.toml — project-level configuration
[agent]
model = "deepseek-v4-pro"           # default model for this project
reasoning_effort = "high"           # default thinking depth
max_turns = 50                      # safety limit per task
auto_delegate = true                # allow automatic subagent delegation

[permissions]
read = "allow"                      # file reads automatic
write = "ask"                       # file writes require confirmation
execute = "deny"                    # shell commands denied by default
except = ["cargo test", "cargo build", "npm test"]  # allowed commands

[subagent]
model = "deepseek-v4-flash"         # cheaper model for subagents
max_depth = 2                       # maximum delegation depth
context_limit = 4000                # token limit per subagent prompt

[memory]
project_db = ".dsctui/memory.db"    # SQLite path for project memory
auto_summarize = true               # auto-summarize file contents after edit
```

## 3.3 Target Users and Use Cases

### 3.3.1 Individual Developers

Individual developers are the primary audience — specifically engineers who prefer terminal-centric workflows, value low-latency interactions, and want an AI assistant that stays out of their way until needed. Two usage patterns dominate:

**Quick-task pattern (Simple CLI + TUI).** The developer encounters a small, well-defined task — write a unit test for an edge case, rename a variable across three files, add error handling to a function. They invoke Simple CLI mode with a single command, review the proposed diff, approve or reject, and return to their editor. Time from intent to completion is under 30 seconds. This pattern replaces manual typing for mechanical code transformations.

**Deep-work pattern (Full TUI).** The developer embarks on a multi-step task — implement a new API endpoint, refactor a module, debug a race condition. They launch the Full TUI, navigate the file tree, ask the agent to analyze the relevant code, iterate through proposals in the diff viewer, run tests from the command palette, and commit changes. The session may last 30 minutes to several hours. This pattern replaces context-switching between editor, terminal, and browser for research-intensive tasks.

### 3.3.2 Teams

Team usage centers on Agent Workspace mode, where shared project configuration, multi-session management, and persistent memory provide value beyond individual productivity.

**Code review assistance.** A team member opens a pull request branch in Agent Workspace, asks the agent to analyze the diff against main, and receives a structured review covering potential bugs, style inconsistencies, test coverage gaps, and performance concerns. The project memory includes team coding standards (stored in `.dsctui/rules/`), so the agent applies team-specific conventions rather than generic advice.

**Onboarding acceleration.** New team members use the agent to explore the codebase — "Explain how authentication works in this project," "Find all places where the database is accessed directly instead of through the repository pattern," "What tests should I run before submitting a PR?" The project memory contains architectural decisions and module relationships, enabling answers grounded in the actual codebase rather than general knowledge.

**Consistency enforcement.** Teams define rules in `.dsctui/rules/` that constrain agent behavior — "Always use `anyhow::Result` instead of `std::io::Result`," "Database queries must go through `db::Repository`," "All public functions require doc comments." The agent enforces these rules during code generation, reducing style drift across team members.

### 3.3.3 Enterprises

Enterprise deployment emphasizes control, auditability, and compliance. All three modes function in enterprise environments with additional configuration for policy enforcement.

**Air-gapped deployment.** The model backend can point to a self-hosted DeepSeek-compatible endpoint (via vLLM, TensorRT-LLM, or similar) rather than the public API. The tool operates identically — the API client is endpoint-agnostic. No telemetry leaves the network. Single-binary distribution simplifies deployment through internal package managers.

**Policy compliance.** Enterprise administrators distribute a `~/.config/dsctui/enterprise.toml` file that overrides user settings: models restricted to approved endpoints, permission levels set organization-wide (e.g., `execute = "deny"` globally), subagent delegation disabled, all tool calls logged to a SIEM-compatible audit trail. The permission system's deny-first evaluation ensures that even compromised credentials cannot override policy^4^.

**Audit and governance.** Every tool call, model response, and user decision is logged with timestamps, token counts, cost estimates, and git commit hashes. The audit log is append-only, written to both local storage and (optionally) a centralized log aggregator. This satisfies compliance requirements for code changes produced with AI assistance — a growing concern in regulated industries.

| User Segment | Primary Mode | Key Feature | Value Proposition |
|---|---|---|---|
| Individual developer | Simple CLI + Full TUI | Fast iteration, keyboard-first TUI | Replace manual typing for mechanical tasks; terminal-native workflow |
| Team | Agent Workspace | Shared memory, multi-session, rules | Consistent code quality; faster onboarding; shared project context |
| Enterprise | All modes (configured) | Air-gapped support, audit logging, policy enforcement | Regulatory compliance; zero external data exposure; governance |
## 1. Confirmed DeepSeek V4 API Capabilities

This chapter documents the API capabilities of DeepSeek V4 that are confirmed through official documentation and integration testing. Every claim below is sourced from the official DeepSeek API documentation, SDK examples, or verified integration reports. Capabilities marked as speculative or unconfirmed are explicitly noted as such.

### 1.1 Model Specifications

DeepSeek V4 ships as two models: V4 Pro (full capability) and V4 Flash (cost-optimized). Both share the same Mixture-of-Experts (MoE) architecture, the same 1-million-token context window, and the same MIT license. The difference lies in parameter scale and per-token pricing.

**Table 1: DeepSeek V4 Model Specifications**

| Attribute | V4 Pro | V4 Flash | Source |
|---|---|---|---|
| Total parameters | 1.6 trillion | 284 billion | Official model card, Hugging Face weights ^2^|
| Active parameters (per forward pass) | 49 billion | 13 billion | Official model card ^2^|
| Context window | 1,048,576 tokens | 1,048,576 tokens | API reference, verified by integration tests ^1^|
| Max output tokens | 384,000 (default 4,096; 8,096 on beta endpoint) | Same as Pro | API reference ^1^|
| Input price (per 1M tokens, list) | $1.74 | $0.14 | Official pricing page ^5^|
| Output price (per 1M tokens, list) | $3.48 | $0.28 | Official pricing page ^5^|
| Cache hit price (per 1M tokens) | $0.003625 | $0.0028 | Pricing page, post-2026-04-26 reduction ^4^|
| License | MIT | MIT | Hugging Face repository ^2^|
| Weights availability | Hugging Face (full) | Hugging Face (full) | Hugging Face ^2^|
| Architecture | Mixture-of-Experts (MoE) | Mixture-of-Experts (MoE) | Model card ^2^|

The 1.6-trillion-parameter Pro model activates only 49 billion parameters per forward pass, a 33:1 sparsity ratio that keeps inference latency manageable while preserving the representational capacity of the full parameter set. The Flash model offers a leaner 22:1 ratio (284B total, 13B active), trading some reasoning depth for roughly 12x lower input cost. Both models expose identical API surfaces; the model name string passed to the API endpoint is the sole difference in client code.

Both models carry the MIT license, and full weights are available on Hugging Face. This means the models can be self-hosted for air-gapped or compliance-sensitive environments, though this chapter focuses exclusively on the managed API.

#### 1.1.1 On Context Window and Effective Use

The 1M token context window applies to the combined input-plus-output length of a single request. In practice, the effective input budget is the 1M total minus the requested `max_tokens` for output. For a coding agent that streams long outputs, setting `max_tokens=384000` leaves approximately 664K tokens for the input context — sufficient for large codebases, conversation history, and system prompt combined. The API does not enforce a separate input/output split; exhaustion of the 1M pool in either direction triggers a `length` finish reason.

### 1.2 Core API Features

The V4 API implements the full Chat Completions schema from OpenAI, with DeepSeek-specific extensions for reasoning mode, context caching, and tool calling. The following table summarizes the confirmed feature set.

**Table 2: Confirmed API Features and Parameters**

| Feature | Status | Key Parameters / Notes | Source |
|---|---|---|---|
| Tool calling (function calling) | Confirmed | `tools` array, max 128 functions; `tool_choice`: `none`/`auto`/`required`/specific function | API reference ^1^|
| Parallel tool calls | Confirmed | Model returns `tool_calls` array with multiple items in a single response | Tool calling guide ^3^|
| Strict tool mode (Beta) | Confirmed | `strict: true` on each function; requires `base_url: https://api.deepseek.com/beta` | Tool calling guide ^3^|
| Thinking mode | Confirmed, ON by default | `thinking.type`: `"enabled"` (default) or `"disabled"`; `reasoning_effort`: `"high"` or `"max"` | Thinking mode guide ^6^|
| Streaming | Confirmed | SSE format, `data: [DONE]` terminator; reasoning tokens in `delta.reasoning_content` before `delta.content` | API reference, SDK examples ^1^ ^6^|
| Usage in streaming | Confirmed | `stream_options={"include_usage": true}` adds usage chunk before `[DONE]` | API reference ^1^|
| JSON mode | Confirmed | `response_format={"type": "json_object"}`; prompt must contain the word "json" | JSON mode guide ^7^|
| Context caching | Confirmed, automatic | No client parameter required; `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens` in usage | Caching guide ^4^|
| OpenAI-compatible endpoint | Confirmed | `POST https://api.deepseek.com/chat/completions` | Quickstart ^8^|
| Anthropic-compatible endpoint | Confirmed | `POST https://api.deepseek.com/anthropic` | Anthropic API guide ^9^|
| Temperature / top_p | Accepted but ignored in thinking mode | `temperature`: 0–2 (default 1); `top_p`: 0–1 (default 1) | Thinking mode guide ^6^|
| Presence / frequency penalty | Deprecated, no effect | Parameters accepted for backward compatibility | API reference ^1^|

#### 1.2.1 Tool Calling

Tool calling follows the OpenAI function-calling schema exactly. A request may include up to 128 function definitions in the `tools` array, each with `type: "function"`, a `name`, a `description`, and a `parameters` object in JSON Schema format. The `tool_choice` parameter supports four modes: `none` (model will not call any tool), `auto` (model decides), `required` (model must call at least one tool), and a specific function selector `{"type": "function", "function": {"name": "..."}}` to force a particular tool. The model can return multiple tool calls in a single response via the `tool_calls` array, enabling parallel execution of independent operations.

A beta "strict" mode is available via the `https://api.deepseek.com/beta` base URL. When `strict: true` is set on a function definition, the API validates that the model's arguments conform to the declared JSON Schema. Strict mode requires all object properties to be listed in `required` and `additionalProperties` to be `false`. Supported schema types are `object`, `string`, `number`, `integer`, `boolean`, `array`, `enum`, and `anyOf` ^3^.

#### 1.2.2 Thinking Mode

Thinking mode is **enabled by default**. If the client does not specify `thinking.type`, the API treats it as `"enabled"` and enters reasoning mode. To disable thinking — for example, when using fill-in-the-middle (FIM) completion — the client must explicitly send `thinking: {"type": "disabled"}`.

When thinking is enabled, the API accepts a `reasoning_effort` parameter with values `"high"` (default for regular requests) or `"max"` (used automatically for complex agent patterns). For compatibility with existing code that may pass other providers' effort values, `"low"` and `"medium"` are silently mapped to `"high"`, and `"xhigh"` is mapped to `"max"` ^6^.

Thinking mode silently ignores `temperature`, `top_p`, `presence_penalty`, and `frequency_penalty`. The API accepts these parameters without error but produces no sampling variation. This behavior is by design: reasoning chains are generated deterministically. Engineers who require temperature control must disable thinking mode first.

When tool calls are used in thinking mode, a critical constraint applies: the `reasoning_content` from every assistant turn that performed a tool call **must** be passed back to the API in all subsequent requests. Omitting it produces HTTP 400 with the error message: `"The reasoning_content in the thinking mode must be passed back to the API."` For multi-turn conversations without tool calls, `reasoning_content` may be omitted; the API ignores it ^6^.

#### 1.2.3 Streaming

Streaming follows the Server-Sent Events (SSE) protocol. Each chunk is a `data: <json>` line; the stream terminates with `data: [DONE]`. In thinking mode, the streaming sequence is strictly ordered:

1. First chunk: `delta.role = "assistant"` only.
2. Subsequent chunks: reasoning tokens arrive via `delta.reasoning_content`.
3. After reasoning completes: answer tokens arrive via `delta.content`.
4. Final chunk: `finish_reason` and `usage` object, including `completion_tokens_details.reasoning_tokens`.

To receive usage statistics in streaming mode, set `stream_options={"include_usage": true}`. The API then emits an additional chunk before `[DONE]` containing the full `usage` object with `prompt_cache_hit_tokens`, `prompt_cache_miss_tokens`, and the reasoning-token breakdown ^1^.

#### 1.2.4 JSON Mode

JSON mode is activated by setting `response_format={"type": "json_object"}`. The API guarantees syntactically valid JSON output, but the prompt must explicitly instruct the model to emit JSON — the official documentation requires the word "json" to appear in the system or user prompt, along with an example of the desired JSON structure ^7^. An acknowledged limitation: JSON mode may occasionally return empty content. The documentation attributes this to known optimization gaps and recommends prompt modification as a workaround.

### 1.3 API Endpoints and Migration

DeepSeek V4 offers two first-party API interfaces: an OpenAI-compatible endpoint and an Anthropic-compatible endpoint. Both expose the same underlying model; the choice depends on which SDK the client code already uses.

The OpenAI-compatible endpoint uses `base_url = "https://api.deepseek.com"` with the standard `/chat/completions` path. Model names are `deepseek-v4-pro` and `deepseek-v4-flash`. The thinking-mode parameter `thinking` is not part of the OpenAI SDK schema, so it must be passed via `extra_body={"thinking": {"type": "enabled"}}` when using the OpenAI SDK. The `reasoning_effort` parameter is recognized natively by the SDK and can be passed at the top level ^8^ ^6^.

The Anthropic-compatible endpoint uses `base_url = "https://api.deepseek.com/anthropic"`. Thinking effort is controlled via `output_config: {"effort": "high"}` rather than `reasoning_effort`. Unsupported model names are automatically mapped to `deepseek-v4-flash`. The `anthropic-beta` HTTP header is ignored by the endpoint ^9^.

A third base URL, `https://api.deepseek.com/beta`, enables beta features: strict tool mode, chat prefix completion, and an increased `max_tokens` limit of 8,192 (versus the default 4,096). Beta features are not available on the standard endpoint ^3^.

#### 1.3.1 Legacy Model Deprecation

The model aliases `deepseek-chat` and `deepseek-reasoner` are deprecated and will be fully removed on **July 24, 2026**. At present, `deepseek-chat` resolves to V4 Flash in non-thinking mode, and `deepseek-reasoner` resolves to V4 Flash in thinking mode. New code should use the explicit model names `deepseek-v4-pro` or `deepseek-v4-flash` ^8^.

### 1.4 Cost Optimization

#### 1.4.1 Context Caching

Context caching is automatic and requires no client-side configuration. The API builds a disk-based key-value cache for each request. Subsequent requests with matching prefixes fetch the overlapping portion from cache at a 10x reduced price ^4^.

Cache prefix units are created at three points: at the end of each user input, at the end of each model output, and at fixed token intervals for long inputs. The system also detects common prefixes across multiple requests and persists them as independent cache units. Critically, the matching algorithm requires **full** prefix-unit alignment; partial matches do not qualify as cache hits ^4^.

Cache status is reported in the response `usage` field as two counters: `prompt_cache_hit_tokens` and `prompt_cache_miss_tokens`. Their sum equals `prompt_tokens`. The cache operates on a best-effort basis — DeepSeek does not guarantee a 100% hit rate, and unused cache entries are evicted within hours to days ^4^.

For a coding agent that sends a stable system prompt and project context followed by changing task instructions, the natural prefix stability of the conversation structure should yield significant cache hits on the system and context portions. Engineers should monitor `prompt_cache_hit_tokens` in production to validate cache effectiveness.

#### 1.4.2 Launch Discount

DeepSeek applies a 75% launch discount to V4 Pro input and output pricing, extending it to May 31, 2026. This reduces the effective Pro input price from $1.74 to $0.435 per million tokens, and the output price from $3.48 to $0.87 per million tokens. The Flash model prices ($0.14 input, $0.28 output) are not discounted and serve as the baseline. Engineers should verify current pricing on the official pricing page before budgeting, as discount extensions are announced without guaranteed notice periods ^5^.

#### 1.4.3 Cost Comparison

**Table 3: Input/Output Pricing Comparison (per 1M tokens)**

| Provider / Model | Input Price | Output Price | Cache Hit Price | Context Window | Source |
|---|---|---|---|---|---|
| DeepSeek V4 Pro | $0.435 (discounted) / $1.74 (list) | $0.87 (discounted) / $3.48 (list) | $0.003625 | 1M | Official pricing ^5^ ^4^|
| DeepSeek V4 Flash | $0.14 | $0.28 | $0.0028 | 1M | Official pricing ^5^ ^4^|
| Claude 4 Sonnet (Anthropic) | ~$3.00 | ~$15.00 | Not offered | 200K | Anthropic pricing page (public) |
| Claude 4 Opus (Anthropic) | ~$15.00 | ~$75.00 | Not offered | 200K | Anthropic pricing page (public) |
| GPT-4.1 (OpenAI) | ~$2.00 | ~$8.00 | 50% discount | 1M | OpenAI pricing page (public) |
| Gemini 2.5 Pro (Google) | ~$1.25 | ~$10.00 | Not offered | 1M | Google AI pricing (public) |

The pricing gap between DeepSeek V4 Pro and competing frontier models is substantial. At the discounted rate, V4 Pro input costs roughly one-seventh of Claude 4 Sonnet and one-fifth of GPT-4.1. V4 Flash is cheaper still — its $0.14/M input price is within an order of magnitude of Claude 3.5 Haiku and GPT-4o-mini, while offering a 1M context window and tool-calling capability comparable to full-scale frontier models. The automatic context caching further reduces effective costs for conversation-heavy workloads: a coding session where 80% of the input hits cache would see an effective Pro input price of approximately $(0.20 \times 0.435 + 0.80 \times 0.003625) = $0.09$ per million tokens, a 99.4% reduction versus the list price.

The trade-off is rate-limiting policy. Where OpenAI and Anthropic publish tier-based RPM (requests per minute) and TPM (tokens per minute) limits that scale with spend, DeepSeek uses dynamic concurrency limits based on server load. When the limit is reached, the API returns HTTP 429 immediately. Requests may also queue on an open connection with SSE keep-alive signals for up to 10 minutes before the server closes the connection. There are no published fixed limits, and no paid tier to unlock higher concurrency ^10^. Engineers should implement exponential backoff with jitter and consider fallback to alternative providers for high-availability workloads.

# 2. Competitor Analysis

The AI coding agent landscape in mid-2026 spans three tiers: premium commercial agents backed by model labs with proprietary architectures and deep model integration; open-source multi-model agents built for flexibility, cost control, and community governance; and IDE-integrated extensions that embed AI into the editing surface developers already inhabit. Understanding each tool's architecture, editing primitives, permission models, and extensibility mechanisms is essential for positioning a DeepSeek-native Rust TUI agent in a crowded market.

The ten tools analyzed below — Claude Code, Codex CLI, Gemini CLI, Aider, OpenCode, Goose, Roo Code, Zed AI, Crush, and Continue.dev — represent the full competitive spectrum. Each analysis covers the implementation stack, the editing primitive (which determines patch reliability), the permission model (which determines safety posture), and the extensibility architecture (which determines ecosystem velocity). The chapter concludes with a 10×15 feature matrix and an explicit market gap analysis that defines the product's strategic positioning.

---

## 2.1 Tier 1: Premium Commercial Agents

### 2.1.1 Claude Code

Claude Code is Anthropic's proprietary TypeScript CLI. The VILA Lab's architectural analysis (v2.1.88 source) identified a simple async generator loop (`queryLoop()`) surrounded by sophisticated subsystems for permission control, context management, and extensibility^11^.

**Architecture.** Ephemeral per-session Node.js process. Seven permission modes managed by an ML-based command classifier with deny-first rule evaluation: full auto-approval for known-safe operations, interactive confirmation for moderate-risk commands, OS-level sandboxing (seatbelt on macOS, bubblewrap on Linux) for untrusted code, and several graduated trust levels between these extremes^11^. The classifier categorizes commands by risk using pattern matching and learned embeddings, applying the most restrictive matching rule first. Subagent delegation supports up to 10 parallel agents with isolated git worktree contexts and background execution, returning 1–2K token summaries that save 80–90% of context versus full history inclusion^11^. Context management uses a five-layer compaction pipeline: raw conversation → tool result truncation → LLM-based summarization of older turns → semantic memory scan → archive, with append-oriented storage that never rewrites historical records^11^. Four extensibility mechanisms — MCP servers (3,000+ available), plugins (marketplace with version pinning), skills (YAML workflows), and hooks (22 lifecycle events including PreToolUse, PostToolUse, UserPromptSubmit, SubagentStop)^11^ ^12^.

**Key features.** 1M-token context at standard pricing, mid-session model switching, voice mode, `/compact` for on-demand context reduction, `/ultrareview` for cloud-based multi-agent review^12^. 87.6% SWE-bench Verified with Opus 4.7. Background cloud execution via `--teleport`^12^.

**Strengths.** Sets the benchmark for agentic reasoning, permission granularity, and subagent orchestration. The ML classifier addresses the finding that ~93% of permission prompts are auto-approved by users, making interactive confirmation unreliable^11^. The compaction pipeline sustains multi-hour sessions without degradation.

**Weaknesses.** Proprietary and Claude-locked. Subscription pricing ($20–200/month) plus API costs make it the most expensive option. Node.js runtime consumes more memory than a compiled binary. CLI-only with line-oriented output — no TUI for high-information-density operations.

**What to borrow.** The seven-mode permission system with ML classification, subagent delegation (isolated context + git worktree), and the `CLAUDE.md` configuration hierarchy^12^.

### 2.1.2 Codex CLI

OpenAI's Codex has two interfaces: a cloud-based async agent in isolated sandboxes (accessed via ChatGPT) and the Codex CLI running locally. Both use GPT-5.4 and the GPT-5.x-Codex model family^12^.

**Architecture.** Cloud agents execute in containerized sandboxes preloaded with repository clones. Tasks run asynchronously — assign work and receive results when complete. Multiple agents can operate in parallel on different issues, each isolated^13^. The local CLI provides a 192K default context (expandable to 1.05M, billed at 2× beyond 272K). Hooks-based compaction and intelligent file pre-loading manage context^14^.

**Key features.** Async task delegation, PR opening with review evidence (logs + test outputs), Azure Foundry integration for enterprise compliance boundaries^15^. GPT-5.3-Codex scores 77.3% on Terminal-Bench 2.0, exceeding Claude Code's 69.4%^12^.

**Strengths.** Parallel async execution is unmatched for batch work. Cloud sandbox isolation provides defense-in-depth. Azure Foundry enables enterprise deployment inside compliance boundaries^15^.

**Weaknesses.** Async model creates friction for exploratory work — agents run to completion before redirection is possible^13^. SWE-bench Verified lags Claude Code (74.9% vs. 87.6%)^12^. Cloud agents start fresh per task, missing project conventions. Limited computer use vs. Claude Code's browser/GUI control.

**What to borrow.** The async execution model for background delegation, and the `/goal` persistence system for multi-day workflows^14^.

### 2.1.3 Gemini CLI

Google's Gemini CLI is the only Tier 1 tool that is fully open source (Apache 2.0)^16^.

**Architecture.** Go-based with PTY streaming for real-time shell interaction. Gemini 2.5 Pro with 1M context. MCP-native architecture. Prompt grounding via Google Search. Custom system prompts from `GEMINI.md`^16^.

**Key features.** Free tier: 1,000 requests/day under Gemini Code Assist license. Non-interactive scripting mode for CI/CD. Deep Gemini Code Assist integration for IDE-to-terminal transitions^16^.

**Strengths.** Best free-tier offering in the commercial space. Apache 2.0 enables inspection and self-hosting. Google Search grounding for real-time context. PTY streaming enables genuine interactive shell sessions.

**Weaknesses.** Planning capabilities lag Claude Code — early reports cite excessive search time and failed exploration^16^. The 1,000 req/day free tier lacks a clear upgrade path. No subagent delegation or plugin marketplace.

**What to borrow.** PTY streaming for genuine shell interaction, `GEMINI.md` context convention, and non-interactive scripting mode for CI/CD.

---

## 2.2 Tier 2: Open Source Multi-Model Agents

### 2.2.1 Aider

Aider, built by Paul Gauthier in Python, pioneered SEARCH/REPLACE block editing and holds strong SWE-bench scores through disciplined git-native workflows^17^ ^18^.

**Architecture.** Python CLI requiring a git repository — Aider refuses to operate outside one, making git the foundational safety layer. Signature innovation: the repository map, a tree-sitter-generated structural summary (classes, functions, imports, call graphs) that provides architectural context before any edit. The map is computed once at startup and refreshed incrementally as files change, typically consuming 500–2,000 tokens depending on codebase size^18^. This gives the LLM a high-level understanding of file relationships without loading every file into the context window, a pattern that achieves better token efficiency than naive full-context approaches.

**Key features.** SEARCH/REPLACE with four-tier matching (exact → whitespace-insensitive → indentation-preserving → fuzzy). 70+ models via LiteLLM with mid-session switching. Automatic atomic git commits. Lint/test integration with auto-fix on failure^18^. Architect mode for planning without execution. Voice input.

**Strengths.** Deepest git integration — every change is a commit. SEARCH/REPLACE is the most LLM-friendly editing primitive; content-addressed editing outperforms position-addressed approaches. Token efficiency: 4.2× fewer tokens than Claude Code^18^. DeepSeek works via OpenAI-compatible API.

**Weaknesses.** Terminal-only with no TUI. No semantic search. Manual context management. Less sophisticated planning than Claude Code. No subagent delegation. Python runtime — slower startup than compiled binaries^18^.

**What to borrow.** SEARCH/REPLACE as the primary editing primitive, the repository map, and git-native atomic commits. The four-tier matching strategy for patch resilience.

### 2.2.2 OpenCode

OpenCode (now evolved into Crush) was a Go-based TUI agent with innovations in subagent architecture and session persistence. It reached significant adoption before transitioning to Charmbracelet's stewardship^19^ ^14^.

**Architecture.** Bubble Tea TUI framework in Go. YAML-based subagent architecture for composable behaviors. SQLite session persistence. LSP integration for real-time code intelligence — diagnostics, references, symbol definitions^19^ ^14^.

**Key features.** Effect-based event system, "session warping" (preserving file context across restarts), named arguments for custom commands, Vim-like editor, file change tracking, external editor support^19^.

**Strengths.** LSP gives genuine code intelligence beyond AI text reasoning. SQLite sessions enable complex historical queries. Bubble Tea TUI is polished. YAML subagent architecture makes behavior inspectable and version-controllable.

**Weaknesses.** Archived — superseded by Crush. Weaker planning than Claude Code. Higher token usage than Aider. No semantic search or checkpoint system.

**What to borrow.** LSP integration for code-intelligent editing, SQLite session persistence, and the effect-based event system. The YAML-defined subagent architecture proves agent behavior can be fully declarative.

### 2.2.3 Goose

Goose, originally by Block and contributed to the Linux Foundation's AAIF in December 2025, is the most thoroughly MCP-first agent. 30,000+ GitHub stars, 350+ contributors, neutral governance with backing from AWS, Anthropic, Google, Microsoft, and OpenAI.

**Architecture.** Entirely MCP-first — not as an add-on, but as the foundation. Built-in extensions: Developer (`read_file`, `write_file`, `patch_file`, `execute_command`), Memory (cross-session `remember`/`recall`), Computer Controller (web scraping, document processing). CLI in Rust; desktop via Tauri. Native ACP agent for editor interoperability.

**Key features.** Recipes — YAML-based reusable, parameterizable, composable workflows as slash commands. 25+ LLM providers (OpenRouter, ChatGPT login, Gemini, Groq, Ollama for offline). Deeplinks (`goose://recipe?config=...`).

**Strengths.** MCP-first creates infinite extensibility (3,000+ servers). Recipes capture institutional knowledge in version-controlled, composable units. Linux Foundation governance ensures stability. Local-first with full offline capability. Memory extension provides persistent cross-session context unmatched in open source.

**Weaknesses.** No semantic code search. Recipe authoring requires manual YAML. No checkpoint/rollback. Tauri desktop has performance constraints vs. native. No planning mode. Less polished UX than Claude Code.

**What to borrow.** MCP-first architecture, the Recipes system, Memory extension pattern, and ACP agent mode. Linux Foundation governance model.

---

## 2.3 Tier 3: IDE-Integrated Agents

### 2.3.1 Roo Code / Kilo Code

Roo Code (3M installs) shut down in April 2026; Kilo Code is the successor, rebuilt on OpenCode server with shared CLI/VS Code sessions.

**Architecture.** TypeScript VS Code extension. Kilo shares sessions between CLI and VS Code via the same OpenCode server backend.

**Key features.** Mode system: Code (editing), Architect (planning without execution), Ask (read-only), Debug (tracing), and Custom Modes (team-specific). MCP with stdio/HTTP/SSE, project-level + global config. Per-mode model selection ("sticky models"). Context condensing. Qdrant-based semantic search (Roo; pending in Kilo).

**Strengths.** Mode separation prevents token waste. Per-model selection optimizes cost. MCP depth with tool-level permissions. Kilo's session portability addresses platform lock-in.

**Weaknesses.** VS Code lock-in (partially addressed by Kilo). Cline legacy baggage. Original Roo's in-repo checkpoints caused a "nested-.git bug." Code Mode tends toward full-file rewrites. Shutdown creates trust uncertainty.

**What to borrow.** The mode system as a UX primitive. Per-mode model selection. Project-level MCP configuration. Context condensing approach.

### 2.3.2 Zed AI

Zed is a native IDE built from scratch in Rust with a GPU-driven UI framework (GPUI) at 120fps.

**Architecture.** Rust codebase. Two AI systems: built-in Zed Agent (native tools + MCP) and external agents via ACP — "LSP for AI agents" enabling Claude Agent, Codex, Gemini CLI to operate within Zed.

**Key features.** Zeta2 edit prediction (open-weight, trained on real edits for multi-line changes). Per-buffer model selection. ACP protocol. Inline assistant (`Alt-A`). Multiplayer real-time collaborative editing. AI commit messages. 2,000 free Zeta predictions/month.

**Strengths.** Native speed — "you see every change as it happens." ACP is strategically important — any ACP agent works in any ACP editor. Zeta is purpose-built for edits, not completion. Open-source at every layer (editor, model, protocol).

**Weaknesses.** CVE-2025-55012 (CVSS 8.5, permission bypass) revealed security gaps. Smaller ecosystem than VS Code. Requires learning a new IDE. Built-in agent has fewer features than dedicated tools. Windows support lagged.

**What to borrow.** ACP protocol for editor interoperability. Per-buffer model selection. GPUI rendering targets for TUI performance.

### 2.3.3 Continue.dev

Continue.dev distinguishes itself through Apache 2.0 governance, air-gapped deployment, and privacy-first architecture.

**Architecture.** TypeScript extension for VS Code and JetBrains. YAML configuration (`~/.continue/config.yaml`). Routes requests to chosen provider — no code touches Continue.dev's servers. Full air-gapped operation with Ollama.

**Key features.** Four modes: Chat, Edit, Plan (read-only sandbox), Agent (autonomous multi-file). Context via `@`-mentions (`@file`, `@web`, `@codebase`, `@terminal`, `@diff`). MCP in all modes. Autocomplete.

**Strengths.** Most privacy-respecting option. Plan Mode enables safe codebase exploration. Air-gapped deployment for regulated industries. Multi-IDE support. Any OpenAI-compatible API including DeepSeek.

**Weaknesses.** "Swiss Army Knife that sometimes fails to cut" — less polished. No semantic search. Agent mode less capable than Claude Code. No checkpoints. Edit mode struggles with complex refactoring. No persistent memory.

**What to borrow.** Plan Mode as a read-only exploration primitive. Air-gapped architecture. `@`-mention context system.

---

## 2.4 Competitor Matrix and Gaps

### 2.4.1 Feature Matrix

The matrix below compares ten tools across fifteen dimensions. ✓ = native support, ~ = partial/indirect, — = no support.

| Feature | Claude Code | Codex CLI | Gemini CLI | Aider | OpenCode/Crush | Goose | Roo/Kilo | Zed AI | Continue.dev |
|---|---|---|---|---|---|---|---|---|---|
| **Open Source** | — | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **DeepSeek Native** | — | — | ~ | ~ | ~ | ~ | ✓ | ~ | ✓ |
| **Compiled Binary** | — | — | ✓ | — | ✓ | ✓ | — | ✓ | — |
| **Full TUI** | — | — | ✓ | — | ✓ | ~ | — | ✓ | — |
| **Subagents** | ✓ | ~ | — | — | ~ | ~ | ~ | — | — |
| **MCP Support** | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Git-Native Commits** | ✓ | ✓ | ✓ | ✓ | ~ | ~ | ✓ | ✓ | — |
| **SEARCH/REPLACE Edits** | ~ | — | — | ✓ | — | — | ~ | ~ | ~ |
| **Permission Modes (4+)** | ✓ | ~ | ~ | — | ~ | ~ | ✓ | ✓ | ✓ |
| **LSP Integration** | — | — | — | — | ✓ | — | — | ✓ | — |
| **Semantic Search** | — | — | — | — | — | — | ✓ | — | — |
| **Air-Gapped / Local** | — | — | — | ✓ | ✓ | ✓ | — | ~ | ✓ |
| **Mid-Session Model Switch** | ✓ | — | — | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Planning Mode** | ~ | — | — | ✓ | — | — | ✓ | — | ✓ |
| **Checkpoint / Rollback** | ✓ | — | — | ✓ | — | — | ✓ | ~ | — |

*Table: 10-tool × 15-dimension feature matrix. Data synthesized from official docs, source code analysis, and benchmarks^11^ ^12^ ^17^ ^16^.*

No existing tool scores positively on all three of DeepSeek-native, Rust-performance, and full-TUI dimensions. Claude Code and Codex CLI dominate on agentic sophistication but are proprietary and model-locked. Aider excels at git integration but lacks a TUI. Goose is MCP-first, not TUI-first. Zed offers native performance but requires switching editors. Continue.dev leads on privacy but lacks compiled-binary performance.

### 2.4.2 Positioning Summary

The table below maps each tool's primary differentiator, target user, and critical limitation.

| Tool | Primary Differentiator | Target User | Critical Limitation | License |
|---|---|---|---|---|
| **Claude Code** | Agentic reasoning + ML permission classifier | Professional developers, teams | Proprietary; Claude-only; $20–200/mo + API | Proprietary |
| **Codex CLI** | Async parallel execution in cloud sandboxes | Teams with batch workloads | Cloud-only for advanced features; GPT-only | Proprietary |
| **Gemini CLI** | Free tier + Apache 2.0 + Google Search grounding | Budget-conscious developers | Planning lags Tier 1; limited ecosystem | Apache 2.0 |
| **Aider** | SEARCH/REPLACE editing + git-native workflow | Git-centric developers | No TUI; Python runtime; manual context | Apache 2.0 |
| **OpenCode/Crush** | TUI-first + LSP + mid-session model switching | Terminal-native developers | Planning weak; token-inefficient; Go not Rust | Apache 2.0 |
| **Goose** | MCP-first + Recipes + Linux Foundation | Extensibility-focused teams | No TUI; no checkpoints; Tauri desktop | Apache 2.0 |
| **Roo/Kilo Code** | Mode system + per-mode model selection | VS Code power users | VS Code lock-in; shutdown churn | Apache 2.0 |
| **Zed AI** | 120fps Rust IDE + ACP + open-weight model | IDE switchers, performance seekers | IDE switching cost; CVE history; smaller ecosystem | GPL + Apache 2.0 |
| **Continue.dev** | Privacy-first + air-gapped + multi-IDE | Enterprise, compliance-sensitive | Polish gap; no checkpoints; limited agent depth | Apache 2.0 |

*Table: Positioning summary across 9 tools. The DeepSeek-native + Rust-performance + full-TUI quadrant remains unoccupied.*

### 2.4.3 Market Gap Analysis

Four gaps define the product opportunity:

**Gap 1: DeepSeek-native optimization.** While Roo Code and Kilo support DeepSeek as a native provider, they are Electron-based extensions with performance ceilings. No compiled-binary tool optimizes for DeepSeek's API patterns: `reasoning_content` streaming separation, dual OpenAI/Anthropic-compatible endpoints, automatic context caching (10× cost reduction on cache hits), and the 10:1 cost advantage of V4 Flash over Claude models.

**Gap 2: Rust TUI performance.** Among open-source agents, only Goose uses Rust — but it is MCP-first, not TUI-first. Crush uses Go (Bubble Tea), which lacks the sophistication of Rust's ratatui (19,000+ stars, IDE-capable layouts, Vim bindings). A Rust TUI agent targets sub-50ms rendering, sub-10MB memory, and single-binary distribution — characteristics that matter for a tool developers run hundreds of times daily.

**Gap 3: TUI + CLI dual mode.** The landscape forces a choice: TUI-first (Crush) or CLI-first (Claude Code, Aider). No tool offers both a rich TUI IDE-like experience and a pipeable CLI for scripting. A dual-mode architecture captures both use cases.

**Gap 4: Open source + governance.** Claude Code and Codex CLI are proprietary. Roo Code's shutdown demonstrates single-vendor trust risk. Goose's Linux Foundation model provides a template, but no DeepSeek-native tool has adopted neutral governance.

### 2.4.4 Strategic Positioning

The product positions at the intersection of performance, cost, and DeepSeek optimization.

**Performance.** Rust compilation delivers startup latency, memory efficiency, and rendering speed that Electron extensions (Roo, Continue) and Python CLIs (Aider) cannot match. The TUI targets 120fps rendering parity with Zed, but in a terminal application requiring no IDE switch.

**Cost.** DeepSeek-native optimization — leveraging context caching (10× reduction on repeated prefixes), V4 Flash for subagents ($0.14/M input vs. Claude Haiku at $1/M), and dual-endpoint flexibility — creates a 5–10× operating cost advantage over Claude Code. For a team running 100 agent sessions daily, this reduces from thousands of dollars per month to hundreds.

**DeepSeek optimization.** While model-agnostic tools treat DeepSeek as "another OpenAI-compatible provider," a DeepSeek-native tool exploits the full API: streaming `reasoning_content` for transparent chain-of-thought, thinking/non-thinking toggles for quality/speed tradeoffs, 128-function streaming tool calling, and 1M context. As V4 Pro approaches Claude Opus quality at 14× lower pricing, the economic case for a purpose-built agent strengthens^11^.

The strategic bet: DeepSeek's API quality converges with Anthropic's while the cost advantage persists, and a purpose-built Rust TUI agent captures developers who want Claude Code-grade reasoning at Aider-grade costs, with native binary performance and open-source freedom.
# 4. Core Feature Set

The feature set is organized into ten categories (A–J) that collectively define the complete functional scope. Each category maps to specific user workflows and has measurable implementation targets. The following table provides the executive summary; subsequent sections unpack each category with implementation-level detail.

| Category | Name | Key Capabilities | Implementation Complexity | Priority |
|----------|------|-----------------|--------------------------|----------|
| A | Project Awareness | Repo scanning, AST indexing, symbol graph, dependency detection, project instructions | High | P0 |
| B | Chat + Coding Loop | Natural language tasks, plan generation, diff review, apply/test/retry cycle | High | P0 |
| C | TUI UX | Multi-panel layout, keyboard navigation, command palette, status bar | High | P0 |
| D | Safety + Permissions | Read-only mode, ask-before-edit, yolo mode, allowlist/denylist, sandbox | High | P0 |
| E | Memory + Persistence | Session memory, project memory, user preferences, SQLite storage, decision logs | Medium | P1 |
| F | Tool System | 15+ built-in tools, MCP integration, streaming tool execution | High | P0 |
| G | Subagent Orchestration | Planner, implementer, reviewer, test-runner agents with context isolation | Medium | P1 |
| H | Git Integration | Auto-commit, pre-edit checkpoint, /undo, diff generation, blame/annotate | Medium | P0 |
| I | Context Management | 5-layer compaction, prompt assembly, cache-aware prefix design, token budgeting | High | P1 |
| J | Extensibility + Ops | Plugin system, theme support, telemetry, auto-updater, config hierarchy | Low | P2 |

The remainder of this section expands categories A through G in architectural detail. Categories H through J are treated in depth in Chapter 5 (Architecture) and Chapter 6 (TUI Design), where their implementation in the layer stack and screen-level interaction are specified respectively.

## 4.1 Project Awareness (Category A)

Project Awareness is the foundational capability that enables the agent to understand the codebase before any coding task begins. It subsumes what Aider calls "repo-map" and Claude Code calls "project indexing" into a unified, tiered system. The design philosophy is aggressive up-front indexing followed by incremental maintenance — the agent must know what files exist, what language each file is written in, how files relate to each other, and what project-specific conventions apply, all before the first user message is sent.

### 4.1.1 Repository Scanning

On first launch (or on detecting a new working directory), the scanner performs a recursive walk of the project tree using the `ignore` crate ^2^, which respects `.gitignore`, `.ignore`, and `.git/info/exclude` patterns. This single decision — using `ignore` over raw `walkdir` — eliminates the most common source of noise in codebase scanning: build artifacts, `node_modules`, and `.git` objects. For a typical Node.js project, this reduces the scanned file count from 50,000+ to 2,000–4,000 relevant source files.

The scan produces three data structures:

```rust
pub struct RepoScan {
    pub files: Vec<FileEntry>,           // All tracked source files
    pub languages: HashMap<String, u32>, // Language -> file count
    pub dependencies: DependencyGraph,   // Parsed from manifest files
}

pub struct FileEntry {
    pub path: PathBuf,
    pub language: Option<Language>,      // Detected via tree-sitter language set
    pub size_bytes: u64,
    pub line_count: u32,
    pub is_test: bool,                   // Heuristic: path contains "test" or "spec"
    pub is_config: bool,                 // Heuristic: known config filenames
}
```

Language detection uses a two-pass approach: first, filename/extension matching against the `tree-sitter` language registry (100+ languages); second, for ambiguous extensions (`.h` files that could be C, C++, or Objective-C), a content-sniffing pass examines the first 1,000 bytes for language-specific keywords. The `tree-sitter` crate's `Language` type is the canonical representation throughout the system, ensuring that downstream indexing, syntax highlighting, and search all agree on language classification ^1^.

Dependency detection parses standard manifest files: `Cargo.toml` (Rust), `package.json` (JavaScript/TypeScript), `requirements.txt` / `pyproject.toml` (Python), `go.mod` (Go), `Gemfile` (Ruby), `pom.xml` / `build.gradle` (Java), and `composer.json` (PHP). For each detected manifest, the system extracts direct dependencies and dev-dependencies, storing the result in:

```rust
pub struct DependencyGraph {
    pub manifests: Vec<Manifest>,
    pub direct_deps: Vec<Dependency>,
    pub dep_tree_depth: u32,             // Max depth of dependency tree
    pub ecosystem: Ecosystem,            // Primary detected ecosystem
}
```

This dependency graph serves two purposes: it feeds into the system prompt ("You are working in a Rust project with 47 dependencies including tokio and axum"), and it enables ecosystem-aware tool selection (e.g., preferring `cargo test` over generic test commands).

### 4.1.2 Codebase Indexing

The indexing system builds three complementary representations of the codebase, updated incrementally via `notify` file watchers ^5^:

**Tree-sitter AST Index.** Every source file is parsed into its concrete syntax tree using the appropriate tree-sitter grammar. The indexer extracts symbols (functions, structs, classes, methods, traits, interfaces, enums, constants) and their relationships (inheritance, imports, calls). The tree-sitter query API enables pattern-based extraction without full compilation:

```rust
// Extract all function definitions across languages
let query = r#"
    (function_item name: (identifier) @func.name) ; Rust
    (function_declaration name: (identifier) @func.name) ; JS/TS
    (function_definition name: (identifier) @func.name) ; Python
"#;
```

The AST index is stored in SQLite via `sqlx` ^4^with the schema:

```sql
CREATE TABLE symbols (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    language TEXT NOT NULL,
    symbol_type TEXT NOT NULL,     -- 'function', 'struct', 'class', 'trait', etc.
    name TEXT NOT NULL,
    signature TEXT,                -- Full signature for display
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    parent_id INTEGER REFERENCES symbols(id), -- For nested scopes
    UNIQUE(file_path, name, symbol_type, start_line)
);

CREATE INDEX idx_symbols_type ON symbols(symbol_type);
CREATE INDEX idx_symbols_file ON symbols(file_path);
CREATE INDEX idx_symbols_name ON symbols(name);
```

**Symbol Graph.** Built from the AST index, the symbol graph tracks cross-reference relationships — which function calls which other function, which struct is referenced by which file. This graph enables the "repo-map" feature: when the user asks a question about a specific function, the system can include not just that function's definition, but all functions that call it and all functions it calls, ranked by relevance. The graph uses petgraph's `DiGraph` ^3^with symbol IDs as node indices and weighted edges representing reference counts.

**Full-Text Search Index.** The `tantivy` crate ^6^provides a Lucene-inspired search engine with sub-10ms query latency. Each source file becomes a tantivy document with three fields: `path` (stored, string), `content` (indexed, tokenized text), and `language` (stored, string). This enables fuzzy code search across the entire codebase — a capability that tree-sitter queries alone cannot provide, since they require knowing the symbol name in advance.

The three indices are maintained incrementally. On file change (detected via `notify`), the system: (1) re-parses the changed file with tree-sitter, (2) updates affected rows in the SQLite symbol table, (3) updates the cross-reference graph, and (4) re-indexes the file content in tantivy. The total incremental update latency for a single-file change is under 200ms on a mid-range laptop.

### 4.1.3 Project Instructions

The system implements a hierarchical instruction file system inspired by Claude Code's CLAUDE.md pattern ^7^but adapted for DeepSeek Code's workflow. Four levels of instructions are discovered and merged at runtime, with higher-precedence files overriding lower-precedence ones:

| Level | Path | Scope | Committed to Git |
|-------|------|-------|-----------------|
| User global | `~/.config/deepseek-code/instructions.md` | All projects | No |
| Project | `./.deepseek-code/instructions.md` | This repository | Yes |
| Local | `./.deepseek-code/instructions.local.md` | This repo, this machine | No (git-ignored) |
| Directory | `./.deepseek-code/<subdir>/instructions.md` | Specific subdirectory | Yes |

Discovery traverses upward from the current working directory, collecting all applicable instruction files. The merge is prefix-order: user-global first, then project-level, then local overrides, then directory-specific. Each file uses YAML frontmatter for metadata:

```markdown
---
priority: 100                    # Higher wins on conflict
scope: "src/**/*.rs"            # File glob this applies to
---

# Project Conventions

Use anyhow::Result<T> for error handling, never Box<dyn Error>.
Prefer tokio::sync::mpsc over async-channel.
All public functions must have rustdoc comments.
```

Instructions are injected into the context window as user-level messages (probabilistic compliance, not system-prompt deterministic), following the pattern established by Claude Code's architecture research ^8^. This placement means the model treats them as strong suggestions rather than hard rules, which produces more natural responses while still guiding behavior.

## 4.2 Chat + Coding Loop (Category B)

The coding loop is the core interaction model: the user describes a task in natural language, the agent plans, reads files, proposes edits, applies them, runs tests, and reports results. This section specifies the loop mechanics, not the TUI presentation (which is covered in Chapter 6).

### 4.2.1 Natural Language Task Processing

When a user submits a request, the system executes a plan-generate-act pipeline modeled on the ReAct pattern ^9^but adapted for code editing:

1. **Task Classification.** The incoming request is classified into one of: `read_only` (question about code, no changes), `single_file_edit` (modify one file), `multi_file_refactor` (coordinated changes across files), `exploratory` (investigate and report), or `test_fix` (test is failing, fix it). Classification uses a lightweight heuristic (keyword matching + file reference counting) rather than a model call, to minimize latency for simple queries.

2. **Context Assembly.** For the classified task, the system assembles relevant context: file summaries from the repo-map for referenced files, the full content of files mentioned in the request, recent git diff (if any), and relevant symbol definitions. The total context is token-counted and, if it exceeds the budget (default: 60% of context window), trimmed using the symbol-graph ranking to keep the most relevant symbols.

3. **Plan Generation (for complex tasks).** Multi-file changes trigger a planning phase using DeepSeek V4 Pro. The plan specifies which files to modify, what changes to make in each, and what tests to run. The plan is presented to the user for approval before execution — this is a safety-critical step that prevents the model from making unwanted changes.

4. **Execution.** The plan is translated into a sequence of tool calls (read, edit, write, run_command, test). Each tool call passes through the permission system (Section 4.4) before execution.

### 4.2.2 Diff Review and Application

The agent uses SEARCH/REPLACE blocks as the primary edit primitive, based on converging industry evidence that this format achieves the best balance of LLM-friendliness and application reliability ^10^. A SEARCH/REPLACE block specifies the exact text to find and the exact text to replace it with:

```
<<<<<<< SEARCH
    pub fn old_function(x: i32) -> i32 {
        x * 2
    }
=======
    pub fn new_function(x: i32) -> i32 {
        x * 2 + 1
    }
>>>>>>> REPLACE
```

The patch engine implements 4-tier matching: exact match (character-for-character), whitespace-insensitive match (ignoring leading/trailing whitespace differences), indentation-preserving match (normalizing indentation levels), and fuzzy match (using similar's text diff algorithms ^20^). This fallback chain means the patch succeeds even when the model produces slightly inaccurate whitespace or when the file has changed subtly since the model read it.

Each proposed edit flows through a review pipeline:

1. **Validation.** The SEARCH text must match at least one location in the file. If ambiguous (multiple matches), the system rejects the patch and asks the model to include more context lines.
2. **Preview.** The diff is displayed in the TUI diff panel using `similar`'s unified diff output, with syntax highlighting via `syntect`.
3. **User Approval.** Depending on the permission mode (Section 4.4.1), the user may be asked to approve each edit, each file's batch of edits, or all edits in a session.
4. **Application.** Approved edits are written atomically: the file is read, patches are applied in order, and the result is written to a temporary path before being moved into place.
5. **Post-apply Verification.** After application, the system runs: (a) the file's formatter (rustfmt, prettier, black, etc.) if one is configured, (b) the project's linter, and (c) relevant tests. Failures trigger an auto-fix loop where the error output is sent back to the model with a request to fix the issue.

### 4.2.3 Error Handling and Auto-Fix Loop

The retry loop is the recovery mechanism when edits produce compilation errors, test failures, or lint violations. The loop state machine has three phases:

```
Edit Applied -> Format Check -> Lint Check -> Test Run -> Success
      ^              |              |             |
      |           Fail          Fail          Fail
      +--------------+--------------+-------------+
                     |
            Send error to model
            (max 3 retry attempts)
```

Each retry attempt includes the original edit context plus the error output. After 3 failed retries, the system stops and presents the error to the user for manual resolution. This cap prevents infinite retry loops that burn API tokens without progress.

## 4.3 TUI UX (Category C)

The TUI is the user's primary interface to the agent. Its design principle is "IDE-like information density with terminal simplicity" — multiple panels showing different aspects of the agent's operation, all navigable by keyboard, with mouse support as a secondary convenience.

### 4.3.1 Layout

The main screen uses a constraint-based layout system via `ratatui` ^21^. The layout is responsive: on screens wider than 120 columns, it shows five panels; on narrower screens, panels collapse or become tab-switchable.

```
+------------------------------------------------------------------+
| [Model: V4 Pro]  [Tokens: 12.4K/128K]  [Mode: Ask]  [main]  [?]  |  <- Status Bar
+----------+-----------------------------------------+-------------+
|          |                                         |             |
|  File    |           Chat / Response Panel         |   Plan /    |
|  Tree    |                                         |   Context   |
|          |  User: Add error handling to the        |             |
|  ▼ src   |  parse_config function                  |  1. Read    |
|    main  |                                         |     src/... |
|    lib   |  Assistant: I'll add error handling     |  2. Modify  |
|    mod   |  to parse_config. Let me start by      |     parse_  |
|  ▼ tests |  reading the current function.          |     config  |
|    test_ |                                         |  3. Add     |
|    integ |  [Read] src/lib.rs:47-62               |     tests   |
|          |                                         |             |
+----------+-----------------------------------------+-------------+
|  Diff / Code Preview Panel                                       |
|  --- src/lib.rs                                                  |
|  +++ src/lib.rs                                                  |
|  @@ -47,6 +47,9 @@                                               |
|    pub fn parse_config(path: &str) -> Result<Config> {           |
|  +    if !std::path::Path::new(path).exists() {                  |
|  +        return Err(anyhow::anyhow!("Config file not found"));  |
|  +    }                                                           |
|       let content = std::fs::read_to_string(path)?;              |
+------------------------------------------------------------------+
| [deepseek-code]  Press ? for help    3 pending edits   cost: $0.02 |  <- Bottom Bar
+------------------------------------------------------------------+
```

The five panels are: **File Tree** (left sidebar, navigable directory tree), **Chat Panel** (center, conversation history with streaming responses), **Plan Panel** (right, structured task plan with checkable steps), **Diff Panel** (bottom, code diffs and file previews), and **Status Bar** (top, model, tokens, mode, git branch). Users switch between panel focus with `Tab` / `Shift-Tab` or direct keybindings (`Ctrl+F` file tree, `Ctrl+C` chat, `Ctrl+D` diff, `Ctrl+P` plan).

### 4.3.2 Navigation

All navigation is keyboard-first. The binding system uses three tiers:

**Global shortcuts** work from any panel: `Ctrl+Q` quit, `Ctrl+N` new session, `Ctrl+S` settings, `Ctrl+L` clear chat, `Ctrl+Z` undo last edit, `Ctrl+Y` redo, `Ctrl+O` open file, `/` command palette, `?` help overlay.

**Panel-specific shortcuts** activate only when that panel has focus. In the chat panel: `↑/↓` scroll history, `Enter` send message, `Ctrl+Enter` new line, `Ctrl+R` regenerate response, `Ctrl+Shift+C` copy response. In the diff panel: `a` approve edit, `r` reject edit, `n` next diff, `p` previous diff, `Enter` apply approved.

**Vim-like bindings** (optional, enabled in config) provide hjkl navigation, `gg`/`G` for top/bottom, `Ctrl+U`/`Ctrl+D` for half-page scroll, and `/` for in-panel search. These are implemented as a separate keymap layer that can be toggled at runtime.

Mouse support (via crossterm) enables clicking to focus panels, scrolling with the wheel, and clicking buttons in the diff review interface. It is a convenience layer — every mouse action has a keyboard equivalent.

### 4.3.3 Status Bar

The status bar is information-dense and always visible. It displays:

- **Model selector** (`[V4 Pro ↓]`): Current model with dropdown to switch between V4 Pro, V4 Flash, and custom endpoints. Shows a colored indicator (green = connected, yellow = rate-limited, red = error).
- **Token meter** (`12.4K/128K`): Current conversation tokens used vs. context window size, with a visual bar that shifts from green (<50%) to yellow (<80%) to red (>90%).
- **Permission mode** (`[Ask]`): Current safety mode — one of ReadOnly, Ask, AcceptEdits, YOLO. Color-coded: blue for ReadOnly, green for Ask, yellow for AcceptEdits, red for YOLO.
- **Git info** (`main +2 ~1 -0`): Current branch with modification counts (staged, unstaged, untracked).
- **Cost estimate** (`$0.02`): Running cost of the current session, calculated from input/output token counts and the current model's pricing.

## 4.4 Safety, Memory, Tools, and Subagents (Categories D–G)

### 4.4.1 Safety and Permission System

The permission system is the most critical safety component. Analysis of Claude Code's source code reveals that users approve approximately 93% of permission prompts, making interactive confirmation behaviorally unreliable as a sole safety mechanism ^22^. The system must maintain safety independently of human vigilance.

Five permission modes are implemented, ordered from most restrictive to least:

| Mode | Description | Auto-approves | Still asks |
|------|-------------|--------------|------------|
| `read_only` | Agent can only read files and run safe commands | Nothing | All write operations |
| `ask` | Standard interactive mode (default) | Read, Grep, Glob | Edit, Write, Bash |
| `accept_edits` | File edits auto-approved; destructive commands still ask | Read, Edit, Write | Bash(rm\*), Bash(git push\*), Bash(sudo\*) |
| `auto` | Heuristic-based: command risk scorer decides | Low-risk operations | Medium/high-risk operations |
| `yolo` | Minimal prompting; deny rules still enforced | Most operations matching allowlist | Anything matching denylist |

The rule evaluation engine uses deny-first matching: deny rules are checked first, then ask rules, then allow rules, then the mode default. A deny rule always takes precedence over an allow rule, even when the allow is more specific ^23^. Rules use tool-pattern syntax:

```
Bash(rm -rf *)        # Deny recursive deletes
Bash(sudo *)           # Deny sudo commands
Read(**/.env*)         # Deny reading env files
Read(**/*.pem)         # Deny reading key files
Edit(/src/**)          # Allow edits in src directory
Bash(npm test)         # Allow running tests
Bash(cargo *)          # Allow all cargo commands
```

The command risk scorer (used in `auto` mode) assigns a 0–100 risk score based on: command name (rm=90, git=20, cargo=10), argument patterns (`-rf` adds +50, `--force` adds +40), target path sensitivity (home directory = +30, system paths = +80), and historical approval rate for similar commands in this project. Scores below 30 auto-approve, 30–70 trigger a prompt, above 70 auto-deny.

### 4.4.2 Memory and Persistence

Memory is organized into four tiers, each serving a distinct purpose and stored in a different mechanism:

**Session Memory** (conversation transcript). Stored as append-only JSONL in SQLite. Each entry is a message: user message, assistant message (text + tool_use blocks), tool_result, or system event (permission decision, compaction boundary). The append-only design makes sessions resumable by design — the full transcript can be replayed from any point ^24^.

**Project Memory** (cross-session knowledge). Stored in `.deepseek-code/memory/` as markdown files. Three sub-types: `conventions.md` (coding standards for this project), `learnings.md` (decisions and their outcomes), and `relationships.md` (files that commonly change together). Project memory is human-editable and committed to git, making it inspectable and version-controlled.

**User Preferences** (persistent settings). Stored in `~/.config/deepseek-code/config.toml` using the `toml` crate ^25^. Includes default model, permission mode, keybindings, theme, API key paths, and custom endpoints. Preferences are loaded at startup and can be overridden per-session via CLI flags or the TUI settings panel.

**Decision/Error Logs** (operational telemetry). Stored in SQLite with the schema:

```sql
CREATE TABLE decisions (
    id INTEGER PRIMARY KEY,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    session_id TEXT NOT NULL,
    task TEXT NOT NULL,              -- What the user asked
    decision TEXT NOT NULL,          -- What the agent decided
    outcome TEXT,                    -- Success, failure, partial
    error_message TEXT,              -- If outcome was failure
    tokens_used INTEGER,
    cost REAL
);
```

These logs enable the auto-memory feature: at session end, the system summarizes the session's decisions and appends relevant learnings to `conventions.md` or `learnings.md`. The summarization is performed by DeepSeek V4 Flash, keeping the cost negligible ($0.14/M input tokens) ^26^.

### 4.4.3 Tool System

The tool system implements 15+ built-in tools, each exposed to the model via OpenAI-compatible function calling schema. DeepSeek V4 supports up to 128 functions per call ^13^, leaving ample headroom for custom tools and MCP extensions.

| Tool | Category | Description | Risk Level |
|------|----------|-------------|------------|
| `Read` | File | Read file contents, optionally with offset/limit | Low |
| `Write` | File | Create or overwrite a file | High |
| `Edit` | File | SEARCH/REPLACE block edit | High |
| `Grep` | Search | Regex search across codebase | Low |
| `Glob` | Search | File pattern matching | Low |
| `FindSymbol` | Search | Find symbol definition by name | Low |
| `Bash` | Execute | Run shell command | Variable |
| `RunTest` | Execute | Run project test suite | Medium |
| `GitStatus` | Git | Check repository status | Low |
| `GitDiff` | Git | Show working tree changes | Low |
| `GitCommit` | Git | Commit changes | Medium |
| `GitLog` | Git | View commit history | Low |
| `GitUndo` | Git | Revert last commit/edit | Medium |
| `WebFetch` | External | Fetch URL content | Medium |
| `McpTool` | Extension | Call an MCP server tool | Variable |

Tool execution follows a concurrency model derived from Claude Code's `partitionToolCalls()` ^11^: read-only tools (Read, Grep, Glob, FindSymbol) execute in parallel, while state-mutating tools (Write, Edit, Bash, GitCommit) execute serially in declaration order. This parallelization reduces latency for the common pattern of "read three files, then edit one."

Streaming tool calls are implemented via `reqwest-eventsource` ^27^: as the model streams its response, tool calls are parsed incrementally. When a tool_use block is fully parsed (name and arguments complete), execution begins immediately, even if the response is still streaming. This can reduce end-to-end latency by 30–50% for multi-tool responses.

MCP (Model Context Protocol) integration enables extending the tool set without modifying core code. The system reads `~/.config/deepseek-code/mcp.json` to discover MCP servers, registers their tools as `McpTool` invocations with the appropriate server routing, and handles authentication per the MCP specification.

### 4.4.4 Subagent Orchestration

Subagents provide context isolation: instead of polluting the main agent's context window with the full transcript of a subtask, a subagent works independently and returns only a condensed summary (typically 1,000–2,000 tokens instead of 10,000+). This yields 80–90% context savings per delegation ^28^.

Four subagent roles are defined:

| Role | Model | Purpose | Tools Available |
|------|-------|---------|----------------|
| `planner` | V4 Pro | Analyze task and produce detailed implementation plan | Read, Grep, Glob, FindSymbol |
| `implementer` | V4 Flash | Execute a specific plan step (read, edit, write) | Read, Write, Edit, Bash |
| `reviewer` | V4 Flash | Review code changes for correctness and style | Read, Grep, GitDiff |
| `test_runner` | V4 Flash | Run tests, analyze failures, propose fixes | Read, Bash, RunTest |

The subagent spawn mechanism creates a fresh context window with: an isolated system prompt defining the role, a restricted tool pool (no recursive subagent spawning), the parent-provided task description, and optionally a subset of project memory. The subagent runs with its own turn limit (default: 20) and returns a structured result:

```rust
pub struct SubagentResult {
    pub summary: String,             // 500-2000 token summary for parent
    pub files_modified: Vec<PathBuf>,
    pub success: bool,
    pub error: Option<String>,
    pub tokens_used: u32,
}
```

Subagents are created via a lightweight YAML frontmatter in instruction files, following the pattern established by Claude Code's `Task` tool ^29^:

```yaml
---
name: code-reviewer
description: Review code changes for correctness
tools: [Read, Grep, GitDiff]
model: deepseek-v4-flash
max_turns: 15
---

You are a code reviewer. Focus on:
1. Correctness of logic
2. Error handling completeness
3. Test coverage of changes
4. Adherence to project conventions
```

The model selection strategy uses V4 Flash for subagents ($0.14/M input, high speed) and V4 Pro ($1.74/M input, maximum reasoning) for the main agent. This 12.4x cost differential means a typical session with 10 subagent delegations costs approximately the same as extending the main agent's context by 80,000 tokens — but with better isolation and cleaner context ^30^.

---

# 5. Architecture and Tech Stack

This section specifies the complete software architecture: a 21-layer stack organized as a Rust workspace with 12+ crates, the technology choices that underpin each layer, and the async event-driven runtime that ties them together.

## 5.1 Architecture Overview

### 5.1.1 21-Layer Architecture

The system is organized into 21 functional layers, each with a single responsibility and well-defined interfaces to adjacent layers. The layering is logical, not physical — a single crate may implement multiple layers, and a single layer may have internal sublayers. The numbering indicates data flow order, not strict dependency hierarchy.

| Layer | Name | Responsibility | Primary Crate(s) |
|-------|------|----------------|-----------------|
| L1 | CLI | Argument parsing, subcommand routing, non-interactive mode | `deepseek-code-cli` |
| L2 | TUI | Terminal rendering, widget composition, screen management | `deepseek-code-tui` |
| L3 | Router | Request routing: CLI command → handler dispatch | `deepseek-code-core` |
| L4 | Session | Session lifecycle, persistence, resume, transcript management | `deepseek-code-session` |
| L5 | Agent | ReAct loop, plan generation, task classification | `deepseek-code-agent` |
| L6 | Model | Model client abstraction, provider routing, token counting | `deepseek-code-model` |
| L7 | DeepSeek Client | HTTP client, SSE streaming, retry logic, rate limiting | `deepseek-code-api` |
| L8 | Tools | Tool registry, execution, streaming dispatch, MCP integration | `deepseek-code-tools` |
| L9 | FS | File operations, path resolution, atomic writes | `deepseek-code-fs` |
| L10 | Patch | SEARCH/REPLACE engine, 4-tier matching, diff generation | `deepseek-code-patch` |
| L11 | Git | Repository operations, auto-commit, worktree management | `deepseek-code-git` |
| L12 | Sandbox | Command sandboxing, allowlist enforcement, resource limits | `deepseek-code-sandbox` |
| L13 | Memory | Session/project/user memory tiers, SQLite persistence | `deepseek-code-memory` |
| L14 | Indexer | Tree-sitter parsing, symbol extraction, search indexing | `deepseek-code-indexer` |
| L15 | Prompt | System prompt assembly, instruction file merging, template expansion | `deepseek-code-prompt` |
| L16 | Context | Token budgeting, context window management, 5-layer compaction | `deepseek-code-context` |
| L17 | Permissions | Rule evaluation, risk scoring, mode enforcement | `deepseek-code-permissions` |
| L18 | Config | Configuration hierarchy, TOML parsing, environment variable merge | `deepseek-code-config` |
| L19 | Plugin | Plugin loading, WASM runtime (future), theme system | `deepseek-code-plugin` |
| L20 | Telemetry | Anonymous usage metrics, error reporting (opt-in) | `deepseek-code-telemetry` |
| L21 | Updater | Self-update mechanism, release checking, binary replacement | `deepseek-code-updater` |

### 5.1.2 ASCII Architecture Diagram

The following diagram shows the primary data flows and layer adjacencies:

```
                              +------------------+
                              |     Terminal     |
                              |  (User Input)    |
                              +--------+---------+
                                       |
                    +------------------v-------------------+
                    |  L2: TUI (ratatui + crossterm)      |
                    |  - Widget tree, layout engine        |
                    |  - Event loop, keybinding system     |
                    +------------------+-------------------+
                                       |
                    +------------------v-------------------+
                    |  L1: CLI (clap derive)                |
                    |  - Subcommands: tui, ask, index, etc. |
                    +------------------+-------------------+
                                       |
     +---------------------------------+----------------------------------+
     |                    L3: Router (tokio::sync::mpsc)                 |
     |  Route: CLI commands → handlers, TUI events → agent loop         |
     +----+----------+------------+-----------+----------+-------------+
          |          |            |           |          |
+---------v---+ +----v-------+ +--v--------+ +v---------++ +----------v--+
| L4: Session | | L18: Config | | L20:      | | L21:     | | L17:       |
|             | |             | | Telemetry | | Updater  | | Permissions|
| - SQLite    | | - TOML      | | - Opt-in  | | - Check  | | - Rules    |
| - JSONL     | | - Env vars  | | - Metrics | | - Update | | - Scoring  |
+----+--------+ +-------------+ +-----------+ +----------+ +----+-------+
     |                                                  ^          |
     |         +----------------------------------------+          |
     |         |                                                 |
     +-------->+         L5: Agent (ReAct Loop)                   |
               |         +----------------------------------+     |
               |         |  1. Receive user message          |     |
               |         |  2. Classify task                |     |
               |         |  3. Assemble context (L16)       |     |
               |         |  4. Call model (L6 → L7)         |     |
               |         |  5. Parse tool calls (L8)        |     |
               |         |  6. Check permissions (L17)      |     |
               |         |  7. Execute tools (L8-L12)       |     |
               |         |  8. Stream results to TUI (L2)   |     |
               |         |  9. Loop or stop                 |     |
               |         +----------------------------------+     |
               |                        |                        |
               |    +-------------------+-------------------+    |
               |    |                   |                   |    |
               | +--v---------+  +------v------+  +--------v-+  |
               | | L16:       |  | L6: Model   |  | L8:      |  |
               | | Context    |  |             |  | Tools    |  |
               | |            |  | - Provider  |  |          |  |
               | | - Token    |  |   routing   |  | - 15+    |  |
               | |   budget   |  | - Cost      |  |   built- |  |
               | | - 5-layer  |  |   tracking  |  |   in     |  |
               | |   compact  |  | - Token     |  | - MCP    |  |
               | |            |  |   counting  |  |          |  |
               | +------------+  +------+------+  +----+-----+  |
               |                        |              |        |
               |               +--------v------+ +-----v----+   |
               |               | L7: DeepSeek   | | L9: FS   |   |
               |               | Client         | | L10:Patch|   |
               |               |                | | L11:Git  |   |
               |               | - reqwest      | | L12:Sandbox| |
               |               | - SSE stream   | +----------+   |
               |               | - Retry logic  |                |
               |               +----------------+                |
               |                                                 |
               |  +-------------------------------------------+  |
               |  | L13: Memory  |  L14: Indexer  | L15: Prompt | |
               |  |              |                |             | |
               |  | - SQLite     | - tree-sitter  | - CLAUDE.md | |
               |  | - File-based | - tantivy      |   style     | |
               |  | - Decision   | - Symbol       | - Template  | |
               |  |   log        |   graph        |   engine    | |
               |  +-------------------------------------------+  |
               |                                                 |
               +-------------------------------------------------+
```

The central architectural pattern is the **Agent Loop (L5)** as the hub of all activity. Every user request flows into the loop; every tool call, model response, and permission check flows through it. The TUI (L2) is a consumer of loop events, not a controller — the loop runs independently and emits events that the TUI renders. This separation enables headless mode (CLI without TUI) by simply replacing L2 with a stdout logger.

### 5.1.3 Crate-Based Modular Design

The workspace is organized into 12 core crates with clear dependency boundaries:

```
deepseek-code/
├── Cargo.toml                    # Workspace manifest
├── crates/
│   ├── deepseek-code-cli/        # L1: CLI parsing, binary entry point
│   ├── deepseek-code-tui/        # L2: Terminal UI, widgets, event loop
│   ├── deepseek-code-core/       # L3: Router, shared types, error types
│   ├── deepseek-code-session/    # L4: Session management, persistence
│   ├── deepseek-code-agent/      # L5: ReAct loop, task classification
│   ├── deepseek-code-model/      # L6: Model abstraction, token counting
│   ├── deepseek-code-api/        # L7: DeepSeek API client, SSE streaming
│   ├── deepseek-code-tools/      # L8: Tool registry, MCP integration
│   ├── deepseek-code-fs/         # L9: File operations, gitignore support
│   ├── deepseek-code-patch/      # L10: SEARCH/REPLACE engine
│   ├── deepseek-code-git/        # L11: Git operations, auto-commit
│   ├── deepseek-code-sandbox/    # L12: Command sandboxing
│   ├── deepseek-code-memory/     # L13: Multi-tier memory, SQLite
│   ├── deepseek-code-indexer/    # L14: Code indexing, search
│   ├── deepseek-code-prompt/     # L15: Prompt assembly, templates
│   ├── deepseek-code-context/    # L16: Context window management
│   └── deepseek-code-permissions/# L17: Permission system, rule engine
├── crates-util/
│   ├── deepseek-code-config/     # L18: Configuration system
│   ├── deepseek-code-telemetry/  # L20: Usage telemetry
│   └── deepseek-code-updater/    # L21: Self-updater
```

The dependency graph enforces layering: `cli` depends on `core`; `tui` depends on `core`, `agent`, `session`; `agent` depends on `model`, `tools`, `context`, `permissions`, `prompt`; `tools` depends on `fs`, `patch`, `git`, `sandbox`; and so on. Cycles are forbidden — any circular dependency indicates a layering violation that must be refactored.

## 5.2 Tech Stack Decision

### 5.2.1 Why Rust

Rust is the implementation language for the entire system. The decision is based on a multi-dimensional analysis of four candidate languages across six criteria relevant to a TUI coding agent.

### 5.2.2 Language Comparison Table

| Dimension | Rust | Go | TypeScript | Python |
|-----------|------|-----|------------|--------|
| **TUI Framework Maturity** | ratatui 0.30 (19.1k stars, IDE-capable layouts, immediate-mode) ^31^| Bubble Tea (40.7k stars, Elm/MVU, simpler layouts) ^32^| Ink (35.6k stars, React-based, limited IDE layouts) ^33^| Textual (34.9k stars, async widgets, high memory) |
| **Memory Efficiency** | 30–40% less than Go, no GC pauses ^34^| Good (GC, occasional pauses) | Poor (V8 heap, 200MB+ baseline) | Poor (highest overhead) |
| **Binary Distribution** | Single static binary, <15MB stripped | Single binary, ~10MB but requires libc | Requires Node.js runtime | Requires interpreter + deps |
| **Async + Streaming** | Native async/await + tokio, type-safe SSE | Goroutines + channels, simpler but less structured | Callbacks/Promises, native EventSource | asyncio, less mature ecosystem |
| **Code Parsing Ecosystem** | tree-sitter (100+ langs) + tantivy (sub-10ms search) ^35^| tree-sitter Go bindings, no tantivy equivalent | Direct TS compiler API, good parsing | Excellent (ast, pylint, mypy) |
| **Git Integration** | git2 (mature, C binding) or gix (pure Rust, 2–10x faster) ^36^| go-git (pure Go, good) | simple-git (wrapper around CLI) | GitPython (wrapper) |
| **Developer Velocity** | Slower (borrow checker, explicit types) | Fast (simple, garbage-collected) | Fast (familiar, large ecosystem) | Fastest (dynamic, REPL) |
| **Type Safety** | Compile-time guaranteed, zero-cost | Runtime + generics (limited) | Erasable (JavaScript at runtime) | Dynamic (runtime errors) |

**The verdict:** Rust wins on the dimensions that matter most for this product — TUI capability, memory efficiency, code parsing/search, and binary distribution — while its developer velocity disadvantage is mitigated by the maturity of the crate ecosystem and the fact that coding agents are long-lived infrastructure projects where compile-time safety pays dividends. Go's Bubble Tea is appealing for simpler TUIs but lacks the layout sophistication needed for an IDE-like multi-panel interface. TypeScript's Ink is limited to React-style component trees that struggle with the irregular panel geometries of a coding IDE. Python's high memory footprint makes it unsuitable for a tool intended to run alongside an editor for hours.

The combination of ratatui (constraint-based layouts) + tokio (async streaming) + tree-sitter (code parsing) + tantivy (code search) + git2/gix (native git) exists only in the Rust ecosystem. No other language provides all five of these capabilities at production maturity ^37^.

### 5.2.3 Key Crates and Versions

The following table specifies each production dependency, its version pin, the feature flags used, and the architectural justification for the choice.

| Crate | Version | Key Features | Justification |
|-------|---------|-------------|---------------|
| `ratatui` | 0.30 | `crossterm_0_29` | Constraint-based layouts for multi-panel IDE interface; immediate-mode rendering at 60 FPS; modular workspace architecture |
| `crossterm` | 0.29 | `event-stream`, `bracketed-paste` | Cross-platform terminal control (Windows/macOS/Linux); async event stream for tokio integration; Kitty keyboard protocol support |
| `tokio` | 1.43 | `rt-multi-thread`, `sync`, `time`, `fs`, `process`, `signal`, `io-util` | Multi-threaded scheduler for CPU + IO work; mpsc/broadcast/watch channels for TUI- agent communication; async process spawning for git/test tools |
| `tokio-stream` | 0.1 | — | IntervalStream and ReceiverStream for merging multiple async event sources into the TUI event loop |
| `reqwest` | 0.12 | `json`, `stream`, `rustls-tls` | Async HTTP client with automatic connection pooling; JSON deserialization via serde; streaming response bodies for SSE |
| `reqwest-eventsource` | 0.6 | — | SSE streaming from DeepSeek API with automatic reconnection; wraps eventsource_stream with retry logic |
| `clap` | 4.5 | `derive`, `env` | Derive-based CLI parsing with doc-comment help text; subcommand enum for git-like CLI structure; shell completion generation |
| `serde` + `serde_json` | 1.0 | `derive` | Request/response serialization for LLM API; TOML config parsing; transcript storage |
| `tree-sitter` | 0.24 | — | Incremental parsing for 100+ languages; query API for symbol extraction; error recovery for incomplete code |
| `tantivy` | 0.22 | — | Lucene-inspired full-text search with BM25 scoring; sub-10ms query startup; multi-threaded indexing |
| `sqlx` | 0.8 | `sqlite`, `runtime-tokio`, `migrate` | Compile-time checked SQL queries; async SQLite with connection pooling; migration management |
| `git2` | 0.19 | — | Mature git operations (status, diff, log, commit, blame); used by cargo and GitHub CLI; C dependency acceptable for now |
| `similar` | 2.7 | — | Multiple diff algorithms (Myers, Patience, Hunt-McIlroy); line/word/character level diffing; unified diff output for TUI |
| `ignore` | 0.4 | — | Gitignore-aware directory walking from ripgrep author; parallel WalkParallel for multi-threaded traversal |
| `notify` | 8 | `tokio` | Cross-platform file watching (inotify/FSEvents/kqueue); debounced event delivery; triggers incremental re-indexing |
| `tracing` | 0.1 | — | Structured logging with spans for async-aware tracing; zero-cost when disabled; integrates with tokio-console |
| `anyhow` | 1.0 | — | Ergonomic error handling for application code; `.context()` for error chaining; automatic backtraces |
| `thiserror` | 2 | — | Typed error enums for library boundaries; derive macro for std::error::Error + Display |
| `toml` | 0.8 | — | TOML 1.0 config file parsing with full serde integration; comment preservation in round-trips |

Two crates warrant special discussion for their trade-offs:

**git2 vs. gix.** `git2` (v0.19) is the conservative choice: it binds libgit2, a mature C library, and provides complete API coverage for all git operations. The downside is the C build dependency and operations that are slower than pure Rust alternatives. `gix` (gitoxide) is the future: pure Rust, 2–10x faster for common operations, parallel by default, and already used by cargo and Helix. The current architecture abstracts git operations behind a trait (`GitBackend`), allowing a migration from `git2` to `gix` without changing any calling code. The trait interface:

```rust
#[async_trait]
pub trait GitBackend: Send + Sync {
    async fn status(&self) -> Result<GitStatus>;
    async fn diff(&self, staged: bool) -> Result<String>;
    async fn commit(&self, message: &str) -> Result<Oid>;
    async fn log(&self, n: usize) -> Result<Vec<Commit>>;
    async fn create_checkpoint(&self) -> Result<Oid>;
    async fn revert_to(&self, oid: Oid) -> Result<()>;
}
```

**sqlx vs. sled.** `sqlx` with SQLite was chosen over `sled` (embedded key-value) because the memory system requires relational queries (joining session tables with decision logs, filtering by timestamp ranges, aggregating token usage). `sled`'s key-value model would require reimplementing these queries in application code. SQLite's reliability and sqlx's compile-time query checking make it the correct choice for structured persistence, despite sled's marginally better write performance for simple key-value operations.

## 5.3 Async Architecture

### 5.3.1 Tokio Runtime Configuration

The application uses tokio's multi-threaded runtime with a custom configuration tuned for the workload pattern of a TUI agent (mostly IO-bound with periodic CPU bursts for indexing and diff computation):

```rust
#[tokio::main(
    flavor = "multi_thread",
    worker_threads = 4,
)]
async fn main() -> Result<()> {
    // Runtime initialized with 4 worker threads:
    // - 1 dedicated to TUI rendering (near-real-time)
    // - 1 dedicated to API communication (SSE streaming)
    // - 2 shared for tool execution, indexing, and general tasks
}
```

The SSE streaming from DeepSeek uses `reqwest-eventsource`, which wraps `reqwest`'s `bytes_stream()` with SSE frame parsing. The stream is consumed token-by-token, with each token immediately forwarded to the TUI via a `tokio::sync::mpsc` channel:

```rust
use reqwest_eventsource::EventSource;
use futures::stream::StreamExt;

let mut es = EventSource::new(request_builder)?;
while let Some(event) = es.next().await {
    match event {
        Ok(reqwest_eventsource::Event::Message(msg)) => {
            // Parse token from SSE data field
            let token = parse_sse_data(&msg.data)?;
            // Forward to TUI without blocking
            ui_tx.send(UiEvent::Token(token)).await?;
        }
        Ok(reqwest_eventsource::Event::Open) => {
            ui_tx.send(UiEvent::StreamConnected).await?;
        }
        Err(e) => {
            ui_tx.send(UiEvent::StreamError(e.to_string())).await?;
            es.close();
        }
    }
}
```

### 5.3.2 Event-Driven TUI Loop

The TUI uses a merged event stream pattern ^38^that combines multiple async event sources into a single `select!`-driven loop. This is the canonical architecture for tokio-based TUIs and eliminates polling overhead:

```rust
pub enum AppEvent {
    Tick,                               // 30 FPS render trigger
    Crossterm(Event),                   // Keyboard, mouse, resize
    ApiToken(String),                   // LLM token from SSE stream
    ToolResult(ToolCallId, ToolOutput), // Tool execution completed
    FileWatcher(Vec<DebouncedEvent>),   // Source file changed
    GitUpdate(GitStatus),               // Git state changed
    SubagentComplete(SubagentId, SubagentResult),
}

// Event sources merged into a single stream
let mut events = select! {
    tick = interval_stream.next() => AppEvent::Tick,
    key = crossterm_stream.next() => AppEvent::Crossterm(key?),
    token = api_rx.recv() => AppEvent::ApiToken(token.unwrap()),
    result = tool_rx.recv() => AppEvent::ToolResult(result.unwrap()),
    files = watcher_rx.recv() => AppEvent::FileWatcher(files.unwrap()),
    git = git_rx.recv() => AppEvent::GitUpdate(git.unwrap()),
    sub = subagent_rx.recv() => AppEvent::SubagentComplete(sub.unwrap()),
};

loop {
    match events.next().await {
        Some(AppEvent::Tick) => {
            terminal.draw(|frame| ui.render(frame, &mut app_state))?;
        }
        Some(AppEvent::Crossterm(Event::Key(key))) => {
            app_state.handle_key(key).await?;
        }
        Some(AppEvent::ApiToken(token)) => {
            app_state.chat_panel.append_token(&token);
        }
        Some(AppEvent::ToolResult(id, output)) => {
            app_state.handle_tool_result(id, output).await?;
        }
        // ... other event handlers
    }
}
```

The tick rate is 30 FPS (33ms interval), which is sufficient for smooth typing feedback and token streaming without excessive CPU usage. Keyboard events are processed via crossterm's `EventStream` with the `event-stream` feature, enabling async key reading without blocking the render thread. File watcher events are debounced using `notify-debouncer-mini` (300ms debounce) to avoid re-indexing on rapid save sequences.

### 5.3.3 Concurrent Operations

Three categories of concurrency are exploited to minimize perceived latency:

**Parallel tool execution.** Read-only tools (Read, Grep, Glob, FindSymbol) execute concurrently via `tokio::task::join_all()`. When the model requests three file reads and a grep, all four operations start simultaneously. State-mutating tools (Write, Edit, Bash, GitCommit) execute serially to avoid race conditions. The tool dispatcher classifies each tool call into its concurrency category before scheduling:

```rust
async fn execute_tool_batch(calls: Vec<ToolCall>) -> Vec<ToolResult> {
    let (read_only, mutating): (Vec<_>, Vec<_>) = calls
        .into_iter()
        .partition(|c| c.tool_name.is_read_only());

    // Execute read-only tools in parallel
    let ro_results = futures::future::join_all(
        read_only.into_iter().map(|c| execute_single_tool(c))
    ).await;

    // Execute mutating tools serially, in order
    let mut mut_results = Vec::new();
    for call in mutating {
        mut_results.push(execute_single_tool(call).await);
    }

    // Merge results in original order
    merge_results_in_order(ro_results, mut_results)
}
```

**Streaming tool calls.** As described in Section 4.4.3, the system begins executing tool calls as soon as their arguments are fully parsed from the SSE stream, without waiting for the complete response. This is implemented by running a streaming parser in a separate tokio task that feeds completed tool calls into the execution pipeline while the stream is still active.

**Background indexing.** The initial codebase scan and full-text index build runs in a background task spawned at startup. The TUI is usable immediately — the chat panel and file tree are available before indexing completes. Indexing progress is displayed in the status bar and plan panel. For large codebases (10,000+ files), the initial index build takes 5–15 seconds; incremental updates thereafter are sub-200ms.

**Subagent parallelism.** Background subagents run in their own tokio tasks with independent context windows. The parent agent continues its conversation while subagents work. When a subagent completes, its result is injected into the parent's event stream via the `subagent_rx` channel. This model supports up to 4 concurrent subagents (configurable), with the limit enforced to prevent API rate limit exhaustion.
## 6. TUI Design

The TUI layer (layer 2 of the 21-layer architecture) renders the entire user-facing surface via ratatui 0.30's immediate-mode framework backed by crossterm 0.29. This section defines the 12 screens, the design system governing them, and the constraint-based layout engine that drives responsive terminal rendering. Every screen follows a consistent header/body/footer zone pattern with transitions governed by a finite screen-state machine in `App.screen: Screen`.

### 6.1 Screen Specifications

Each specification includes an ASCII mockup at 80×24 cells, ratatui `Constraint` expressions, interaction states, and error display behavior.

#### 6.1.1 Welcome / Project Select

Entry screen on first launch. Surfaces recent projects from SQLite, quick actions, and configuration access.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  deepseek-code  v0.1.0                                        [q] quit      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Recent Projects                                                    │   │
│   │                                                                     │   │
│   │  ▸ ~/projects/my-api           Rust     2h ago                      │   │
│   │    ~/work/web-app              TS       1d ago                      │   │
│   │    ~/oss/contrib               Go       3d ago                      │   │
│   │                                                                     │   │
│   │    [j/k] navigate  [Enter] open  [d] remove from list               │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│   │ [n] New      │  │ [c] Clone    │  │ [o] Open     │  │ [,] Settings │   │
│   │   Project    │  │   Repo       │  │   Folder     │  │              │   │
│   └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                             │
│   Tip: Press Ctrl+P anytime for the command palette                        │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Model: deepseek-v4-pro    Cost: $0.000    Session: --                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Header `Length(1)`, center `Min(0)` (projects `Length(12)`, action buttons in `Layout::horizontal([Length(16); 4])`), footer `Length(1)`. Projects use a `List` with reversed `HighlightStyle`.

**States.** Empty: "No recent projects — press `n` to create one" dimmed. Loading: modal overlay (`Clear` + centered `Block`) showing "Scanning repository...".

**Errors.** Missing path: red banner "Path not found — removed from history," entry deleted from SQLite.

#### 6.1.2 Main Agent Screen

Primary 3-panel workspace. The body uses `Layout::horizontal([Length(20), Fill(1), Length(22)])`, producing `[file_tree, chat, sidebar]`. The right sidebar splits via `Layout::vertical([Fill(1), Length(8)])` for plan and tools. Below 70 columns the sidebar collapses to tab-overlay mode; below 48 columns the file tree collapses too.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  my-api  ~/projects/my-api  main*  deepseek-v4-pro  $0.042  12.4K tok      │
├──────────────┬──────────────────────────────────────────────┬───────────────┤
│ src/         │  User                                        │ PLAN          │
│ ├─ main.rs   │  ──────────────────────────────────────────  │ ┌───────────┐ │
│ ├─ lib.rs    │  Add rate limiting to /api/v1/users          │ │ 1. Read   │ │
│ ├─ routes/   │  endpoint.                                   │ │    auth   │ │
│ │  ├─ user.  │                                              │ │ 2. Mod... │ │
│ │  └─ admin  │  Assistant  thinking...                      │ │ 3. Add    │ │
│ │     └─ ... │  ▼                                           │ │    tests  │ │
│ ├─ models/   │  I'll add rate limiting using a token-bu...  │ │ 4. Run    │ │
│ │  └─ user.  │                                              │ └───────────┘ │
│ ├─ middleware│  ```rust                                     │               │
│ │  └─ auth.  │  use std::sync::Arc;                         │ TOOLS         │
│ ├─ Cargo.toml│  use tokio::sync::Mutex;                     │ [read] x3     │
│ └─ .gitignor │  ```                                         │ [grep] x1     │
│              │                                              │ [write] x1    │
│ [1]Chat[2]Pl │                                              │               │
├──────────────┴──────────────────────────────────────────────┴───────────────┤
│ > add rate limiting   [Enter] send  [Ctrl+P] palette  [Ctrl+D] diff        │
├─────────────────────────────────────────────────────────────────────────────┤
│  PRO  ◐ thinking...  3 tools pending  [Ctrl+C] cancel                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

**States.** Streaming: status bar shows `◐ thinking...`; tool calls populate the right sidebar live as they stream in ^8^. Code blocks render in nested `Block` widgets with tree-sitter highlighting.

**Errors.** API errors: modal with code and `[r] retry`. Timeouts: yellow status "Request timed out (30s)."

#### 6.1.3 Chat Screen

Full-screen chat via `Ctrl+T` tab 1. Expanded center panel with message history, streaming display, thinking toggle, and code blocks.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  deepseek-code  Chat                                    [Ctrl+E] export    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  You — 14:32                                                                │
│  Add rate limiting middleware with a sliding window of 100 req/60s.         │
│                                                                             │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Assistant — deepseek-v4-pro — $0.018 — 4.2K tokens                         │
│  ▼ Thinking (2.1K tokens)                                                   │
│  The user wants rate limiting. I'll use a sliding window counter...         │
│                                                                             │
│  I'll implement a sliding window rate limiter:                              │
│                                                                             │
│  ```rust  src/middleware/rate_limit.rs                                      │
│  use std::collections::VecDeque;                                            │
│  use std::sync::Arc;                                                        │
│  use tokio::sync::RwLock;                                                   │
│                                                                             │
│  pub struct SlidingWindow {                                                 │
│      window_secs: u64,                                                      │
│      max_requests: usize,                                                   │
│      requests: Arc<RwLock<VecDeque<Instant>>>,                              │
│  }                                                                          │
│  ```                                                                        │
│  [▸] Show 48 more lines                                                    │
│                                                                             │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Assistant — applying edits... ◐                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  [↑↓] history  [Ctrl+T] tabs  [Ctrl+Space] thinking toggle                │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** `Length(1)` header, `Fill(1)` messages with `Scrollbar`, `Length(1)` input, `Length(1)` footer. Messages are `List<MessageWidget>` items; code blocks are nested `Block` widgets titled with file path.

**States.** `Ctrl+Space` toggles `reasoning_content` visibility. Collapsed code blocks show 10 lines plus `[▸] Show N more`. Auto-scroll to bottom on new tokens; scroll up pauses.

**Errors.** Approaching 1M context: yellow warning "⚠ Context limit approaching — consider /compact."

#### 6.1.4 Plan Screen

Task decomposition with step status, dependency graph, and estimated tokens/cost. Default right-sidebar widget; this is the full-page expansion.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Plan: Add rate limiting                                         [q] back  │
├─────────────────────────────────────────────────────────────────────────────┤
│  Task: Add rate limiting middleware (est. $0.034, ~8.2K tokens)             │
│                                                                             │
│  ┌───┐  [✓] 1. Read existing auth middleware (src/middleware/auth.rs)      │
│  │   │       0.8K tok  $0.003                                               │
│  ▼   │  [✓] 2. Check route definitions in src/routes/mod.rs                │
│      │       0.5K tok  $0.002                                               │
│  ┌───┘  [⟳] 3. Implement SlidingWindow in src/middleware/rate_limit.rs   │
│  │          3.2K tok  $0.012  ◐ writing...                                  │
│  │   ┌──┐ [ ] 4. Add rate limiter to /api/v1/users route                   │
│  │   │      1.5K tok  $0.006  blocked by #3                                 │
│  └───┘  [ ] 5. Write unit tests                                             │
│              2.2K tok  $0.008  blocked by #3, #4                            │
│  ┌───┐  [ ] 6. Run cargo test                                               │
│  └───┘       0.4K tok  $0.002  blocked by #5                                │
│                                                                             │
│  Dependencies: 3→4→5→6 sequential chain; 1, 2 independent                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  [r] refresh  [Enter] view step details  [Ctrl+C] abort plan               │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Header `Length(1)`, task list `Fill(1)`, dependency note `Length(2)`, footer `Length(1)`. Steps render as tree-connected nodes (`│├└─`). Status: `✓` green complete, `⟳` yellow in-progress, `✗` red failed, blank dim pending.

**States.** In-progress steps refresh per-tick. Failed steps expand inline. Plan states: `generating`, `executing`, `paused` (awaiting approval), `complete`.

**Errors.** Generation failure: error card with `[r] regenerate`, `[m] manual entry`.

#### 6.1.5 Diff Review

Safety screen for code edits. Side-by-side SEARCH/REPLACE with per-hunk controls ^1^. Below 70 columns, switches to stacked vertical layout.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Diff Review — 3 changes in 2 files                          [Ctrl+D] close │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  src/middleware/rate_limit.rs                                               │
│  ┌──────────────────────────────┬──────────────────────────────────────┐   │
│  │ SEARCH                       │ REPLACE                              │   │
│  │                              │                                      │   │
│  │ use std::sync::Arc;          │ use std::sync::Arc;                  │   │
│  │                              │ use std::collections::VecDeque;      │   │
│  │ pub struct AuthMiddleware;   │ use std::time::Instant;              │   │
│  │                              │                                      │   │
│  │ impl AuthMiddleware {        │ pub struct SlidingWindow {           │   │
│  │     // ...                   │     window_secs: u64,                │   │
│  │ }                            │     max_requests: usize,             │   │
│  │                              │     requests: Arc<...>,              │   │
│  │                              │ }                                    │   │
│  └──────────────────────────────┴──────────────────────────────────────┘   │
│  [a] accept  [r] reject  [e] edit  [↑↓] next hunk     Hunk 1/3  ✓ pending │
│                                                                             │
│  src/routes/user.rs                                                         │
│  ┌──────────────────────────────┬──────────────────────────────────────┐   │
│  │ .route("/users", get(...))   │ .route("/users", get(...))           │   │
│  │                              │     .layer(rate_limit))               │   │
│  └──────────────────────────────┴──────────────────────────────────────┘   │
│  [a] accept  [r] reject  [e] edit  [↑↓] next hunk     Hunk 2/3  ✓ pending │
├─────────────────────────────────────────────────────────────────────────────┤
│  [A] accept all  [R] reject all  [Ctrl+Enter] apply accepted  [?] help     │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Header `Length(1)`, diff list `Fill(1)`, footer `Length(1)`. Each file: `Layout::horizontal([Fill(1), Fill(1)])` for SEARCH/REPLACE columns as `Paragraph` widgets with line-level styling.

**States.** Per-hunk: `pending`, `accepted` (green border), `rejected` (dimmed strikethrough), `edited` (yellow border). All decided → footer highlights `[Ctrl+Enter] apply accepted`. SEARCH mismatch: `⚠ No match` with `[f] force (fuzzy)`, `[e] manual edit`.

**Errors.** Apply failure: modal "File changed on disk — [r] reload, [d] diff against disk."

#### 6.1.6 Tool Call Timeline

Chronological tool execution log with parameters, results, and timing.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Tool Call Timeline                                         [Ctrl+L] close │
├─────────────────────────────────────────────────────────────────────────────┤
│  Time     Tool       Parameters                        Result    Dur.       │
├─────────────────────────────────────────────────────────────────────────────┤
│  14:32:01 read_file  path: src/middleware/auth.rs      142 lines  12ms      │
│  14:32:03 grep       pattern: "pub struct.*Middleware  2 matches  45ms      │
│                      path: src/middleware"                                 │
│  14:32:04 read_file  path: src/routes/user.rs          89 lines   8ms      │
│  14:32:07 write_file path: src/middleware/rate_limit.  OK        23ms      │
│                      content: [4072 bytes]                                  │
│  ▶ 14:32:08 run_command cmd: cargo check               ◐ running  1.2s     │
│                      timeout: 60s                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│  [f] filter  [Enter] expand  [e] export  [c] clear                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** `Table` widget with columns `Time(10)`, `Tool(12)`, `Parameters(Fill(1))`, `Result(10)`, `Dur(8)`. Sticky header via `Table::header` with `Modifier::REVERSED`.

**States.** Running tools: `◐` with live timer. Failed tools: expand on selection. Filter (`f`): footer becomes a text input.

**Errors.** Timeout: `✗ timeout (60.0s)` in red, `[Enter] view output`.

#### 6.1.7 Command Execution

Live terminal output from shell commands with exit code and kill control.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Command: cargo test                                         [q] close     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  $ cargo test                                                               │
│     Compiling my-api v0.1.0 (/home/dev/projects/my-api)                     │
│      Finished `test` profile [unoptimized + debuginfo] target(s) in 3.42s   │
│       Running unittests src/main.rs (target/debug/deps/my_api-3f2a)         │
│                                                                             │
│  test middleware::rate_limit::tests::test_sliding_window ... ok             │
│  test routes::user::tests::test_get_users ... ok                            │
│                                                                             │
│  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered    │
│  ⎋ Exit code: 0 (success) — 3.8s elapsed                                   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  [Ctrl+C] kill  [Ctrl+S] save output  [↑↓] scroll  [G] end                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Header `Length(1)`, output `Fill(1)` with `Scroll` offset (10,000-line scrollback), footer `Length(1)`. ANSI codes convert to ratatui `Style` via `ansi-to-tui`.

**States.** Running: `[Ctrl+C] kill` + live timer. Completed: exit code colored (0 = green, 1–127 = red, 130 = yellow SIGINT). Killed: "⎋ Killed — [r] rerun."

**Errors.** Non-zero exit: status shows code + stderr tail, `[v] view full` to expand.

#### 6.1.8 Test Results

Structured pass/fail summary with failure details, coverage, and retry.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Test Results                                                [q] close     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Summary: 12 passed  2 failed  0 skipped    Coverage: 87.3%                │
│  ═══════════════════════════════════════════════════════════════════════    │
│                                                                             │
│  ✓ middleware::rate_limit  4 passed                                          │
│  ✓ routes::user            3 passed                                          │
│  ✗ routes::admin           1 failed  1 passed                                │
│                                                                             │
│  ── routes::admin::tests::test_admin_access FAIL ────────────────────────  │
│    Assertion failed: expected status 403, got 200                           │
│    → src/routes/admin.rs:42                                                 │
│                                                                             │
│  ✗ models::user            1 failed  2 passed                                │
│                                                                             │
│  ── models::user::tests::test_validation FAIL ───────────────────────────  │
│    Assertion failed: email regex rejected valid input                       │
│    → src/models/user.rs:118                                                 │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  [r] retry failed  [a] retry all  [v] view full output  [g] goto source    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Summary bar `Length(3)`, suites list `Fill(1)`, footer `Length(1)`. Failed suites expand with error details in nested `Block`. Coverage parsed from `cargo tarpaulin` / `llvm-cov`.

**States.** Running: spinner + elapsed. Retry: delta line shows changes ("+1 fixed, -0 new").

**Errors.** Test framework launch failure: full-screen "`cargo` not in PATH. Install Rust toolchain? [y] open rustup."

#### 6.1.9 Memory / Session

Session persistence, checkpoint browsing, memory search, and restore points.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Memory & Sessions                                           [q] back      │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌───────────────────────────────────────────────┐ │
│  │ Sessions            │  │ Checkpoints — my-api (current)                │ │
│  │                     │  │                                               │ │
│  │ ▸ my-api     now    │  │  Time              Description    Tokens Cost │ │
│  │   web-app    2h ago │  │  14:35  plan: rate limit gen.   8.2K   $0.03│ │
│  │   contrib    1d ago │  │  14:38  edits applied            12.1K  $0.04│ │
│  │                     │  │  14:40  tests passing            2.4K   $0.01│ │
│  │ [n] new session     │  │                                               │ │
│  │ [d] delete          │  │  Memory Search: > rate limit                │ │
│  │                     │  │  ───────────────────────────────────────────  │ │
│  │                     │  │  • "Implemented SlidingWindow for rate lim" │ │
│  │                     │  │    Session: my-api  14:38  [Enter] view     │ │
│  │                     │  │  • "Rate limiting on /api/v1/users uses 10"│ │
│  │                     │  │    Session: web-app  3d ago  [Enter] view   │ │
│  └─────────────────────┘  └───────────────────────────────────────────────┘
├─────────────────────────────────────────────────────────────────────────────┤
│  [s] search memory  [r] restore checkpoint  [Ctrl+E] export session        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** `Layout::horizontal([Length(22), Fill(1)])` → sessions list, checkpoint/memory panel. Right panel: `Layout::vertical([Length(8), Length(6), Fill(1)])` → checkpoints `Table`, search input, results `List`.

**States.** Selected checkpoint expands to show files changed, tools used, token summary. Search (`s`): live fuzzy filter against tantivy memory index.

**Errors.** SQLite locked: "Database locked — retrying..." with automatic 500ms retry up to 5 attempts.

#### 6.1.10 Settings

Model selector, permission mode, theming, keybindings, API credentials. Two-pane: category list left, form right.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Settings                                                    [,] close     │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────────────────────────────────────────────┐ │
│  │ Model        │  │  Model Selection                                     │ │
│  │ Permissions  │  │                                                      │ │
│  │ Theme        │  │  Provider:  DeepSeek ████████████████████████████░ │ │
│  │ Keybindings  │  │                                                      │ │
│  │ API Key      │  │  Model:     ◉ Pro (deepseek-v4-pro)                  │ │
│  │ Advanced     │  │             ○ Flash (deepseek-v4-flash)              │ │
│  │              │  │             ○ Custom...                              │ │
│  │              │  │                                                      │ │
│  │              │  │  Thinking:  [x] Enabled    Depth: high ████░░░     │ │
│  │              │  │                                                      │ │
│  │              │  │  Context caching: [x] Auto (recommended)             │ │
│  │              │  │                                                      │ │
│  │              │  │  Max context: 512K tokens ████████████████░░       │ │
│  │              │  │                                                      │ │
│  └──────────────┘  └──────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│  [↑↓] navigate  [Enter] edit  [Ctrl+S] save  [Ctrl+R] reset defaults       │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** `Layout::horizontal([Length(16), Fill(1)])`. Category `List` with highlight. Form: radio `◉`/`○`, checkbox `[x]`/`[ ]`, slider `███░░` adjusted with `←/→`.

**States.** Unsaved changes: footer "● unsaved" in yellow. API key masked as `sk-...XXXX`, `[Enter]` to reveal. Invalid value: inline error.

**Errors.** Save failure: "Failed to save: permission denied" with `[o] open directory`.

#### 6.1.11 MCP / Tools

Tool ecosystem management: built-in tools, MCP servers, per-tool permissions, enable/disable toggles.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Tools & MCP Servers                                         [q] close     │
├─────────────────────────────────────────────────────────────────────────────┤
│  Built-in (16)   MCP Servers (2)   [Tab] switch tab                         │
├─────────────────────────────────────────────────────────────────────────────┤
│  Tool              Category    Permission    Status    Uses                  │
│  ─────────────────────────────────────────────────────────────────────────  │
│  read_file         fs          auto          ✓ on      142                  │
│  write_file        fs          ask           ✓ on      23                   │
│  edit_file         fs          ask           ✓ on      8                    │
│  grep              search      auto          ✓ on      67                   │
│  run_command       shell       ask           ✓ on      12                   │
│  git_commit        git         ask           ✓ on      5                    │
│  web_fetch         network     deny          ✗ off     0                    │
│  browser_navigate  browser     ask           ✓ on      0                    │
│                                                                             │
│  MCP: localhost:3000 (filesystem)      ✓ connected  6 tools exposed        │
│  MCP: localhost:8080 (github)          ✗ disconnected  [r] retry           │
├─────────────────────────────────────────────────────────────────────────────┤
│  [Enter] toggle  [p] permission cycle  [e] edit MCP  [+] add MCP            │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Tab bar `Length(1)`, tool `Table` `Fill(1)`, MCP rows appended below, footer `Length(1)`. Columns: `Tool(18)`, `Category(12)`, `Permission(12)`, `Status(8)`, `Uses(8)`.

**States.** Permission cycles `auto → ask → deny` on `p`. MCP states: `connecting` (spinner), `connected` (green), `disconnected` (red), `error` (yellow). `[e]` opens inline endpoint/transport editor.

**Errors.** MCP connection failure: "Connection refused: localhost:8080 — check server is running."

#### 6.1.12 Logs / Debug

Structured log viewer for tracing spans, error details, and export.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Logs — deepseek-code.log                                    [Ctrl+E] exp  │
├─────────────────────────────────────────────────────────────────────────────┤
│  Level    Time     Target              Span             Message             │
│  ─────────────────────────────────────────────────────────────────────────  │
│  DEBUG    14:32:01 deepseek_client::req  chat_round     POST /chat/comp    │
│  DEBUG    14:32:02 deepseek_client::sse  chat_round     stream opened      │
│  INFO     14:32:04 agent::tools          execute        read_file: src/..   │
│  DEBUG    14:32:04 agent::patch          apply_hunk     matched tier 1     │
│  WARN     14:32:05 agent::patch          apply_hunk     tier 1 failed, t.. │
│  ERROR    14:32:05 agent::tools          run_command    timeout after 60s  │
│  INFO     14:32:06 agent::session       checkpoint     saved checkpoint..  │
│  DEBUG    14:32:07 ratatui::render      render_frame   3 widgets, 0.8ms   │
│                                                                             │
│  ── ERROR detail ────────────────────────────────────────────────────────   │
│  agent::tools::run_command timeout:                                       │
│    command: "cargo test --package my-api"                                   │
│    elapsed: 60.03s                                                          │
│    span trace: chat_round > execute_tools > run_command                     │
├─────────────────────────────────────────────────────────────────────────────┤
│  [f] filter level  [t] filter target  [/] search  [G] tail  [↑↓] scroll    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layout.** Log `Table` `Fill(1)` with columns `Level(8)`, `Time(10)`, `Target(18)`, `Span(16)`, `Message(Fill(1))`. Error detail `Length(6)` at bottom when `ERROR`/`WARN` selected. Footer `Length(1)`.

**States.** Tail mode (`G`): auto-scrolls, footer `▼ tailing`. Filtered: Level cycles via `f`; target filter (`t`) opens fuzzy picker. Paused (scrolled up): `⏸ paused — [G] resume`.

**Errors.** Missing log file: "Log file not found — restart with logging enabled."

### 6.2 Design System

#### 6.2.1 Color Theme

Default theme is dark with low saturation for long sessions. Defined in a `Theme` struct resolved from `~/.config/deepseek-code/theme.toml` with built-in fallback.

```rust
pub struct Theme {
    pub bg: Color,           // terminal default
    pub fg: Color,           // #c0c5ce — soft white
    pub fg_dim: Color,       // #65737e — inactive
    pub accent: Color,       // #96b5b4 — cyan-teal highlights
    pub accent_bold: Color,  // #8fa1b3 — blue active
    pub success: Color,      // #a3be8c — muted green
    pub warning: Color,      // #ebcb8b — muted yellow
    pub error: Color,        // #bf616a — muted red
    pub border: Color,       // #4f5b66 — dark gray
    pub border_focus: Color, // #96b5b4 — focused accent
    pub thinking: Color,     // #b48ead — purple reasoning
    pub code_bg: Color,      // #2b303b — code blocks
}
```

Syntax highlighting delegates to tree-sitter query captures (`keyword`, `string`, `function`, `comment`, etc.) mapped to theme colors. For 16-color terminals, the theme collapses to nearest ANSI; truecolor is auto-detected via `crossterm::terminal::supports_truecolor`. Borders use `border`/`border_focus` by focus state. Diff SEARCH uses `error` for removed lines; REPLACE uses `success` for added lines. The status bar renders with `Modifier::REVERSED`.

#### 6.2.2 Keyboard Shortcuts

Defined in a `Keybinding` struct loaded from config. Vim-mode (`vim_mode = true`) remaps navigation: `h/j/k/l` for movement, `gg`/`G` for top/bottom, `Ctrl+U`/`Ctrl+D` for half-page scroll. When enabled, `j` shadows the welcome-screen "new project" which remaps to `n`. The command palette (`Ctrl+P`) serves as the universal escape hatch — fuzzy search over all actions, making every function discoverable.

| Category | Key | Action | Screen |
|---|---|---|---|
| Global | `Ctrl+P` | Open command palette | All |
| Global | `Ctrl+C` | Cancel / kill running command | All |
| Global | `Ctrl+Q` | Quit (confirm if pending ops) | All |
| Global | `,` | Open Settings | All |
| Nav | `Ctrl+T` | Cycle tabs (Chat/Plan/Diff/Tools) | Main |
| Nav | `Ctrl+D` | Open diff review | Main |
| Nav | `Ctrl+L` | Open tool timeline | Main |
| Nav | `Esc` | Close modal / go back | All |
| Context | `j`/`k` | Navigate lists | Welcome, Memory, Logs |
| Context | `Tab`/`Shift+Tab` | Cycle panel focus | Main, Settings |
| Context | `↑`/`↓` | Scroll message history | Chat |
| Context | `Ctrl+Space` | Toggle thinking visibility | Chat |
| Context | `a`/`r` | Accept / reject diff hunk | Diff Review |
| Context | `Enter` | Send / open item | Chat, Welcome |
| Context | `f` | Filter current list | Timeline, Logs |
| Context | `g`/`G` | Jump top / bottom | Chat, Logs |

#### 6.2.3 Responsive Layout

The layout engine adapts the 3-panel main screen to terminal dimensions via width thresholds:

| Width | File Tree | Sidebar | Behavior |
|---|---|---|---|
| ≤47 | Hidden | Hidden | All navigation via `Ctrl+P` palette |
| 48–59 | 16 cols | Hidden | Narrow tree only |
| 60–79 | 20 cols | Hidden | Full tree, no sidebar |
| ≥80 | 20 cols | 22 cols | Full 3-panel layout |

When the sidebar collapses, plan and timeline move to `Ctrl+T` tab cycling. When the tree also collapses (below 48 columns), files are accessed via palette. Minimum supported terminal: 40×12; below this, an overlay warns "Terminal too small — resize to at least 48×12." Height adaptation: header and footer are always `Length(1)`, body receives `Min(0)`. Below 8 rows, the footer collapses into the header as reversed right-aligned text.

The layout cache (default in ratatui 0.30) stores constraint calculations across frames. For static layouts like the main screen, this eliminates solver overhead; only dynamic content areas incur per-frame layout cost. Benchmarks show sub-millisecond full-frame renders at 80×24 and under 2ms at 200×60 ^5^.
## 7. Agent Loop and Prompting Protocol

The agent loop orchestrates the cycle of reasoning, tool invocation, permission validation, and error recovery that transforms a user request into verified code changes. The design synthesizes the ReAct pattern ^1^— interleaving reasoning traces with action steps — with Claude Code's `queryLoop` architecture ^2^, adapted for DeepSeek V4's dual-mode API. The result is a deterministic 14-step loop with five execution modes and a phase-specialized prompting protocol.

---

### 7.1 Agent Loop Design

#### 7.1.1 The 14-Step Loop

The loop extends Claude Code's 9-step pipeline ^2^with stages for plan validation, diff review, and test verification. Each step is either a pure transformation or a gated side effect.

| Step | Name | Purpose | Side Effects? |
|------|------|---------|---------------|
| 1 | **Task Ingestion** | Parse user input; classify intent (explain, plan, edit, debug, test) | No |
| 2 | **Model Routing** | Select V4 Flash (fast subtasks), V4 Pro (reasoning), or subagent delegation | No |
| 3 | **Context Assembly** | Gather project memory, conversation history, file contents, tool definitions | No |
| 4 | **Context Shaping** | Apply budget reduction → snip → microcompact → collapse; skip if <50K tokens | No |
| 5 | **Plan Generation** | Decompose task into ordered subtasks with file dependencies and complexity estimates | No |
| 6 | **Permission Check** | Evaluate plan against execution mode; prompt for destructive operations | Yes (user prompt) |
| 7 | **Tool Dispatch** | Generate tool calls; partition into concurrent-safe (read) and exclusive (write) sets | No |
| 8 | **Tool Execution** | Execute reads in parallel, writes serially; capture stdout, stderr, exit codes | Yes (filesystem, shell) |
| 9 | **Patch Generation** | Produce SEARCH/REPLACE blocks or `Edit` tool calls with old_string/new_string pairs | No |
| 10 | **Diff Review** | Present proposed changes; check style compliance and security patterns | No |
| 11 | **Test Execution** | Run affected tests; capture pass/fail with 8K char output truncation | Yes (test runner) |
| 12 | **Error Handling** | Classify error → retry with compacted context → escalate model → surface to user | Yes (memory) |
| 13 | **Response Formulation** | Synthesize results; cite sources; state confidence levels | No |
| 14 | **Memory Update** | Append transcript; update auto-memory; write git checkpoint | Yes (disk, git) |

A **turn** consists of steps 3–14; a task may span multiple turns. The loop terminates on one of five stop conditions ^2^: no `tool_use` blocks in the response (natural finish), max turn count exceeded (default 30), context overflow even after full compaction, explicit user abort, or hook intervention blocking execution.

#### 7.1.2 Sequence Diagram

```
User          Orchestrator        Model (DeepSeek)    Tool Runtime    Permission Gate   Git
 |                  |                    |                  |                |         |
 |--query---------->|                    |                  |                |         |
 |                  |--Step 1: classify->|                  |                |         |
 |                  |--Step 2: route---->|                  |                |         |
 |                  |--Step 3: context-->|                  |                |         |
 |                  |--Step 4: shape ctx-|                  |                |         |
 |                  |--Step 5: generate->|                  |                |         |
 |                  |     plan (thinking)|                  |                |         |
 |                  |<-----plan---------|                  |                |         |
 |                  |--Step 6: permission check------------>|                |         |
 |                  |<-----approval required (destructive)  |                |         |
 |<--show plan------|                    |                  |                |         |
 |--approve-------->|                    |                  |                |         |
 |                  |--Step 7: dispatch tool calls---------->|                |         |
 |                  |--Step 8: execute->|----Read file---->|                |         |
 |                  |                    |<---file content--|                |         |
 |                  |                    |----Grep symbol-->|                |         |
 |                  |                    |<---matches-------|                |         |
 |                  |--Step 9: propose-->|                  |                |         |
 |                  |     patch blocks   |                  |                |         |
 |                  |<-----patches------|                  |                |         |
 |                  |--Step 10: diff review (if applicable) |                |         |
 |<--show diff------|                    |                  |                |         |
 |--confirm-------->|                    |                  |                |         |
 |                  |--Step 11: run tests------------------>|--test cmd ---->|         |
 |                  |                    |                  |<--test results-|         |
 |                  |--Step 12: error?-->|                  |                |         |
 |                  |  Y: retry ladder   |                  |                |         |
 |                  |  N: continue       |                  |                |         |
 |                  |--Step 13: respond  |                  |                |         |
 |<--result---------|                    |                  |                |         |
 |                  |--Step 14: update memory/git----------->|                |--commit->
 |                  |                    |                  |                |<--ok----|
```

Two tool execution paths exist. The `StreamingToolExecutor` begins executing read operations as soon as their arguments parse from the streaming response. A fallback `runTools()` path classifies calls via `partitionToolCalls()` into concurrent-safe (Read, Grep, Glob) and exclusive (Write, Edit, Bash) sets ^2^. Read tools run in parallel; write tools run serially to prevent race conditions.

#### 7.1.3 Core Loop Pseudocode

The loop is an async generator yielding stream events, letting the TUI consume events incrementally.

```rust
async fn agent_loop(params: QueryParams) -> impl Stream<Item = StreamEvent> {
    try_stream! {
        let mut state = RuntimeState::new(params.max_turns);
        let mut recovery = RecoveryLadder::new();

        'turn_loop: for turn in 0..params.max_turns {
            // Steps 3-4: Context assembly and shaping
            let mut messages = context_gather(&state, &params).await?;
            messages = context_shape(messages, &params.model).await?;

            // Step 5: Plan generation (thinking mode for V4 Pro)
            yield StreamEvent::PlanStart;
            let plan = generate_plan(&messages, &params).await?;
            yield StreamEvent::PlanComplete(plan.summary());

            // Step 6: Permission gate
            match check_permission(&plan, &params.mode).await? {
                Permission::Deny => {
                    yield StreamEvent::Denied("plan rejected by permission rule");
                    break 'turn_loop;
                }
                Permission::Ask => {
                    yield StreamEvent::AwaitingApproval(plan.clone());
                    // Block until TUI signals user response
                }
                Permission::Allow => {}
            }

            // Steps 7-8: Model call with error recovery ladder
            yield StreamEvent::ModelCallStart(turn);
            let response = match call_model(&messages, &params, &state.tool_budget).await {
                Ok(r) => r,
                Err(ModelError::ContextOverflow) => {
                    if !recovery.has_attempted_reactive_compact {
                        recovery.has_attempted_reactive_compact = true;
                        messages = reactive_compact(messages).await?;
                        continue 'turn_loop;
                    }
                    if !recovery.has_attempted_full_compact {
                        recovery.has_attempted_full_compact = true;
                        messages = full_compaction(messages, &state).await?;
                        continue 'turn_loop;
                    }
                    yield StreamEvent::Fatal("context exhausted after compaction");
                    break 'turn_loop;
                }
                Err(ModelError::MaxOutputTokens) => {
                    if recovery.output_token_escalation < 3 {
                        recovery.output_token_escalation += 1;
                        params.model.max_output_tokens *= 2; // 8K -> 16K -> 32K -> 64K
                        continue 'turn_loop;
                    }
                    yield StreamEvent::Fatal("max output exceeded after escalation");
                    break 'turn_loop;
                }
                Err(e) => {
                    yield StreamEvent::Error(format!("model error: {}", e));
                    break 'turn_loop;
                }
            };

            // Stop condition: no tool calls
            let tool_calls = parse_tool_use_blocks(&response);
            if tool_calls.is_empty() {
                yield StreamEvent::Result(response.text);
                break 'turn_loop;
            }

            // Step 8: Execute tools
            let partitioned = partition_tool_calls(&tool_calls);
            for batch in partitioned.concurrent_batches() {
                for result in execute_concurrent(batch).await {
                    yield StreamEvent::ToolResult(result.id, result.output);
                    state.messages.push(ToolResultMsg::from(result));
                }
            }
            for exclusive in partitioned.serial_queue() {
                if !check_tool_permission(&exclusive, &params.mode).await? {
                    yield StreamEvent::ToolDenied(exclusive.name);
                    continue;
                }
                let result = execute_exclusive(exclusive).await;
                yield StreamEvent::ToolResult(result.id, result.output);
                state.messages.push(ToolResultMsg::from(result));
            }

            // Step 11: Test execution (if patches applied)
            if state.has_patches_this_turn() {
                match run_affected_tests(&state).await {
                    TestResult::Pass => yield StreamEvent::TestsPassed,
                    TestResult::Fail(output) => {
                        yield StreamEvent::TestsFailed(output.truncate(8192));
                        state.messages.push(UserMsg::new(
                            format!("Tests failed:\n{}", output)
                        ));
                    }
                }
            }

            if state.turn_count >= params.max_turns || state.should_abort {
                yield StreamEvent::Stop(state.stop_reason());
                break 'turn_loop;
            }
            state.turn_count += 1;
        }

        // Step 14: Memory update
        state.persist_transcript().await?;
        state.update_auto_memory().await?;
        git_commit_checkpoint(&state).await?;
        yield StreamEvent::SessionSaved;
    }
}
```

The recovery ladder uses named continue points rather than returns, making every transition independently testable ^2^. Guards prevent infinite loops: one-shot compaction flags, hard caps of 3 recovery attempts per trigger type, and circuit breakers on repeated failures. Stop hooks never run on error responses, preventing "error → hook blocks → retry → error" spirals ^2^.

The `tool_use` / `tool_result` invariant is enforced structurally: every `tool_use` must have a paired `tool_result` before the next API call. On abort during streaming, the executor drains remaining requests by emitting synthetic `tool_result` blocks; the API rejects assistant messages with unmatched `tool_use` blocks ^2^.

---

### 7.2 Execution Modes

The execution mode determines the autonomy boundary between agent and user. Claude Code implements seven modes ^2^; this design adapts five for common workflows. The mode is set at session start and overridable per-command.

**No-edit mode (7.2.1)** makes the agent read-only. Write tools (Edit, Write, Bash with side effects) are filtered from the tool schema before each model call. The model may describe changes it would make, but the permission gate intercepts write calls with `Permission::Deny`. This mode serves code review, architecture explanation, and onboarding.

**Plan-only mode (7.2.2)** generates a detailed execution plan at step 5 and pauses before any tool invocation. The plan includes files to read, files to modify, changes per file (described, not implemented), test commands, and estimated token cost. The user must approve before the agent proceeds. Appropriate for multi-file refactoring (5+ files) and architecture decisions where global coherence matters ^6^.

**Auto-approve mode (7.2.3)** is the recommended default. Non-destructive operations (Read, Grep, Glob, test runs) execute without confirmation; destructive operations (Edit, Write, risky Bash) trigger permission prompts. The permission pipeline evaluates rules in strict deny → ask → allow order ^2^. Deny rules always take precedence, even when allow rules are more specific.

Risk classification uses pattern matching for MVP, graduating to an ML classifier in v1. Anthropic's analysis found users approve approximately 93% of permission prompts, making interactive confirmation behaviorally unreliable as the sole safety mechanism ^2^. Automatic risk classification with tiered escalation compensates for this auto-approval tendency.

**YOLO mode (7.2.4)** minimizes confirmations to deny-rule matches only. All other operations execute automatically. Restricted to trusted environments — isolated worktrees, personal projects with git backups, or CI with read-only production access. A persistent warning banner displays in the TUI; downgrading to a safer mode is supported mid-session via `/mode`.

**Dry-run mode (7.2.5)** simulates all actions. Read operations execute normally (idempotent); write operations produce diff previews without filesystem changes. Bash commands undergo syntax validation and display with predicted working directory but do not run. Tests execute against a temporary working tree copy.

| Mode | Read Tools | Write Tools | Bash | Tests | Confirmation |
|------|-----------|-------------|------|-------|-------------|
| No-Edit | Allowed | Blocked | Read-only only | Allowed | Never |
| Plan-Only | Allowed (after approval) | Allowed (after approval) | Allowed (after approval) | Allowed (after approval) | Once per plan |
| Auto-Approve | Auto | Ask (destructive) | Ask (risky) | Auto | Selective |
| YOLO | Auto | Auto | Auto | Auto | Deny-rules only |
| Dry-Run | Normal | Simulated (diff only) | Syntax-check only | On temp copy | Never |

---

### 7.3 Prompting Protocol

Five specialized system prompts are tuned to specific workflow phases. They compose dynamically at runtime: base identity + phase-specific instructions + DeepSeek configuration + filtered tool definitions ^5^. All prompts use the OpenAI message format; DeepSeek V4 is fully API-compatible ^5^. The system prompt is passed as the `system` parameter, not a user message, to maximize compliance probability.

#### 7.3.1 Main Coding Agent Prompt

```
You are DeepSeek Code TUI, an expert coding assistant operating in a terminal
interface. You help users write, read, debug, and refactor code.

CORE RULES:
1. You have access to file system tools (Read, Grep, Glob, Edit, Write, Bash).
   Use them rather than guessing file contents.
2. Always read a file before editing it. Never assume you know its content.
3. Prefer SEARCH/REPLACE edit blocks for changes. Each block must match exactly
   one location in the target file. Include 2-3 lines of surrounding context
   in the SEARCH section to disambiguate.
4. When uncertain, say "I'm not certain" rather than hallucinating.
5. Cite your sources: [from file:path.ts], [from training data], or [inference].

SAFETY BOUNDARIES:
- Do NOT read files matching **/.env*, **/secrets/**, **/*.pem, or ~/.ssh/**
  unless explicitly requested by the user.
- Do NOT execute commands that delete files or modify git history without
  explicit user confirmation.
- Do NOT make network requests except through the WebFetch tool.
- Before destructive operations, warn the user and ask for confirmation.

OUTPUT FORMAT:
- Use markdown code blocks with language tags for all code.
- For file edits, show the SEARCH/REPLACE block, then confirm the file changed.
- When tests fail, show the relevant failure lines, not the full output.
- End responses with a brief summary of what was done.

THINKING MODE:
- Enable thinking mode for: planning multi-file changes, debugging complex
  issues, architectural decisions.
- Disable thinking mode for: single-file edits, summaries, status reports.
```

#### 7.3.2 Planner Prompt

```
You are the Planning Agent. Decompose the coding task into an ordered
execution plan. Do NOT write code yet — only plan.

For each subtask, provide:
1. ID (T1, T2, ...)
2. Description (one sentence)
3. Files to read (paths)
4. Files to modify or create (paths)
5. Dependencies (subtasks that must complete first)
6. Complexity: Low (<50 lines), Medium (50-200 lines), High (>200 lines)
7. Risk flag: Safe (additive), Caution (modifies existing logic), Risky
   (changes public APIs, database schemas, authentication)

DEPENDENCY ANALYSIS:
- Read all affected files before proposing changes.
- Identify cross-file references (imports, shared types, test dependencies).
- Order subtasks so foundational changes (types, interfaces) precede consumers.
- Flag circular dependencies and suggest breaking strategies.

If the task is trivial (single file, <20 lines), state that and proceed
directly to implementation.
```

#### 7.3.3 Implementer Prompt

```
You are the Implementation Agent. Write code changes based on an approved
plan or direct instruction.

CODE GENERATION RULES:
1. Match existing style: indentation, naming, imports, error handling.
2. Use SEARCH/REPLACE blocks for all edits. Each block changes one contiguous
   region; use multiple blocks for non-contiguous changes.
3. Keep search strings unique — they must match exactly one location.
4. Include 2-3 lines of context in the SEARCH section.
5. Do NOT change unrelated code. Note issues separately.

STYLE ADHERENCE:
- Check rustfmt.toml, .eslintrc, CONTRIBUTING.md for project conventions.
- Follow the project's error handling pattern (Result/Option, try/catch, etc.).
- Use existing abstractions rather than introducing new ones unless justified.

TEST REQUIREMENTS:
- Every bug fix includes a test that fails before and passes after.
- Every new feature has unit tests for the happy path and one error case.
- Run existing tests after changes. Fix failures or explain obsolete tests.
- Prefer table-driven tests for multiple similar cases.

Before finishing, verify: search string matches file content exactly;
replacement compiles (imports, types, syntax); all references updated.
```

#### 7.3.4 Reviewer Prompt

```
You are the Code Review Agent. Review proposed changes for correctness,
security, performance, and style compliance.

REVIEW CHECKLIST:
1. Correctness: Does the change match the plan? Check logic, edge cases,
   error handling, type safety.
2. Bugs: Off-by-one errors, null dereferences, race conditions, resource
   leaks, unhandled error paths.
3. Security: Injection vulnerabilities (SQL, command, XSS), hardcoded
   secrets, unsafe deserialization, missing authentication. Flag new
   dependencies with known CVEs.
4. Style: Compliance with project conventions. Flag inconsistent naming,
   formatting, or missing documentation on public APIs.
5. Performance: O(n^2) where O(n log n) is achievable, unnecessary
   allocations, N+1 query patterns.
6. Tests: New tests present? Edge cases covered? Existing tests still valid?

For each issue: Severity (Critical/Warning/Suggestion), file and line range,
description (one sentence), recommended fix (if applicable).

End with a verdict: APPROVE, APPROVE_WITH_NOTES, or REQUEST_CHANGES.
If APPROVE, state what you checked.
```

#### 7.3.5 DeepSeek-Specific Prompt

```
You are operating on DeepSeek V4. Follow these mode guidelines:

THINKING MODE (enable for: planning, debugging, architecture):
- Set thinking: true in the API call.
- Reasoning streams in reasoning_content, separate from the main response.
- Use for: multi-step reasoning, comparing alternatives, tracing complex
  bugs, designing abstractions.
- Do NOT repeat reasoning in the main response — summarize conclusions only.

NON-THINKING MODE (enable for: summaries, single edits, tool calls):
- Set thinking: false (default).
- Respond directly without reasoning preamble.
- Use for: status reports, simple edits, file reads, test summaries.

MODE SWITCHING:
- The orchestrator sets the mode per turn. Do not switch yourself.
- If deeper reasoning is needed mid-task, note it; the orchestrator may
  re-call with thinking enabled.

STREAMING:
- Your response streams token-by-token. Begin tool calls as soon as you
  know which tool to use — do not wait for the full response.
- The TUI displays text in real-time. Keep prose concise; put detailed
  reasoning in thinking_content.

TOKEN EFFICIENCY:
- V4 Flash handles subtasks: prioritize brevity, omit unnecessary explanation.
- V4 Pro handles planning: thoroughness over brevity, explore edge cases.
```

The thinking/non-thinking split is a critical optimization. DeepSeek V4's `reasoning_content` field lets the TUI display a "thinking..." indicator with the reasoning stream, then replace it with the final answer ^5^. This is more responsive than waiting for the complete reasoning chain. The orchestrator tracks which agent type requires which mode and sets the API parameter per turn.

At runtime, the five prompts compose into a single system message: base identity and safety rules from the main prompt, task guidance from the phase-specific prompt (planner, implementer, or reviewer), model configuration from the DeepSeek prompt, and relevant tool definitions filtered semantically to the current phase ^6^. This composition keeps individual prompts focused while ensuring no critical safety rule is omitted.
## 8. Memory, Safety, and Patch Engine

This chapter addresses three infrastructure layers that determine whether the agent operates reliably: the memory and context system that feeds the model, the safety layer that gates tool execution, and the patch engine that transforms LLM output into verified file changes. Each layer uses concrete data structures, classification rules, and recovery mechanisms. The chapter assumes familiarity with the agent loop described in Chapter 7 and builds on its ReAct-based execution model.

### 8.1 Memory and Context System

#### 8.1.1 Tiered Memory Architecture

Relying solely on the LLM context window for memory leads to context pollution and degraded performance over long sessions. ^2^The system implements a three-tier memory model:

**Session memory** stores the conversation transcript: user messages, assistant reasoning traces, tool calls, and tool results. This tier is ephemeral, cleared when the session ends, and checkpointed to SQLite every 5 turns for crash recovery.

**Project memory** persists across sessions and stores file summaries, architectural decisions, error patterns, and command history. It is scoped to the project root directory (identified by the containing git repository). Structured data lives in SQLite; long-form notes (decision logs, convention files) are stored as Markdown in `.deepseek/memory/` to remain human-editable and version-controllable. ^1^**User memory** stores global preferences: permission mode defaults, model selection, custom rules, and API key references (not the keys themselves). This tier is stored in `~/.deepseek/preferences.toml` and loaded at startup.

Only the session tier changes during a conversation. Project and user tiers are read-only during a turn and updated asynchronously at session end. This separation prevents preference drift within a session and ensures that project knowledge accumulates across sessions without polluting the active context window.

#### 8.1.2 SQLite Schema

The SQLite database (`~/.deepseek/state.db`) contains 12 tables organized into four functional groups. The schema optimizes for append-only writes on the hot path with batch updates for analytics.

```
+------------------+       +------------------+       +------------------+
|    sessions      |       |    messages      |       |  checkpoints     |
|------------------|       |------------------|       |------------------|
| id (PK)          |<------| session_id (FK)  |       | id (PK)          |
| project_path     |       | id (PK)          |       | session_id (FK)  |
| started_at       |       | role             |       | git_ref          |
| ended_at         |       | content          |       | created_at       |
| model_used       |       | tool_calls(JSON) |       | description      |
| turn_count       |       | tool_results(JSON|      +------------------+
| status           |       | timestamp        |
+------------------+       | token_count      |       +------------------+
         |                 +------------------+       |   preferences    |
         |                         |                   |------------------|
         v                         v                   | id (PK)          |
+------------------+       +------------------+       | key              |
|  file_summaries  |       |    decisions     |       | value            |
|------------------|       |------------------|       | scope            |
| id (PK)          |       | id (PK)          |       | project_path(FK) |
| session_id (FK)  |       | session_id (FK)  |       | updated_at       |
| file_path        |       | topic            |       +------------------+
| summary_text     |       | decision         |
| line_count       |       | rationale        |       +------------------+
| hash             |       +------------------+       |  memory_index    |
+------------------+                                   |------------------|
                                                       | id (PK)          |
+------------------+       +------------------+       | project_path(FK) |
|     errors       |       |     patches      |       | category         |
|------------------|       |------------------|       | content          |
| id (PK)          |       | id (PK)          |       | relevance_score  |
| session_id (FK)  |       | session_id (FK)  |       | created_at       |
| tool_name        |       | file_path        |       +------------------+
| error_message    |       | search_block     |
| recovery_action  |       | replace_block    |       +------------------+
+------------------+       | match_tier       |       |   usage_stats    |
                           +------------------+      |------------------|
                                                      | id (PK)          |
+------------------+                                  | session_id (FK)  |
|   tool_results   |                                  | model            |
|------------------|                                  | input_tokens     |
| id (PK)          |                                  | output_tokens    |
| session_id (FK)  |                                  | latency_ms       |
| tool_call_id     |                                  | cost_usd         |
| tool_name        |                                  | timestamp        |
| result_text      |                                  +------------------+
| is_error         |
| truncated        |
| timestamp        |
+------------------+
```

Key design decisions: `messages` stores `tool_calls` and `tool_results` as JSONB columns to preserve nested structure without a normalized sub-table that would complicate the append-only write path. `file_summaries` stores SHA-256 hashes to detect external modifications without re-reading files. `memory_index` stores structured observations (file relationships, error patterns, conventions) inserted at session end and retrieved via BM25 text search at the start of subsequent sessions. No vector embeddings are used at this layer; structured retrieval achieves ~170K tokens/year versus ~19.5M for full context replay, justifying the added complexity. ^5^#### 8.1.3 Context Assembly: Nine Ordered Sources

For every model call, the assembler builds messages from nine sources in strict priority order:

1. **System prompt** — base behavior rules and available tool schemas
2. **Project instructions** — `.deepseek.md` in the project root storing team conventions ^1^3. **Auto-memory block** — observations from prior sessions (file relationships, error patterns, decisions)
4. **File contents** — files explicitly added to the conversation
5. **Search results** — ripgrep or tree-sitter index lookups
6. **Git status** — branch, changed files, recent commits
7. **Conversation history** — compacted via the graduated pipeline if necessary
8. **Dynamic tool schemas** — filtered to relevant tools when semantic filtering is enabled
9. **User message** — the current query

Sources 1-3 form the stable prefix and benefit from prompt caching; sources 4-8 vary per turn. This split aligns with DeepSeek's automatic context caching, which reduces repeated-prefix costs by 10x on cache hits. ^4^When the assembled context exceeds the effective context window (`context_window - output_reserve - safety_buffer`), a graduated compaction pipeline applies in order of increasing destructiveness: per-tool-result budget capping (8K characters per result), history snipping (dropping tool results older than 20 turns), cache-aware micro-compaction (preserving cache boundaries), context collapse projection (read-time virtual view over history), and finally full auto-compaction (model-generated summary as last resort). ^3^### 8.2 Tool Execution Safety

#### 8.2.1 Command Risk Classification

Every shell command is classified into one of three risk levels before execution. Classification uses a two-stage pipeline: pattern matching against a compiled trie (approximately 200 patterns), followed by heuristic scoring for unmatched commands.

| Category | Risk Level | Pattern Examples |
|----------|-----------|------------------|
| Read-only inspection | Safe | `ls`, `cat`, `head`, `grep`, `find`, `git status`, `git log --oneline`, `rg` |
| Build and test | Safe | `npm test`, `cargo test`, `pytest`, `make`, `cargo build`, `go test` |
| Git read | Safe | `git branch`, `git show`, `git stash list`, `git remote -v` |
| Package management | Sensitive | `npm install`, `cargo add`, `pip install`, `go get` |
| Git write | Sensitive | `git commit`, `git checkout -b`, `git merge`, `git rebase` |
| Destructive file ops | Destructive | `rm -rf *`, `dd if=* of=*`, `mkfs.*`, `format`, `shred` |
| Destructive DB ops | Destructive | `DROP TABLE`, `DROP DATABASE`, `TRUNCATE TABLE` |
| Privilege escalation | Destructive | `sudo *`, `chmod 777 *`, `chown root:*` |
| Force git ops | Destructive | `git push --force*`, `git reset --hard*`, `git clean -fd` |
| Credential access | Sensitive | `cat ~/.ssh/*`, `cat ~/.aws/*`, `cat .env`, `printenv` with tokens |

Pattern matching evaluates rules in order of specificity (longest match wins). Each pattern carries a risk level and confidence score. The ruleset loads at startup from `~/.deepseek/rules/safety.toml` with project overrides from `.deepseek/safety.toml`.

Commands matching no pattern enter heuristic scoring: +3 for destructive keywords (`rm`, `drop`, `delete`, `truncate`, `format`, `dd`), +2 for write flags (`-f`, `--force`, `-y`, `--yes`), +2 for privilege escalation (`sudo`, `doas`), +1 for globs (`*`, `?`), and -1 for safe flags (`--dry-run`, `-n`, `--list`). Score 0 or below maps to safe, 1-2 to sensitive, 3+ to destructive. ^6^#### 8.2.2 Permission Rules

Rules follow deny-first evaluation: deny rules checked first, then ask, then allow. The first match wins. A deny rule always takes precedence over an allow rule, even when more specific. A broad `Bash(rm -rf *)` deny cannot be overridden by a narrow `Bash(rm -rf /tmp/build)` allow. ^7^Default policy: safe commands are allowed, sensitive commands prompt for confirmation, destructive commands are denied by default with per-case override allowed. This default-deny stance is essential because approximately 93% of permission prompts are approved by users in production, making interactive confirmation behaviorally unreliable as the sole safety mechanism. ^8^Rules are scoped at three levels (highest to lowest precedence): managed policies in `/etc/deepseek/safety.toml`, user policies in `~/.deepseek/safety.toml`, and project policies in `.deepseek/safety.toml` (committed to git). Array settings like `permissions.allow` merge across scopes. ^7^```toml
# .deepseek/safety.toml — example project-level configuration
[permissions]
deny = [
  "Bash(rm -rf /)", "Bash(rm -rf ~)", "Bash(dd if=* of=*)",
  "Bash(sudo *)", "Read(**/.env*)", "Read(**/*.pem)",
]
ask = [
  "Bash(rm -rf *)", "Bash(git push --force*)",
  "Bash(git reset --hard*)", "Bash(chmod 777 *)",
]
allow = [
  "Bash(npm *)", "Bash(cargo *)", "Bash(git status)",
  "Bash(git log *)", "Bash(rg *)", "Read(*)", "Edit(*)",
]
```

#### 8.2.3 Sandbox Constraints

Every tool execution runs within a sandbox with four constraints: timeout, output truncation, secrets redaction, and working directory restriction.

Default timeout is 30 seconds for shell commands and 60 seconds for build/test commands. Commands can request extended timeout via `timeout_ms`, up to a hard maximum of 300 seconds. Exceeding the timeout triggers `SIGTERM`, then `SIGKILL` after 5 seconds.

Output is truncated to 10,000 lines or 1 MB, whichever is reached first, with a marker `[... output truncated: N lines hidden]` appended. ^3^Secrets redaction scans all output through a DFA matcher: AWS access key IDs (`AKIA[20 chars]`), GitHub tokens (`ghp_[36 chars]`), generic API keys (`[key|token|secret|password]=[alphanumeric]{16,}`), and private key headers. Matches are replaced with `[REDACTED:<type>]`.

Working directory restriction constrains file operations to the project root and subdirectories. Path traversal attempts (e.g., `Read(/etc/passwd)`, `Write(../../outside)`) are rejected before execution. Symlinks are resolved before validation.

### 8.3 Patch Engine

#### 8.3.1 SEARCH/REPLACE Primary Format

The patch engine uses SEARCH/REPLACE blocks with 4-tier matching. Content-addressed editing (search strings) outperforms position-addressed editing (line numbers): minimal unified diff achieves ~14% pass@1 accuracy on LLM edit benchmarks, while content-aware formats like BlockDiff reach ~56% and SEARCH/REPLACE achieves ~70-80% on evolved codebases. ^9^```
path/to/file.rs
<<<<<<< SEARCH
    let mut config = Config::load("settings.toml");
    config.port = 8080;
=======
    let mut config = Config::load("settings.toml");
    config.port = env::var("PORT").unwrap_or(8080);
>>>>>>> REPLACE
```

**Tier 1 — Exact match.** Literal byte-for-byte comparison. Succeeds for ~60-70% of edits. ^10^**Tier 2 — Whitespace-insensitive match.** Leading and trailing whitespace normalized per line. Adds ~10-15% cumulative coverage. ^10^**Tier 3 — Indentation-preserving match.** Relative indentation preserved but absolute level allowed to shift. Handles mixed indentation or code copied from different nesting levels. Adds ~5-10% cumulative coverage. ^20^**Tier 4 — Fuzzy match.** Levenshtein similarity between SEARCH block and candidate regions, searching outward from estimated anchor locations. Accepted if similarity exceeds 0.75 and the match is unique (second-best must score at least 0.15 lower). ^21^If all tiers fail, the engine returns a `SearchNotFound` error with the file path, SEARCH block, similarity scores of best and second-best candidates, and a suggested correction. For files under ~400 lines with extensive edits (>50% of file), the engine falls back to full-file rewrite. ^22^#### 8.3.2 Edit Pipeline

Every file change passes through a nine-stage pipeline:

```
[Read] -> [Generate] -> [Validate] -> [Preview] -> [Approve] -> [Apply] -> [Format] -> [Test] -> [Commit]
   ^          |            |            |           |          |          |          |          |
   |          |            |            |           |          |          |          |          |
   +----------+------------+------------+-----------+----------+----------+----------+----------+
   (on failure after Apply: git reset --hard to checkpoint)
```

**Read.** File content and SHA-256 hash are read and cached.

**Generate.** LLM produces SEARCH/REPLACE blocks.

**Validate.** Parser checks well-formedness; matcher attempts tiers 1-4 against cached content. Any failure rejects the entire batch.

**Preview.** Unified diff presented for review.

**Approve.** User confirmation in interactive mode; auto-accept for trusted file patterns in headless mode.

**Apply.** Edits written to disk after re-checking the file hash against cache. Hash mismatch aborts with `FileChangedError`. ^23^**Format.** Project formatter runs once per file, deferred until all edits in a multi-file transaction are applied to avoid line-number invalidation. ^24^**Test.** Relevant tests execute (convention-based: `src/foo.rs` maps to `tests/test_foo.rs`). Failures return error output to the LLM; pipeline loops to Generate.

**Commit.** On success, a git commit is created. On failure after Apply, `git reset --hard` to the pre-edit checkpoint. ^25^#### 8.3.3 Core Data Structures

```rust
// Complete patch proposal with one or more file changes
struct Patch {
    patch_id: Uuid,
    session_id: Uuid,
    created_at: DateTime<Utc>,
    files: Vec<FileChange>,
    status: PatchStatus,
}

enum PatchStatus {
    Draft, Validated, Approved, Applied, Formatted, Tested, Committed,
    Failed(String), RolledBack,
}

// Single file modification
struct FileChange {
    file_path: PathBuf,
    hunks: Vec<Hunk>,
    original_hash: String,          // SHA-256 at read time
    original_content: String,
    modified_content: Option<String>,
}

// One SEARCH/REPLACE block
struct Hunk {
    search: String,
    replace: String,
    match_tier: Option<MatchTier>,
    similarity_score: Option<f64>,
}

enum MatchTier {
    Exact, WhitespaceInsensitive, IndentPreserving, Fuzzy(f64),
}

// Tool call parsed from model response
struct ToolCall {
    call_id: String,
    name: String,
    arguments: serde_json::Value,
    status: ToolCallStatus,
}

enum ToolCallStatus {
    Pending, AwaitingApproval, Executing,
    Completed(ToolResult), Denied(String), Failed(String),
}

// Permission request for sensitive/destructive operations
struct ApprovalRequest {
    request_id: Uuid,
    tool_call: ToolCall,
    risk_level: RiskLevel,
    rule_matched: Option<String>,
    prompt_text: String,
    timeout_seconds: u64,
    resolution: Option<ApprovalResolution>,
}

enum RiskLevel { Safe, Sensitive, Destructive }

enum ApprovalResolution {
    Approved, Denied(String), ApprovedOnce, ApprovedAlways(String),
}

// Tool execution result
struct ExecutionResult {
    call_id: String,
    stdout: String,
    stderr: String,
    exit_code: i32,
    truncated: bool,
    duration_ms: u64,
    secrets_redacted: Vec<String>,
}
```

#### 8.3.4 Multi-File Transactions

Multi-file changes (e.g., renaming a function and updating call sites) are atomic: all edits validated before any applied; if any edit fails after partial application, all changes roll back.

The protocol: (1) create a git checkpoint by committing the current working tree; (2) validate all SEARCH/REPLACE blocks across all files (tiers 1-4); abort if any block fails; (3) apply edits bottom-up by line number within each file to minimize line-shift effects; (4) run the code formatter once per modified file after all edits are applied; (5) execute tests; (6) on pass, create a final commit; on fail, `git reset --hard` to checkpoint and return errors to the LLM. ^25^Bottom-up ordering is critical for multi-hunk edits. If two hunks target lines 50 and 150, applying line 150 first ensures that insertions or deletions around line 50 do not shift the second hunk's location. This eliminates the "line drift" problem that causes sequential edit strategies to fail ~15-20% of the time. ^26^Git serves as the transaction log. The checkpoint commit is the atomic restore point. This trades fine-grained partial rollback for simplicity: `git reset --hard` restores edited files and any side effects from build processes or auto-generated files. ^25^Checkpoints apply to every edit, creating linear history traversable via `/undo` (executes `git revert` on the most recent agent commit). Checkpoints are garbage-collected after 7 days or when exceeding 50 per session, with older checkpoints squashed into an archive commit to prevent repository bloat.
## 9. Subagents, Config, and Distribution

### 9.1 Subagent System

The primary motivation for subagent delegation is context isolation, not parallelism. When a parent agent delegates a focused subtask—"review all test coverage for the auth module"—the child's full conversation history (which may exceed 10,000 tokens) stays contained. Only a 1,000–2,000 token summary returns to the parent, yielding an 80–90% context savings per delegation ^3^. Claude Code's subagent pattern implements this through fresh context windows, isolated tool pools, and optional git worktree isolation; this architecture is the baseline for DeepSeek Code TUI's design ^2^.

#### 9.1.1 MVP Subagents

The MVP ships four subagent roles that cover the core code-modification workflow. Each role maps to a specific system prompt and a restricted toolset, ensuring that the model stays within its lane.

| Role | Purpose | Model | Toolset | Max Turns |
|------|---------|-------|---------|-----------|
| `Lead` | Main orchestrator; handles user-facing reasoning and coordinates delegation | V4 Pro | All tools + `delegate` | 50 |
| `Planner` | Task decomposition; outputs file-level change plans without executing | V4 Flash | `Read`, `Grep`, `Glob` | 15 |
| `Implementer` | Executes SEARCH/REPLACE blocks against specified files | V4 Flash | `Read`, `Edit`, `Write`, `Bash(git)` | 30 |
| `Reviewer` | Code review of proposed or committed changes | V4 Flash | `Read`, `Grep`, `Glob`, `Bash(git diff)` | 20 |

The `Lead` agent runs on V4 Pro because it performs complex multi-step reasoning, resolves ambiguities in user intent, and decides when to delegate. All other MVP agents run on V4 Flash ($0.14/M input tokens vs. Pro at $1.74/M) ^2^, keeping delegation costs low even at high volume. The `Planner` has no write tools—its job is to read the codebase, understand constraints, and emit a structured plan that the `Implementer` executes. This separation prevents the common failure mode where a planning model prematurely edits files before the plan is complete.

The delegation flow follows a strict pipeline:

```
User Request
    |
    v
Lead Agent (reasoning: handle directly or delegate?)
    |
    +---> Direct (single-file fix, quick read) --> Execute in main loop
    |
    +---> Delegate (multi-file, well-scoped) --> Spawn Planner
                                              |
                                              v
                                        Planner (reads codebase)
                                              |
                                              v
                                        Returns plan to Lead
                                              |
                                              v
                                        Lead approves / revises
                                              |
                                              v
                                        Spawn Implementer
                                              |
                                              v
                                        Returns result summary
                                              |
                                              v
                                        Spawn Reviewer (optional)
                                              |
                                              v
                                        Lead presents to user
```

Subagents cannot spawn nested subagents. This flat hierarchy prevents exponential context fragmentation and keeps the delegation graph inspectable. Each subagent invocation is a fresh instance with opt-in memory scope via the `memory` field in its definition ^2^.

#### 9.1.2 v1 Additional Roles

Post-MVP expansion adds five specialized roles, each addressing a specific failure mode observed in production coding agents:

| Role | Purpose | Model | Addresses |
|------|---------|-------|-----------|
| `TestRunner` | Executes test suites, parses output, diagnoses failures | V4 Flash | "Tests pass on my machine" gaps between implementer and CI |
| `SecurityReviewer` | Scans changes for secrets, injection risks, permission leaks | V4 Pro | Credential leaks in generated code; CWE pattern matching |
| `DocsWriter` | Updates README, API docs, and inline comments to match changes | V4 Flash | Documentation drift after refactoring |
| `GitAgent` | Handles branching, committing, rebasing, and merge conflict resolution | V4 Flash | Keeps main agent focused on code, not git mechanics |
| `DependencyAgent` | Manages package installation, version bumps, lockfile updates | V4 Flash | Prevents "it works but I forgot to add the dependency" |

The `SecurityReviewer` runs on V4 Pro rather than Flash because security analysis requires deeper reasoning about control flow and taint propagation. All other v1 agents use Flash for cost efficiency.

#### 9.1.3 Subagent Design Principles

Every subagent definition follows a consistent template derived from Claude Code's YAML frontmatter pattern ^2^:

```toml
[subagent.reviewer]
name = "code-reviewer"
description = "Code review specialist. Invoke for thorough review of any changes."
model = "deepseek-v4-flash"
max_turns = 20
tools = ["Read", "Grep", "Glob", "Bash"]
disallowed_tools = ["Write", "Edit"]
permission_mode = "read_only"
background = false
isolation = "none"
```

The isolation field has three levels. `none` means the subagent reads and writes in the main working tree—appropriate for the `Reviewer` and `Planner`, which only read. `worktree` spawns a git worktree on a temporary branch, giving the subagent its own checkout; this is the default for `Implementer` and `DocsWriter` when modifying multiple files. `full` adds a container sandbox on top of worktree isolation, reserved for the `SecurityReviewer` when executing untrusted code.

Context isolation enforces prompt self-containment: the child sees nothing from the parent's conversation. Everything the subagent needs must be in its task description. This is intentional—it forces the `Lead` agent to articulate precise, self-contained task specifications rather than relying on shared conversational context that would leak and pollute.

Result aggregation strips the full conversation history. The subagent returns only a summary block:

```
Subagent: Implementer
Task: Add input validation to auth.ts, login.ts, register.ts
Status: completed
Files modified: 3
Files created: 0
Test status: passed (12/12)
Notes: Used zod for validation schema; existing tests updated to cover new checks.
```

The parent appends this summary (typically 500–1,500 tokens) in place of the 10,000+ token full transcript, achieving the documented 80–90% savings ^3^.

### 9.2 Configuration Files

Configuration uses a two-layer hierarchy: global settings in `~/.config/deepseek-code/config.toml` apply across all projects, while project-specific settings in `.deepseek-code/project.toml` override them for the current repository. This mirrors Claude Code's 4-level permission hierarchy (managed → user → project → local) ^2^but collapses the two system-level scopes into one for simplicity in the MVP.

#### 9.2.1 Global Config

The global config stores credentials, defaults, and user preferences. It lives at `~/.config/deepseek-code/config.toml` on Unix and `%APPDATA%\deepseek-code\config.toml` on Windows.

```toml
# ~/.config/deepseek-code/config.toml
# Global configuration for DeepSeek Code TUI

[api]
# DeepSeek API key. Leave empty to be prompted on first run.
api_key = ""
# Override the base URL for self-hosted or proxy deployments
base_url = "https://api.deepseek.com"
# Default model for the Lead agent
model = "deepseek-v4-pro"
# Request timeout in seconds
timeout = 120

[models]
# Model used for subagent delegation (all non-Lead roles)
subagent_model = "deepseek-v4-flash"
# Maximum output tokens per response
max_output_tokens = 8192
# Enable thinking mode for complex reasoning tasks
thinking_mode = true

[ui]
# Theme: "dark", "light", "system"
theme = "dark"
# Show streaming tool calls in real-time
stream_tools = true
# Editor for inline file editing (must be CLI-compatible)
editor = "nvim"
# Pager for long outputs
pager = "less"

[permissions]
# Permission mode: "interactive", "accept_edits", "auto", "dont_ask"
# interactive: prompt for every risky operation
# accept_edits: auto-approve file edits, ask for shell commands
# auto: use rule-based classifier (deny > ask > allow)
# dont_ask: apply deny rules only, no prompts (for CI)
mode = "interactive"
# Deny rules evaluated first; most specific match wins
deny = [
    "Bash(rm -rf *)",
    "Bash(git push --force *)",
    "Bash(git reset --hard *)",
    "Bash(sudo *)",
    "Bash(curl * | sh)",
    "Read(**/.env*)",
    "Read(**/*.pem)",
    "Read(**/*.key)",
    "Read(~/.ssh/**)",
]
# Auto-allow these patterns without prompting
allow = [
    "Read(*)",
    "Grep(*)",
    "Glob(*)",
    "Bash(git status)",
    "Bash(git log *)",
    "Bash(npm test)",
    "Bash(cargo test)",
]
# Always ask for these patterns (middle priority)
ask = [
    "Bash(git push *)",
    "Bash(git checkout *)",
    "Bash(npm install *)",
    "Bash(cargo build)",
]
# Enable deny-first rule evaluation: deny rules always override allows
deny_first = true

[memory]
# Session persistence: "full", "compact", "none"
session_persistence = "compact"
# Maximum number of turns before triggering compaction
compaction_turns = 20
# Enable auto-memory (agent writes observations to project memory)
auto_memory = true
# Path to SQLite memory database
memory_db = "~/.config/deepseek-code/memory.db"

[telemetry]
# Opt-in anonymous usage statistics
eabled = false
# Crash reporting endpoint (empty = disabled)
crash_report_url = ""
```

The `deny_first` flag implements the deny-first rule evaluation that Claude Code's permission system uses, where a broad deny ("deny all `rm -rf`") cannot be overridden by a narrow allow ^2^. This prevents the most common permission escalation bug: an overly permissive allow rule accidentally permitting a dangerous operation.

#### 9.2.2 Project Config

The project config lives at `.deepseek-code/project.toml` in the repository root. It is committed to git so teams share settings, but should not contain API keys (those stay in the global config).

```toml
# .deepseek-code/project.toml
# Project-specific configuration for DeepSeek Code TUI

[project]
# Human-readable project name
name = "my-api-service"
# Primary language for context-aware prompting
language = "typescript"
# Framework hints for better code generation
framework = "express"
# Test command used by the TestRunner subagent
test_command = "npm test"
# Lint command used by the Reviewer subagent
lint_command = "npm run lint"

[model_override]
# Override the default model for this project
# Useful for large projects where Pro provides better reasoning
lead_model = "deepseek-v4-pro"
subagent_model = "deepseek-v4-flash"

[instructions]
# Project-specific system prompt additions
# These are injected into the Lead agent's system prompt
# after the base prompt and before tool definitions
context = """
This is a REST API service using Express.js with TypeScript.
All routes are defined in src/routes/ with corresponding tests in src/routes/__tests__/.
Use Zod for input validation. Prefer async/await over callbacks.
Environment variables are loaded via dotenv and typed in src/config/env.ts.
"""

# Per-directory instructions (evaluated in path order)
[[instructions.directory]]
path = "src/routes"
context = "All route handlers must use the asyncHandler wrapper from src/utils/asyncHandler.ts."

[[instructions.directory]]
path = "src/db"
context = "All database access goes through the repository pattern in src/db/repositories/. Raw queries require explicit approval."

[tools]
# Additional tool definitions specific to this project
# Each entry becomes a callable tool in the agent's toolset
[[tools.custom]]
name = "run_migration"
description = "Run database migrations using knex. Use this when schema changes are needed."
command = "npx knex migrate:latest"
readonly = false

[[tools.custom]]
name = "generate_types"
description = "Generate TypeScript types from the database schema."
command = "npx kysely-codegen --out-file src/db/types.ts"
readonly = true

[subagents]
# Subagent configuration overrides for this project
[subagents.security_reviewer]
enabled = true
# Additional security rules for this project
rules = [
    "Flag any direct SQL interpolation outside src/db/repositories/",
    "Require rate-limit middleware on all POST/PUT/DELETE routes",
    "Verify JWT token validation uses the RS256 algorithm",
]

[subagents.test_runner]
enabled = true
# Coverage threshold for the TestRunner to enforce
min_coverage = 80

[git]
# Auto-commit behavior: "always", "ask", "never"
auto_commit = "always"
# Commit message prefix
commit_prefix = "ds: "
# Pre-edit checkpointing: create a branch before any changes
checkpoint_branch = "deepseek-backup"

[memory]
# Project-level memory files (injected into context automatically)
# Listed in priority order
files = [
    "ARCHITECTURE.md",
    "API_GUIDELINES.md",
    "TESTING_CONVENTIONS.md",
]
```

The `[instructions]` section implements the CLAUDE.md pattern: project-specific context is injected as user-level context (probabilistic compliance) rather than system prompt (deterministic compliance), which the model treats as strong suggestions ^2^. The `[[instructions.directory]]` entries support per-directory rules with path-based scoping, allowing different conventions for `src/routes/` versus `src/db/`.

#### 9.2.3 Configuration Precedence

Settings resolve in strict precedence, highest to lowest:

1. Command-line flags (`--model`, `--permission-mode`)
2. Environment variables (`DEEPSEEK_CODE_MODEL`, `DEEPSEEK_CODE_API_KEY`)
3. Project config (`.deepseek-code/project.toml`)
4. Global config (`~/.config/deepseek-code/config.toml`)
5. Built-in defaults

Array fields like `permissions.allow` and `permissions.deny` merge across scopes rather than replace. If the global config denies `Bash(rm -rf *)` and the project config allows `Bash(rm -rf /tmp/*)`, the deny rule still blocks because deny-first evaluation applies at merge time ^2^. This prevents a project-level configuration from silently weakening safety rules established at the global level.

### 9.3 Installation and Distribution

#### 9.3.1 Distribution Methods

The binary is distributed as a single static executable (Rust, `musl` target on Linux, native on macOS and Windows) with no runtime dependencies. Five installation channels cover the major platform and preference combinations:

| Method | Command | Platforms | Best For |
|--------|---------|-----------|----------|
| **Homebrew** | `brew install deepseek-code` | macOS, Linux | macOS users; auto-updates via `brew upgrade` |
| **Cargo** | `cargo install deepseek-code` | Any with Rust toolchain | Rust developers; build from source |
| **Install script** | `curl -fsSL ... \| sh` | macOS, Linux | Quick start; CI pipelines |
| **WinGet** | `winget install DeepSeek.Code` | Windows | Native Windows package management |
| **APT / DNF** | `apt install deepseek-code` / `dnf install deepseek-code` | Debian/Ubuntu, Fedora/RHEL | Linux system package managers |

The install script (hosted on GitHub Releases) detects the platform, downloads the correct binary, verifies the SHA-256 checksum against a signed manifest, and places the binary in `~/.local/bin` (or a user-specified prefix). It runs non-interactively and exits with a clear error if the checksum does not match. This is the method shown in quick-start documentation because it requires no prerequisite package manager.

The Homebrew tap (`deepseek-code/homebrew-tap`) and system package repositories (APT/DNF) are maintained via automated CI pipelines that trigger on release tags. Each release produces:

- `deepseek-code-{version}-x86_64-apple-darwin.tar.gz`
- `deepseek-code-{version}-aarch64-apple-darwin.tar.gz`
- `deepseek-code-{version}-x86_64-unknown-linux-musl.tar.gz`
- `deepseek-code-{version}-aarch64-unknown-linux-musl.tar.gz`
- `deepseek-code-{version}-x86_64-pc-windows-msvc.zip`
- `SHA256SUMS` (signed with the release signing key)

#### 9.3.2 First-Run Setup

On first invocation after installation, the binary runs an interactive setup wizard if no API key is configured:

```
$ deepseek-code

Welcome to DeepSeek Code TUI.

Step 1/4: API Key
Enter your DeepSeek API key (get one at https://platform.deepseek.com):
> sk-...

Step 2/4: Model Selection
Default model for main reasoning [deepseek-v4-pro]:
> deepseek-v4-pro
Subagent model for fast tasks [deepseek-v4-flash]:
> deepseek-v4-flash

Step 3/4: Project Scan
Scanning current directory... found tsconfig.json, package.json
Project type: TypeScript (Node.js)
Create .deepseek-code/project.toml? [Y/n]
> y

Step 4/4: Permission Mode
Permission mode:
  [1] interactive  - ask before every risky operation
  [2] accept_edits - auto-approve file edits, ask for commands
  [3] auto         - rule-based auto-approval (requires config)
  [4] dont_ask     - deny-only mode for automation
Select [1-4, default: 1]:
> 1

Setup complete. Run 'deepseek-code --help' for usage.
```

The project scan (Step 3) detects `package.json`, `Cargo.toml`, `go.mod`, `pyproject.toml`, `pom.xml`, and similar markers to infer language, framework, and test commands. It generates a starter `project.toml` with sensible defaults for the detected stack. Users can skip this with `--no-setup` to run in headless mode.

#### 9.3.3 Self-Update

The binary includes a self-update command that checks the GitHub Releases API, verifies artifact checksums, and performs an atomic swap:

```
$ deepseek-code --update

Current version: 0.3.1
Latest version:  0.4.0
Release notes:   https://github.com/deepseek-code/tui/releases/tag/v0.4.0

Download: 5.2 MB
Checksum: verified (SHA-256)

Update now? [Y/n]
> y

Downloading... done
Verifying signature... valid
Replacing binary (atomic)... done
Updated to 0.4.0
```

The update mechanism follows four steps: (1) query the releases API for the latest version tag; (2) download the platform-matching artifact and its `SHA256SUMS` file; (3) verify the checksum and GPG signature against the embedded public key; (4) write the new binary to a temporary path and perform an `rename()` syscall for atomic replacement. On Windows, where running executables cannot be overwritten, the updater renames the current binary to `.old`, writes the new binary to the original path, and schedules the `.old` file for deletion on next process exit.

If the user prefers their package manager, `deepseek-code --update` detects installation via Homebrew, APT, or DNF and defers to the appropriate manager (`brew upgrade deepseek-code`, `apt upgrade`, etc.) instead of performing a manual binary swap. This avoids version drift between the package manager's records and the actual binary on disk.
## 10. Roadmap, Evaluation, and Risks

This final chapter translates the architectural specifications from preceding chapters into an actionable development plan. It defines four release milestones, specifies the crate-level repository structure, establishes quantitative evaluation criteria, inventories 24 risks across three categories, enumerates the first ten engineering tasks, and closes with a concrete build-versus-defer recommendation.

### 10.1 Development Roadmap

The roadmap is divided into four milestones spanning twelve months. Each milestone delivers a shippable artifact with explicit scope boundaries; no milestone depends on features from subsequent milestones.

| Milestone | Timeline | Deliverables | Explicitly Excluded |
|-----------|----------|-------------|-------------------|
| **MVP 0.1** | Month 1–2 | Chat loop with DeepSeek V4, project scan, read/grep tools, SEARCH/REPLACE patch engine with 4-tier matching, diff review screen, command execution with permission confirmation, Pro/Flash routing, session save/load, TOML config | Memory system beyond session persistence; git integration beyond basic commit; subagents; MCP; plugins |
| **MVP 0.2** | Month 2–3 | Tiered memory (session/project/user), git checkpoint/undo, test runner integration, command palette, token/cost statistics panel, file tree with syntax highlighting, search index (tantivy) | Subagents; AST-aware editing; plugin system; advanced permission ML classifier |
| **v1.0** | Month 4–6 | Subagent orchestration (Planner, Implementer, Reviewer, TestRunner), MCP server support, project-wide AST index, plugin system (WASM), advanced permissions with ML classifier, multi-file transactions | Team mode; IDE bridge; cloud sync; enterprise policy server |
| **v2.0** | Month 7–12 | Background agent execution, multi-session dashboard, team mode with shared project memory, IDE bridge (LSP + extension), enterprise policy enforcement, analytics dashboard | AI model training; hosted SaaS; marketplace |

MVP 0.1 implements the agent loop from Chapter 7 without memory tiering, git checkpointing, or subagent delegation. Pro/Flash routing is present from day one: task classification and file reads route to V4 Flash ($0.14/M input), while planning, editing, and error recovery route to V4 Pro ($1.74/M input)^2^. This routing keeps median per-task cost below $0.05 for typical read-modify-test cycles. The TUI implements three of the twelve screens from Chapter 6: welcome/project-select, main 3-panel agent screen, and diff review^1^.

MVP 0.2 adds the three-tier memory system (Chapter 8.1) with the 12-table SQLite schema, git integration with pre-edit checkpoints and `/undo` via `git revert`, and the test-runner tool with convention-based file mapping^5^. The TUI gains the command palette (Ctrl+P), token/cost statistics, and the tool-call timeline screen.

v1.0 introduces the four subagent types from Chapter 9.1 — Planner, Implementer, Reviewer, and TestRunner — each running in isolated context with V4 Flash and returning 1–2K token summaries for 80–90% context savings versus full-history delegation^4^. MCP support enables integration with external tool servers. The permission system graduates from pattern-matching heuristics to an ML-based risk classifier following Claude Code's architecture^4^.

v2.0 targets team and enterprise workflows: background agents for long-running tasks, multi-session dashboard, team mode with shared project memory (synchronized via a `.deepseek/team/` git directory), IDE bridge with LSP companion and VS Code extension, and enterprise policy enforcement.

### 10.2 Repository Structure

The codebase is a Cargo workspace with twelve crates. Each crate has a single responsibility, a defined public interface, and explicitly declared dependencies on sibling crates.

```
deepseek-code/
├── Cargo.toml                    # workspace definition
├── crates/
│   ├── cli/                      # clap arguments, subcommands (ask, index, tui)
│   ├── tui/                      # ratatui widgets, screen state machine, event loop
│   ├── agent-core/               # ReAct loop, session manager, turn orchestration
│   ├── deepseek-client/          # API client: OpenAI + Anthropic endpoints, streaming
│   ├── tools/                    # built-in tool implementations (read, write, grep, bash, git, test)
│   ├── patch-engine/             # SEARCH/REPLACE parser, 4-tier matcher, transactions
│   ├── memory/                   # tiered memory, SQLite ops, context assembly, compaction
│   ├── indexer/                  # tree-sitter parser, tantivy index, symbol graph, file watcher
│   ├── permissions/              # risk classifier, rule engine, approval flow, sandbox
│   ├── config/                   # TOML parsing, hierarchy (global/project/local), validation
│   ├── telemetry/                # usage stats, cost tracking, error reporting (opt-in)
│   └── mcp/                      # MCP client, server discovery, tool adapter
└── docs/
```

**Per-crate responsibilities and public interfaces:**

| Crate | Primary Types | Depends On | Public API Surface |
|-------|--------------|------------|-------------------|
| `cli` | `Args`, `Subcommand`, `CliApp` | `config`, `tui`, `agent-core` | `main()` entry point; argument parsing only |
| `tui` | `App`, `Screen`, `Widget impls` | `agent-core`, `config` | `App::run(session)` — blocks until session ends |
| `agent-core` | `AgentLoop`, `Session`, `Turn`, `Router` | `deepseek-client`, `tools`, `patch-engine`, `memory`, `permissions` | `AgentLoop::run(task)` → `Stream<TurnEvent>` |
| `deepseek-client` | `Client`, `ChatRequest`, `StreamingResponse` | `telemetry` | `Client::chat(req)` → `Stream<Chunk>`; dual-format support |
| `tools` | `Tool trait`, `Read`, `Write`, `Bash`, `Git`, `Grep` | `permissions` | `Tool::execute(&self, ctx) -> ToolResult`; 15+ impls |
| `patch-engine` | `Patch`, `FileChange`, `Hunk`, `Matcher` | `indexer` (AST-aware matching) | `Patch::validate(files)` → `Result<ValidatedPatch>` |
| `memory` | `MemoryManager`, `ContextAssembler`, `Compactor` | `config` | `MemoryManager::load(session_id)`; `ContextAssembler::build()` → `Vec<Message>` |
| `indexer` | `Index`, `SymbolGraph`, `FileWatcher` | — (leaf crate) | `Index::search(query)` → `Vec<SearchResult>` |
| `permissions` | `RiskClassifier`, `RuleEngine`, `ApprovalRequest` | `config` | `RiskClassifier::score(cmd)` → `RiskLevel` |
| `config` | `Config`, `ModelConfig`, `PermissionConfig` | — (leaf crate) | `Config::load(path)` → `Arc<Config>`; merge hierarchy |
| `telemetry` | `UsageRecorder`, `CostTracker` | — (leaf crate) | `UsageRecorder::record(event)`; `CostTracker::session_total()` → `f64` |
| `mcp` | `McpClient`, `ServerDiscovery`, `ToolAdapter` | `tools` | `McpClient::connect(endpoint)` → `Vec<Box<dyn Tool>>` |

The dependency graph is strictly acyclic. Leaf crates (`config`, `telemetry`, `indexer`) have no internal dependencies. Integration crates (`agent-core`, `tui`) depend on multiple leaf crates but not on each other. The `cli` crate is the sole entry point with a `main` function.

### 10.3 Evaluation Plan

#### 10.3.1 Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Task success rate | ≥75% (MVP), ≥85% (v1) | Human annotator judges completion without human intervention |
| Patch correctness (first-apply) | ≥80% (MVP), ≥90% (v1) | Fraction of SEARCH/REPLACE blocks applying without tier fallback |
| Test pass rate after edit | ≥70% (MVP), ≥85% (v1) | Affected test suite passes within 3 retry attempts |
| Command safety: false negatives | 0 | Destructive commands executed without approval (audit log) |
| Command safety: false positives | ≤10% | Safe commands incorrectly flagged for approval |
| Approval burden | ≤2.5 prompts/task (MVP), ≤1.5 (v1) | Average permission prompts per completed task |
| End-to-end latency (simple task) | ≤15s median | Wall-clock from submission to response for "explain this function" |
| End-to-end latency (edit task) | ≤60s median | Wall-clock for read → edit → test → respond cycle |
| Token usage per task | ≤15K input, ≤3K output (median) | Token counts from API `usage` field |
| Cost per task | ≤$0.08 median, ≤$0.50 p95 | Token usage × model price |
| TUI frame time | ≤5ms p99 | Event receipt to render completion |
| Context cache hit rate | ≥60% after turn 3 | `cached_tokens / total_input_tokens` from API response |

#### 10.3.2 Benchmark Tasks

Six task categories cover core workflows. Each milestone must hit the targets above on these tasks before release:

**Fix bug.** The agent receives a bug report (error message + file location) and produces a fix passing the relevant test. Example: "`panicked at 'index out of bounds'` in `src/parser.rs:142`."

**Add feature.** The agent implements a new function following existing patterns. Example: "Add a `DELETE /api/users/:id` endpoint following the existing `POST` handler."

**Write tests.** The agent generates tests achieving ≥80% line coverage for a given module. Example: "Write unit tests for `RateLimiter::check()` in `src/limit.rs`."

**Refactor.** The agent restructures code without changing behavior, verified by existing tests. Example: "Extract database access logic from `handler.rs` into `repository.rs`."

**Update dependency.** The agent updates a dependency version and fixes breaking changes. Example: "Update `tokio` from 1.0 to 1.5 and fix API changes."

**Explain codebase.** The agent answers architectural questions about an unfamiliar project. Example: "How does authentication work? Trace the flow from request to validation."

Each task runs against three project sizes: small (≤50 files), medium (≤500 files), and large (≤5000 files). Metrics targets apply to the medium category.

### 10.4 Risk Analysis

#### 10.4.1 Technical Risks (10 items)

| # | Risk | Sev | Prob | Mitigation |
|---|------|-----|------|------------|
| T1 | DeepSeek API breaking changes; legacy names deprecated July 2026^2^| 4 | Med | Dual endpoint support (OpenAI + Anthropic); endpoint switch is config-only |
| T2 | Model hallucination produces incorrect edits that pass tests | 4 | High | Exact-match SEARCH/REPLACE; diff review screen; test gate on every edit |
| T3 | V4 Flash inadequate for subagent reasoning (v1.0) | 3 | Med | Fallback to V4 Pro; human review of subagent outputs during training |
| T4 | TUI rendering bugs on Windows terminal emulators | 3 | Med | crossterm backend; CI on Windows runners; ConEmu/Windows Terminal priority |
| T5 | ratatui performance degrades at >1000 file tree nodes | 2 | Med | Virtualized List widget; incremental loading; benchmarked at 10K nodes |
| T6 | SQLite contention under concurrent subagent writes | 3 | Med | WAL mode; per-subagent in-memory buffers; single writer thread |
| T7 | Fuzzy match tier too permissive — accepts wrong block | 3 | Low | Uniqueness guard (second-best score ≥0.15 lower)^3^; mandatory user review |
| T8 | Token budget miscalculation causes context overflow | 3 | Med | 8K token safety buffer; compaction triggers at 80% capacity^6^|
| T9 | Tree-sitter grammar unavailable for niche language | 2 | Low | Graceful degradation to text-only; community grammar contributions |
| T10 | DeepSeek rate limits throttle agent during peak usage | 3 | Med | Exponential backoff; request queue with priority; Flash as Pro fallback |

#### 10.4.2 Product Risks (6 items)

| # | Risk | Sev | Prob | Mitigation |
|---|------|-----|------|------------|
| P1 | Claude Code habit and subscription inertia block adoption | 4 | High | Per-task cost transparency; single-binary zero-config; compatible keybindings |
| P2 | Feature parity pressure causes scope creep | 3 | High | Explicit defer list (Section 10.5.2); compete on cost and speed, not feature count |
| P3 | Cost unpredictability on complex tasks | 3 | Med | Real-time cost display; task-level budget caps; monthly spending alerts |
| P4 | Early releases perceived as unreliable | 4 | Med | Default read-only mode; git rollback on every edit; dogfood before release |
| P5 | Open-source sustainability without revenue | 3 | Med | Optional paid cloud tier (team sync, managed MCP); core tool remains free |
| P6 | DeepSeek pricing increase eliminates cost advantage | 4 | Low | Model-agnostic backend; cost optimizer selects cheapest capable model |

#### 10.4.3 Mitigation Strategies

**Abstraction layers isolate external dependencies.** The `deepseek-client` crate unifies OpenAI-format and Anthropic-format APIs behind a single request/response type. If DeepSeek changes endpoints, only one crate changes. The `tools` crate defines a `Tool` trait implemented by built-in tools, MCP tools, and plugin tools alike.

**Incremental delivery validates assumptions early.** MVP 0.1 ships in 6–8 weeks. If SEARCH/REPLACE fails to achieve the 80% first-apply target on real codebases, the team can pivot to `str_replace` or full-file rewrite before building the rest of the system around a flawed primitive^3^.

**Community building starts at launch.** The repository opens on day one with `CONTRIBUTING.md`, issue templates, and a public roadmap. Early adopters contribute tree-sitter grammars, MCP adapters, and themes — scaling language coverage without core team effort.

### 10.5 First 10 Engineering Tasks and Final Recommendation

#### 10.5.1 Immediate Next Steps

These ten tasks, in order, take the project from repository creation to the first end-to-end demo. Effort estimates assume two full-time Rust developers.

| # | Task | Crate | Effort | Dependencies |
|---|------|-------|--------|--------------|
| 1 | Repository setup: Cargo workspace, CI (GitHub Actions), lint, test harness, license | root | 1 day | — |
| 2 | DeepSeek API client: OpenAI endpoint, SSE streaming, `reasoning_content` separation | `deepseek-client` | 3 days | Task 1 |
| 3 | Basic TUI loop: ratatui init, crossterm events, 60 FPS render, screen state machine | `tui` | 4 days | Task 1 |
| 4 | File read tool: path resolution, SHA-256 hash, in-memory cache | `tools` | 2 days | Task 1 |
| 5 | Command execution: Bash tool with timeout, output truncation, secrets redaction DFA | `tools` | 3 days | Task 4 |
| 6 | Grep tool: ripgrep wrapper with pattern, path, context lines | `tools` | 2 days | Task 4 |
| 7 | SEARCH/REPLACE parser: block parsing, well-formedness validation | `patch-engine` | 3 days | Task 4 |
| 8 | 4-tier matcher: exact → whitespace → indent → fuzzy; similarity scoring; full-file fallback | `patch-engine` | 4 days | Task 7 |
| 9 | Agent loop integration: ReAct wiring (tools → permissions → execution → response) | `agent-core` | 4 days | Tasks 2, 3, 5, 6, 8 |
| 10 | Pro/Flash routing + config: complexity classifier, model selection, TOML config, cost display | `agent-core`, `config` | 3 days | Task 9 |

Total: 29 days — approximately 6 weeks for two developers accounting for integration, review, and testing. Task 9 produces the first demo: a user types a request, the agent reads files, proposes a SEARCH/REPLACE edit, and shows a diff.

#### 10.5.2 What Not to Build Yet

**Background agents** (v2.0). Asynchronous execution requires a job queue, persistence layer, and notification system — infrastructure that adds no value until the interactive loop is reliable.

**Team mode** (v2.0). Shared memory and conventions require synchronization protocols, conflict resolution, and access control. Individual adoption must precede team adoption.

**Cloud sync** (indefinite). Hosted storage of conversation history or API keys introduces GDPR and SOC-2 obligations. All data stays local until enterprise demand justifies the compliance investment.

**IDE bridge** (v2.0). Splitting effort between TUI and VS Code extension dilutes both. The TUI must prove terminal-native agents match IDE-integrated tools on core tasks first.

**ML permission classifier** (v1.0). Pattern-matching + heuristics achieve ≤10% false positives with zero training overhead^4^. The ML upgrade waits for accumulated labeled data.

**Vector database for memory** (indefinite). SQLite + BM25 search achieves required retrieval quality at lower complexity than vector embeddings^5^. Tantivy's vector search can be added later without schema changes.

#### 10.5.3 Final Recommendation

Start MVP 0.1 immediately. The core hypothesis — that a Rust-built, DeepSeek-native TUI agent can match Claude Code's core loop at 10–40x lower cost — is testable in 6–8 weeks. The components (DeepSeek client, SEARCH/REPLACE engine, ratatui TUI, ReAct loop) have established patterns from prior art; none require research.

Implementation order matters. Build the patch engine first (Tasks 7–8): it is the riskiest component because matching quality determines whether the agent can edit reliably. If the 4-tier matcher fails ≥80% first-apply on real commits, the project must pivot format before committing the architecture^3^. Build the TUI loop second (Task 3): it surfaces agent behavior and provides the diff review safety surface. The API client (Task 2) is third — straightforward, but must be validated against the live DeepSeek streaming endpoint.

Ship MVP 0.1 when it completes three of six benchmark tasks (fix bug, add feature, write tests) on small-to-medium projects under $0.08 median cost. Do not wait for git integration, memory, or subagents — those are MVP 0.2 expansions that improve efficiency but are not required to validate the core hypothesis.

The market gap identified in Chapter 2 remains open: no tool combines DeepSeek-native optimization, Rust performance, full TUI experience, and open-source distribution. Claude Code is vendor-locked and priced at $20–200/month. Aider lacks TUI polish. Goose is MCP-first, not TUI-first. The window is bounded by how quickly the ecosystem moves to support DeepSeek V4. A 6–8 week MVP delivery lands while DeepSeek's 75% launch discount on V4 Pro remains active through May 31, 2026^2^, creating a compelling cost narrative for early adopters.

Build the core loop. Validate the patch engine. Ship.
