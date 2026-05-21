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

MVP 0.1 implements the agent loop from Chapter 7 without memory tiering, git checkpointing, or subagent delegation. Pro/Flash routing is present from day one: task classification and file reads route to V4 Flash ($0.14/M input), while planning, editing, and error recovery route to V4 Pro ($1.74/M input)[^1^]. This routing keeps median per-task cost below $0.05 for typical read-modify-test cycles. The TUI implements three of the twelve screens from Chapter 6: welcome/project-select, main 3-panel agent screen, and diff review[^2^].

MVP 0.2 adds the three-tier memory system (Chapter 8.1) with the 12-table SQLite schema, git integration with pre-edit checkpoints and `/undo` via `git revert`, and the test-runner tool with convention-based file mapping[^3^]. The TUI gains the command palette (Ctrl+P), token/cost statistics, and the tool-call timeline screen.

v1.0 introduces the four subagent types from Chapter 9.1 — Planner, Implementer, Reviewer, and TestRunner — each running in isolated context with V4 Flash and returning 1–2K token summaries for 80–90% context savings versus full-history delegation[^4^]. MCP support enables integration with external tool servers. The permission system graduates from pattern-matching heuristics to an ML-based risk classifier following Claude Code's architecture[^4^].

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
| T1 | DeepSeek API breaking changes; legacy names deprecated July 2026[^1^] | 4 | Med | Dual endpoint support (OpenAI + Anthropic); endpoint switch is config-only |
| T2 | Model hallucination produces incorrect edits that pass tests | 4 | High | Exact-match SEARCH/REPLACE; diff review screen; test gate on every edit |
| T3 | V4 Flash inadequate for subagent reasoning (v1.0) | 3 | Med | Fallback to V4 Pro; human review of subagent outputs during training |
| T4 | TUI rendering bugs on Windows terminal emulators | 3 | Med | crossterm backend; CI on Windows runners; ConEmu/Windows Terminal priority |
| T5 | ratatui performance degrades at >1000 file tree nodes | 2 | Med | Virtualized List widget; incremental loading; benchmarked at 10K nodes |
| T6 | SQLite contention under concurrent subagent writes | 3 | Med | WAL mode; per-subagent in-memory buffers; single writer thread |
| T7 | Fuzzy match tier too permissive — accepts wrong block | 3 | Low | Uniqueness guard (second-best score ≥0.15 lower)[^5^]; mandatory user review |
| T8 | Token budget miscalculation causes context overflow | 3 | Med | 8K token safety buffer; compaction triggers at 80% capacity[^6^] |
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

**Incremental delivery validates assumptions early.** MVP 0.1 ships in 6–8 weeks. If SEARCH/REPLACE fails to achieve the 80% first-apply target on real codebases, the team can pivot to `str_replace` or full-file rewrite before building the rest of the system around a flawed primitive[^5^].

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

**ML permission classifier** (v1.0). Pattern-matching + heuristics achieve ≤10% false positives with zero training overhead[^4^]. The ML upgrade waits for accumulated labeled data.

**Vector database for memory** (indefinite). SQLite + BM25 search achieves required retrieval quality at lower complexity than vector embeddings[^3^]. Tantivy's vector search can be added later without schema changes.

#### 10.5.3 Final Recommendation

Start MVP 0.1 immediately. The core hypothesis — that a Rust-built, DeepSeek-native TUI agent can match Claude Code's core loop at 10–40x lower cost — is testable in 6–8 weeks. The components (DeepSeek client, SEARCH/REPLACE engine, ratatui TUI, ReAct loop) have established patterns from prior art; none require research.

Implementation order matters. Build the patch engine first (Tasks 7–8): it is the riskiest component because matching quality determines whether the agent can edit reliably. If the 4-tier matcher fails ≥80% first-apply on real commits, the project must pivot format before committing the architecture[^5^]. Build the TUI loop second (Task 3): it surfaces agent behavior and provides the diff review safety surface. The API client (Task 2) is third — straightforward, but must be validated against the live DeepSeek streaming endpoint.

Ship MVP 0.1 when it completes three of six benchmark tasks (fix bug, add feature, write tests) on small-to-medium projects under $0.08 median cost. Do not wait for git integration, memory, or subagents — those are MVP 0.2 expansions that improve efficiency but are not required to validate the core hypothesis.

The market gap identified in Chapter 2 remains open: no tool combines DeepSeek-native optimization, Rust performance, full TUI experience, and open-source distribution. Claude Code is vendor-locked and priced at $20–200/month. Aider lacks TUI polish. Goose is MCP-first, not TUI-first. The window is bounded by how quickly the ecosystem moves to support DeepSeek V4. A 6–8 week MVP delivery lands while DeepSeek's 75% launch discount on V4 Pro remains active through May 31, 2026[^1^], creating a compelling cost narrative for early adopters.

Build the core loop. Validate the patch engine. Ship.
