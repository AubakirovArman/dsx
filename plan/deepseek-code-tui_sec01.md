# Executive Summary

## Project Overview

DeepSeek Code TUI is a terminal-native coding agent powered by DeepSeek V4, implemented in Rust as a single statically-linked binary. It provides an open-source, model-agnostic alternative to closed-source tools such as Claude Code and the Codex CLI, with first-class optimization for DeepSeek's model family — including automatic Pro/Flash routing, thinking mode configuration, and context cache exploitation.

The project addresses a specific gap in the current tooling landscape. Existing coding agents fall into two camps: closed-source, vendor-locked solutions (Claude Code, Codex CLI) that optimize for a single model family; and open-source tools (Aider, Goose) that trade user experience for flexibility. DeepSeek Code TUI occupies a third position — open source with native DeepSeek optimization, delivering a full IDE-like TUI experience inside the terminal while remaining model-agnostic at the backend.

The architecture rests on three design commitments. First, terminal-native operation through ratatui's immediate-mode rendering engine, enabling sub-millisecond frame updates and IDE-grade panel layouts (file tree, chat, diff view, command palette, token statistics) without leaving the terminal[^2^]. Second, DeepSeek-native API exploitation — automatic context caching (cache hit pricing at one-tenth of standard input rates[^1^]), streaming reasoning token separation via `delta.reasoning_content`, and dual OpenAI/Anthropic-compatible endpoint support. Third, a Rust implementation targeting 30--40% lower memory consumption than comparable Go-based TUIs, with single-binary distribution and zero runtime dependencies[^2^].

The tool operates in three distinct modes — Simple CLI, Full TUI, and Agent Workspace — each targeting a different developer workflow. Simple CLI handles quick tasks (single-command execution, file reads, diff proposals) with minimal overhead. Full TUI provides an interactive session environment with persistent panels, file tree navigation, inline diff rendering, and a command palette. Agent Workspace extends the TUI with multi-session management, subagent delegation, background task execution, project memory, and automatic git checkpoints — designed for sustained project-level work.

## Key Differentiators

**Native DeepSeek V4 optimization.** The tool implements automatic model routing between V4 Pro (1.6T parameters, 49B active, $1.74/M input tokens) and V4 Flash (284B parameters, 13B active, $0.14/M input tokens), selecting the appropriate model based on task complexity[^1^]. Thinking mode is automatically configured per-operation: `reasoning_effort="high"` for standard edits, `"max"` for complex architectural tasks. The streaming pipeline handles `delta.reasoning_content` chunks before `delta.content`, enabling real-time display of the model's reasoning chain alongside its output[^1^]. Automatic context caching reduces repeated-prefix costs by 10x, with cache hit tokens billed at $0.003625/M for Pro[^1^].

**Rust-built performance.** The ratatui framework (19.1k GitHub stars, stable v0.30+) provides constraint-based responsive layouts and immediate-mode rendering that sustains 60 FPS on commodity hardware[^2^]. The full dependency stack — tokio for async, tree-sitter for code parsing across 100+ languages, tantivy for sub-10ms code search, git2 for repository operations — compiles to a single binary with no runtime prerequisites[^2^]. Memory benchmarks against Go's Bubble Tea show 30--40% reduction in resident set size, a meaningful advantage for a tool that runs alongside IDEs and language servers[^2^].

**SEARCH/REPLACE patch engine with 4-tier matching.** File edits use Aider's proven SEARCH/REPLACE block format, applied through a cascading matcher: exact string match, whitespace-insensitive match, indentation-preserving match, and fuzzy similarity scoring[^5^]. Research on LLM-generated diffs shows that line-number-based formats (unified diff) fail at rates exceeding 86%, while exact-string replacement fails 15--20% of the time due to cached content drift[^5^]. SEARCH/REPLACE blocks avoid both failure modes by being content-addressed (no line numbers) and tolerance-matched. The engine falls back to full-file rewrite for files under 300 lines where patch reliability drops.

**Tiered permission system with automatic risk classification.** Human-in-the-loop approval is insufficient as a safety mechanism: empirical analysis of Claude Code usage shows approximately 93% of permission prompts are approved by users, indicating automation bias[^4^]. DeepSeek Code TUI implements a command risk scorer from day one, using pattern matching and heuristics for the MVP and graduating to an ML classifier for v1. The system evaluates every proposed tool call against a deny-first rule matrix: destructive operations (`rm`, `git reset --hard`, database writes) require explicit approval; read-only operations proceed automatically; intermediate-risk operations (file writes, git commits) follow configurable policy[^4^].

**Subagent orchestration with context isolation.** Delegation to subagents returns 1--2K token summaries instead of 10K+ full execution histories, yielding 80--90% context savings per delegation[^4^]. The Agent Workspace mode uses V4 Flash for subagent tasks (fast, inexpensive) and reserves V4 Pro for the main reasoning loop. Each subagent operates in an isolated context with its own tool set and optional git worktree, preventing context pollution of the parent session[^4^].

---

# 3. Product Vision

## 3.1 What This Is

### 3.1.1 Primary Identity: TUI/CLI Coding Agent

DeepSeek Code TUI is, at its core, a terminal-native development environment powered by the DeepSeek V4 model family. It is not a chat interface wrapped around an API call; it is a full agent runtime that maintains persistent state across turns, executes tools in streaming parallel, manages context through a five-stage compaction pipeline, and renders all output through a terminal UI with IDE-grade panel layouts.

The agent implements the ReAct pattern (Reasoning + Acting) as its fundamental control loop[^4^]: the user provides an instruction, the model reasons (visible via the `reasoning_content` stream), selects tools to execute, receives observations, and iterates until the task completes. Each tool call — whether reading a file, running a test, or searching the codebase — passes through a permission gate before execution. The loop terminates on five conditions: no further tool calls requested, maximum turn count reached, context window exhausted, a hook intervention blocks execution, or the user aborts[^4^].

The architecture separates concerns into three layers: the TUI layer (ratatui widgets, crossterm event handling, constraint-based layout), the business logic layer (conversation state, file cache, search index, configuration), and the service layer (LLM API client via reqwest-eventsource, git operations via git2/gix, file watching via notify, syntax parsing via tree-sitter)[^2^]. An async event loop (tokio) merges multiple event streams — keyboard input, timer ticks, API SSE chunks, file system notifications, and git operation completions — into a unified event type that drives state transitions[^2^].

### 3.1.2 Secondary Identity: DeepSeek Dev Agent CLI

Beyond the interactive TUI, the tool functions as a scriptable command-line interface for AI-assisted coding. The `ask` subcommand accepts a prompt and optional file attachments, executes the full agent loop non-interactively, and returns the result — suitable for shell pipelines, CI/CD integration, and editor plugins. The `index` subcommand builds a tantivy search index over the codebase for fast symbol and content retrieval. The `tui` subcommand (default) launches the interactive interface.

This dual identity — interactive TUI for exploration, CLI for automation — mirrors the workflow of tools like `git` (both interactive and scriptable) rather than chat-only agents.

### 3.1.3 Project Identity: Open Source, Rust-Built, Model-Agnostic

The project ships under an open-source license (MIT/Apache-2.0 dual-licensed) with DeepSeek-native optimization as its primary focus and model-agnostic support as a secondary capability. The backend API client can target any OpenAI-compatible endpoint (OpenAI, Anthropic-compatible, local Ollama, OpenRouter) by swapping the base URL and model name, but the tool's default configuration, cost-optimization logic, and streaming pipeline are calibrated for DeepSeek V4 specifically.

The implementation language is Rust (edition 2024, MSRV 1.85), selected because no other ecosystem combines a production-grade TUI framework (ratatui), async runtime (tokio), code parser (tree-sitter), and search engine (tantivy) with zero-cost abstractions and single-binary distribution[^2^]. Go's Bubble Tea (40.7k stars) and TypeScript's Ink (35.6k stars) are viable alternatives for simpler applications but lack tantivy-equivalent search and are 2--3 years behind ratatui in layout system maturity[^2^].

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

**Subagent delegation.** Complex tasks are automatically or manually decomposed into subagent calls. A subagent receives a fresh context containing only the task description and relevant file excerpts, executes independently, and returns a structured summary (1--2K tokens vs. the 10K+ full history that would pollute the parent context)[^4^]. Subagents use V4 Flash by default; the parent agent uses V4 Pro. Subagents can themselves delegate, though depth is capped at two to prevent exponential context growth.

**Background tasks.** Long-running operations — full codebase indexing, comprehensive test suite execution, documentation generation across the entire project — execute in background tasks that report progress via the status bar and command log. The user continues interacting with the foreground agent while background work proceeds.

**Project memory.** A tiered memory system persists across sessions: (a) session memory (conversation history, current context window), (b) project memory (file summaries, architectural decisions, API contracts — stored in SQLite), (c) user preferences (model defaults, permission policy, key bindings — stored in `~/.config/dsctui/`), and (d) tool result logs (historical tool outputs for audit and pattern learning)[^4^]. Even with DeepSeek's 1M context window, relying solely on the context window for memory leads to pollution and degraded performance; the tiered approach keeps the context window focused on the active working set[^4^].

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

**Policy compliance.** Enterprise administrators distribute a `~/.config/dsctui/enterprise.toml` file that overrides user settings: models restricted to approved endpoints, permission levels set organization-wide (e.g., `execute = "deny"` globally), subagent delegation disabled, all tool calls logged to a SIEM-compatible audit trail. The permission system's deny-first evaluation ensures that even compromised credentials cannot override policy[^4^].

**Audit and governance.** Every tool call, model response, and user decision is logged with timestamps, token counts, cost estimates, and git commit hashes. The audit log is append-only, written to both local storage and (optionally) a centralized log aggregator. This satisfies compliance requirements for code changes produced with AI assistance — a growing concern in regulated industries.

| User Segment | Primary Mode | Key Feature | Value Proposition |
|---|---|---|---|
| Individual developer | Simple CLI + Full TUI | Fast iteration, keyboard-first TUI | Replace manual typing for mechanical tasks; terminal-native workflow |
| Team | Agent Workspace | Shared memory, multi-session, rules | Consistent code quality; faster onboarding; shared project context |
| Enterprise | All modes (configured) | Air-gapped support, audit logging, policy enforcement | Regulatory compliance; zero external data exposure; governance |
