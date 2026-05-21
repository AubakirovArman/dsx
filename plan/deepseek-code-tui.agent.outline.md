# DeepSeek Code TUI: Architecture & Product Document

## Executive Summary
### Project Overview
#### DeepSeek Code TUI is a next-generation terminal coding agent powered by DeepSeek V4, built in Rust
#### Positioned as open-source, model-agnostic, performance-optimized alternative to Claude Code/Codex CLI
#### Three modes: Simple CLI, Full TUI, Agent Workspace — serving different user workflows

### Key Differentiators
#### Native DeepSeek V4 optimization (Pro/Flash routing, thinking mode, context caching)
#### Rust-built performance (30-40% less memory, single binary, sub-millisecond TUI rendering)
#### Full TUI IDE experience (panels, file tree, diff view, command palette — not just chat)
#### SEARCH/REPLACE patch engine with 4-tier matching for reliable edits
#### Tiered permission system with automatic risk classification

## 1. Confirmed DeepSeek V4 API Capabilities (~2000 words, 3 tables)
### 1.1 Model Specifications
#### 1.1.1 V4 Pro: 1.6T total params, 49B active, 1M context, $1.74/M input, $3.48/M output
#### 1.1.2 V4 Flash: 284B total, 13B active, 1M context, $0.14/M input, $0.28/M output
#### 1.1.3 Both models: MIT license, weights on Hugging Face, OpenAI + Anthropic API compatibility
### 1.2 Core API Features
#### 1.2.1 Tool calling: OpenAI-style function calling, max 128 functions, parallel tool calls supported
#### 1.2.2 Thinking mode: enabled by default via `thinking.type` parameter, depth via `reasoning_effort` (high/max)
#### 1.2.3 Streaming: SSE format with `delta.reasoning_content` before `delta.content`, `stream_options.include_usage`
#### 1.2.4 JSON mode: `response_format.type: json_object`, requires "json" in prompt
### 1.3 API Endpoints and Migration
#### 1.3.1 OpenAI-compatible: `https://api.deepseek.com`, model: `deepseek-v4-pro` or `deepseek-v4-flash`
#### 1.3.2 Anthropic-compatible: `https://api.deepseek.com/anthropic`, effort via `output_config.effort`
#### 1.3.3 Legacy deprecation: `deepseek-chat` and `deepseek-reasoner` deprecated July 24, 2026
### 1.4 Cost Optimization
#### 1.4.1 Context caching: automatic, cache hit at 1/10 price ($0.003625/M for Pro), stable prefix design
#### 1.4.2 75% launch discount on Pro extended to May 31, 2026
#### 1.4.3 Cost comparison table: DeepSeek vs Claude vs Gemini vs GPT-4 pricing per 1M tokens

## 2. Competitor Analysis (~3000 words, 2 tables)
### 2.1 Tier 1: Premium Commercial Agents
#### 2.1.1 Claude Code: 7 permission modes, subagents, 1M context, $20-200/mo, proprietary — best agentic reasoning
#### 2.1.2 Codex CLI: cloud sandbox, async execution, GPT-5 — best for parallel task execution
#### 2.1.3 Gemini CLI: open source (Apache 2), free tier 1000 req/day, PTY streaming — best free option
### 2.2 Tier 2: Open Source Multi-Model Agents
#### 2.2.1 Aider: git-native atomic commits, 70+ models, SEARCH/REPLACE editing — best git integration
#### 2.2.2 OpenCode: LSP integration, YAML subagent architecture, SQLite sessions — most inspectable
#### 2.2.3 Goose: MCP-first, Rust-built, Linux Foundation — best extensibility
### 2.3 Tier 3: IDE-Integrated Agents
#### 2.3.1 Roo Code/Kilo Code: mode system (Code/Architect/Ask/Debug), VS Code extension
#### 2.3.2 Zed AI: 120fps GPU rendering, ACP protocol, open-weight edit model — best native performance
#### 2.3.3 Continue.dev: true open source, air-gapped deployment, plan mode sandbox — best privacy
### 2.4 Competitor Matrix and Gaps
#### 2.4.1 Feature matrix table: 10 tools × 15 features (tool calling, subagents, MCP, git, TUI, etc.)
#### 2.4.2 Identified market gap: no tool combines DeepSeek-native + Rust performance + full TUI + open source
#### 2.4.3 Strategic positioning: compete on performance, cost, and DeepSeek optimization

## 3. Product Vision (~2000 words)
### 3.1 What This Is
#### 3.1.1 Primary: TUI/CLI coding agent — terminal-native development environment powered by DeepSeek V4
#### 3.1.2 Secondary: DeepSeek Dev Agent CLI — command-line interface for AI-assisted coding
#### 3.1.3 Identity: open-source, Rust-built, model-agnostic with DeepSeek-native optimization
### 3.2 Three Product Modes
#### 3.2.1 Simple CLI mode: command → response, file read, diff proposal — for quick tasks and scripting
#### 3.2.2 Full TUI mode: interactive panels, file tree, chat, diff view, command logs, token stats — for sessions
#### 3.2.3 Agent Workspace mode: multi-session, subagents, background tasks, project memory, checkpoints — for projects
### 3.3 Target Users and Use Cases
#### 3.3.1 Individual developers: quick edits, refactoring, test writing, bug fixes — Simple CLI + TUI
#### 3.3.2 Teams: code review, onboarding, documentation, consistency enforcement — Agent Workspace
#### 3.3.3 Enterprises: air-gapped deployment, policy compliance, audit trails — all modes with local features

## 4. Core Feature Set (~3000 words, 1 table)
### 4.1 Project Awareness (Category A)
#### 4.1.1 Repo scanning: file tree, gitignore support, language detection, dependency detection
#### 4.1.2 Codebase indexing: tree-sitter AST, symbol graph, file summaries — repo-map inspired
#### 4.1.3 Project instructions: `.deepseek-code/instructions.md` hierarchical config
### 4.2 Chat + Coding Loop (Category B)
#### 4.2.1 Natural language tasks: plan generation, file read, grep/search, edit proposal
#### 4.2.2 Diff review: SEARCH/REPLACE preview, approve/reject, apply, format, test, retry loop
#### 4.2.3 Error handling: test run integration, lint error detection, auto-fix loop
### 4.3 TUI UX (Category C)
#### 4.3.1 Layout: main screen with panels (chat, file tree, diff, logs, plan, tool calls)
#### 4.3.2 Navigation: keyboard shortcuts, command palette, tabs, vim-like bindings optional, mouse support
#### 4.3.3 Status bar: model selector, cost/token meter, permission mode, session info, git branch
### 4.4 Safety, Memory, Tools, Subagents (Categories D-G)
#### 4.4.1 Safety: read-only mode, ask-before-edit/command, yolo mode, allowlist/denylist, sandbox
#### 4.4.2 Memory: session + project + user preferences, SQLite storage, decision/command/error logs
#### 4.4.3 Tool system: 15+ built-in tools (read/write/edit/grep/run_command/git/test/etc.), MCP integration
#### 4.4.4 Subagents: planner, implementer, reviewer, test-runner — V4 Flash for subagents, Pro for main

## 5. Architecture and Tech Stack (~3000 words, 2 diagrams, 1 table)
### 5.1 Architecture Overview
#### 5.1.1 21-layer architecture: CLI → TUI → Router → Session → Agent → Model → DeepSeek Client → Tools → FS → Patch → Git → Sandbox → Memory → Indexer → Prompt → Context → Permissions → Config → Plugin → Telemetry → Updater
#### 5.1.2 ASCII architecture diagram showing data flow and layer dependencies
#### 5.1.3 Crate-based modular design: 12+ crates in workspace
### 5.2 Tech Stack Decision
#### 5.2.1 Rust wins: ratatui + tokio + reqwest + serde + clap + git2 + tree-sitter + tantivy + sqlx + similar
#### 5.2.2 Comparison table: Rust vs Go vs TypeScript vs Python — performance, ecosystem, distribution, dev speed
#### 5.2.3 Key crates and versions with justification for each choice
### 5.3 Async Architecture
#### 5.3.1 tokio runtime with SSE streaming via reqwest-eventsource
#### 5.3.2 Event-driven TUI loop: merge keyboard events + API stream + file watcher + subagent results
#### 5.3.3 Concurrent operations: API calls non-blocking, tool execution parallel where safe

## 6. TUI Design (~2500 words, 12 ASCII mockups)
### 6.1 Screen Specifications
#### 6.1.1 Welcome/project select: recent projects, new project, clone repo, settings access
#### 6.1.2 Main agent screen: 3-panel layout (file tree left, chat center, status bottom)
#### 6.1.3 Chat screen: message history, streaming display, thinking content toggle, code blocks
#### 6.1.4 Plan screen: task breakdown, step status, dependency graph, estimated tokens/cost
#### 6.1.5 Diff review: side-by-side SEARCH/REPLACE, syntax highlighting, accept/reject per hunk
#### 6.1.6 Tool call timeline: chronological tool execution, parameters, results, timing
#### 6.1.7 Command execution: terminal output stream, exit code, timeout indicator, kill button
#### 6.1.8 Test results: pass/fail summary, failure details, coverage indicator, retry option
#### 6.1.9 Memory/session: session list, checkpoint browser, memory search, restore point
#### 6.1.10 Settings: model selector (Pro/Flash), permission mode, theme, keybindings, API key
#### 6.1.11 MCP/tools: installed tools, MCP servers, enable/disable, permission per tool
#### 6.1.12 Logs/debug: structured logs, tracing spans, error details, export option
### 6.2 Design System
#### 6.2.1 Color theme: dark default, low-saturation palette, syntax highlighting via tree-sitter
#### 6.2.2 Keyboard shortcuts: Ctrl+P command palette, Ctrl+T new tab, Ctrl+D diff view, Esc back
#### 6.2.3 Responsive layout: constraint-based ratatui layouts adapting to terminal size

## 7. Agent Loop and Prompting Protocol (~2500 words, 1 diagram, pseudocode)
### 7.1 Agent Loop Design
#### 7.1.1 14-step loop: task → classify → route model → collect context → generate plan → check permissions → execute tools → propose patch → diff review → test → error handle → iterate → respond → update memory
#### 7.1.2 ASCII sequence diagram showing agent loop with user, model, tools, and permission gates
#### 7.1.3 Pseudocode for core agent loop with error recovery ladder
### 7.2 Execution Modes
#### 7.2.1 No-edit mode: read-only, analysis and explanation only
#### 7.2.2 Plan-only mode: generates plan, pauses for approval before execution
#### 7.2.3 Auto-approve mode: non-destructive operations automatic, destructive still ask
#### 7.2.4 YOLO mode: minimal confirmation, for trusted environments only
#### 7.2.5 Dry-run mode: simulates actions without side effects, shows what would happen
### 7.3 Prompting Protocol
#### 7.3.1 Main coding agent prompt: identity, tool use rules, safety guidelines, output format
#### 7.3.2 Planner prompt: task decomposition, dependency analysis, complexity estimation
#### 7.3.3 Implementer prompt: code generation, style adherence, test requirement
#### 7.3.4 Reviewer prompt: code review, bug detection, security check, style compliance
#### 7.3.5 DeepSeek-specific: thinking mode for planning/debugging, non-thinking for summaries

## 8. Memory, Safety, and Patch Engine (~2500 words, 2 diagrams, schemas)
### 8.1 Memory and Context System
#### 8.1.1 Tiered memory: session (conversation) → project (file summaries, decisions) → user (preferences)
#### 8.1.2 SQLite schema: 12 tables (sessions, messages, file_summaries, decisions, commands, errors, patches, checkpoints, preferences, tool_results, memory_index, usage_stats)
#### 8.1.3 Context assembly: 9 ordered sources (system prompt, project instructions, memory, file contents, search results, git status, conversation history, tool schemas, user message)
### 8.2 Tool Execution Safety
#### 8.2.1 Command classifier: risk levels (safe/sensitive/destructive) with pattern matching + heuristics
#### 8.2.2 Permission rules: default-deny destructive, ask for sensitive, allow safe commands
#### 8.2.3 Sandbox: timeout (30s default), output truncation (10K lines), secrets redaction, cwd restrictions
#### 8.2.4 Destructive command detection: rm, drop, delete, format, dd, etc. with confirmation required
### 8.3 Patch Engine
#### 8.3.1 SEARCH/REPLACE primary format: 4-tier matching (exact → whitespace-insensitive → indent-preserving → fuzzy)
#### 8.3.2 Edit pipeline: read → generate → validate → preview → approve → apply → format → test → rollback-if-fail
#### 8.3.3 Data structures: Patch, FileChange, Hunk, ToolCall, ApprovalRequest, ExecutionResult
#### 8.3.4 Multi-file transactions: all-or-nothing validation, git checkpoint before, rollback on failure

## 9. Subagents, Config, and Distribution (~2000 words, config examples)
### 9.1 Subagent System
#### 9.1.1 MVP subagents: Lead (main), Planner (task decomposition), Implementer (code changes), Reviewer (code review)
#### 9.1.2 v1 additions: TestRunner, SecurityReviewer, DocsWriter, GitAgent, DependencyAgent
#### 9.1.3 Subagent design: V4 Flash for speed, isolated context, limited toolset, result summary to parent
### 9.2 Configuration Files
#### 9.2.1 Global config: `~/.config/deepseek-code/config.toml` — API key, default model, theme, permissions
#### 9.2.2 Project config: `.deepseek-code/project.toml` — model override, instructions, tools, team settings
#### 9.2.3 Examples: complete config.toml and project.toml with all options documented
### 9.3 Installation and Distribution
#### 9.3.1 Methods: Homebrew, cargo install, install script (curl | sh), WinGet, APT/DNF
#### 9.3.2 First-run setup: API key prompt, model selection, project scan, permission mode selection
#### 9.3.3 Self-update: check latest release, download binary, verify checksum, atomic swap

## 10. Roadmap, Evaluation, and Risks (~2500 words, 2 tables)
### 10.1 Development Roadmap
#### 10.1.1 MVP 0.1 (month 1-2): minimal chat, project scan, read/grep, SEARCH/REPLACE patch, diff review, command execution with confirmation, Pro/Flash routing, session save, simple config
#### 10.1.2 MVP 0.2 (month 2-3): memory system, git integration, test runner, improved TUI, command palette, token/cost stats
#### 10.1.3 v1.0 (month 4-6): subagents, MCP support, project index, AST search, plugin system, advanced permissions
#### 10.1.4 v2.0 (month 7-12): background agents, multi-session dashboard, team mode, IDE bridge, enterprise policies
### 10.2 Repository Structure
#### 10.2.1 Workspace layout: 12 crates (cli, tui, agent-core, deepseek-client, tools, patch-engine, memory, indexer, permissions, config, telemetry, mcp)
#### 10.2.2 Per-crate responsibilities, public interfaces, and dependencies
### 10.3 Evaluation Plan
#### 10.3.1 Metrics: task success rate, patch correctness, test pass rate, command safety, approval burden, latency, token usage, cost per task
#### 10.3.2 Benchmark tasks: fix bug, add feature, write tests, refactor, update dependency, explain codebase
### 10.4 Risk Analysis
#### 10.4.1 Technical risks: DeepSeek API changes, model hallucination, TUI complexity, Windows compatibility
#### 10.4.2 Product risks: user adoption against established tools, feature parity pressure, cost unpredictability
#### 10.4.3 Mitigation strategies: abstraction layers, extensive testing, incremental delivery, community building
### 10.5 First 10 Engineering Tasks and Final Recommendation
#### 10.5.1 Immediate next steps: repo setup, DeepSeek API client, basic TUI loop, file reading, command execution
#### 10.5.2 What not to build yet: background agents, team mode, cloud sync, IDE bridge
#### 10.5.3 Final recommendation: start MVP 0.1 immediately, focus on core loop + patch engine + TUI, ship in 6-8 weeks

# References
## Research Files
- deepseek_v4_api_dim01.md — DeepSeek V4 API capabilities
- rust_tui_ecosystem_dim02.md — Rust TUI ecosystem analysis
- competitors_additional_dim03.md — Additional competitor analysis
- agent_architecture_patterns_dim04.md — Agent architecture patterns
- code_editing_patch_engine_dim05.md — Code editing and patch engine
- deepseek_code_tui_insight.md — Cross-dimension insights
- deepseek_code_tui_cross_verification.md — Cross-verification results
