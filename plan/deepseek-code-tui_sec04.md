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

On first launch (or on detecting a new working directory), the scanner performs a recursive walk of the project tree using the `ignore` crate [^1^], which respects `.gitignore`, `.ignore`, and `.git/info/exclude` patterns. This single decision — using `ignore` over raw `walkdir` — eliminates the most common source of noise in codebase scanning: build artifacts, `node_modules`, and `.git` objects. For a typical Node.js project, this reduces the scanned file count from 50,000+ to 2,000–4,000 relevant source files.

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

Language detection uses a two-pass approach: first, filename/extension matching against the `tree-sitter` language registry (100+ languages); second, for ambiguous extensions (`.h` files that could be C, C++, or Objective-C), a content-sniffing pass examines the first 1,000 bytes for language-specific keywords. The `tree-sitter` crate's `Language` type is the canonical representation throughout the system, ensuring that downstream indexing, syntax highlighting, and search all agree on language classification [^2^].

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

The indexing system builds three complementary representations of the codebase, updated incrementally via `notify` file watchers [^3^]:

**Tree-sitter AST Index.** Every source file is parsed into its concrete syntax tree using the appropriate tree-sitter grammar. The indexer extracts symbols (functions, structs, classes, methods, traits, interfaces, enums, constants) and their relationships (inheritance, imports, calls). The tree-sitter query API enables pattern-based extraction without full compilation:

```rust
// Extract all function definitions across languages
let query = r#"
    (function_item name: (identifier) @func.name) ; Rust
    (function_declaration name: (identifier) @func.name) ; JS/TS
    (function_definition name: (identifier) @func.name) ; Python
"#;
```

The AST index is stored in SQLite via `sqlx` [^4^] with the schema:

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

**Symbol Graph.** Built from the AST index, the symbol graph tracks cross-reference relationships — which function calls which other function, which struct is referenced by which file. This graph enables the "repo-map" feature: when the user asks a question about a specific function, the system can include not just that function's definition, but all functions that call it and all functions it calls, ranked by relevance. The graph uses petgraph's `DiGraph` [^5^] with symbol IDs as node indices and weighted edges representing reference counts.

**Full-Text Search Index.** The `tantivy` crate [^6^] provides a Lucene-inspired search engine with sub-10ms query latency. Each source file becomes a tantivy document with three fields: `path` (stored, string), `content` (indexed, tokenized text), and `language` (stored, string). This enables fuzzy code search across the entire codebase — a capability that tree-sitter queries alone cannot provide, since they require knowing the symbol name in advance.

The three indices are maintained incrementally. On file change (detected via `notify`), the system: (1) re-parses the changed file with tree-sitter, (2) updates affected rows in the SQLite symbol table, (3) updates the cross-reference graph, and (4) re-indexes the file content in tantivy. The total incremental update latency for a single-file change is under 200ms on a mid-range laptop.

### 4.1.3 Project Instructions

The system implements a hierarchical instruction file system inspired by Claude Code's CLAUDE.md pattern [^7^] but adapted for DeepSeek Code's workflow. Four levels of instructions are discovered and merged at runtime, with higher-precedence files overriding lower-precedence ones:

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

Instructions are injected into the context window as user-level messages (probabilistic compliance, not system-prompt deterministic), following the pattern established by Claude Code's architecture research [^8^]. This placement means the model treats them as strong suggestions rather than hard rules, which produces more natural responses while still guiding behavior.

## 4.2 Chat + Coding Loop (Category B)

The coding loop is the core interaction model: the user describes a task in natural language, the agent plans, reads files, proposes edits, applies them, runs tests, and reports results. This section specifies the loop mechanics, not the TUI presentation (which is covered in Chapter 6).

### 4.2.1 Natural Language Task Processing

When a user submits a request, the system executes a plan-generate-act pipeline modeled on the ReAct pattern [^9^] but adapted for code editing:

1. **Task Classification.** The incoming request is classified into one of: `read_only` (question about code, no changes), `single_file_edit` (modify one file), `multi_file_refactor` (coordinated changes across files), `exploratory` (investigate and report), or `test_fix` (test is failing, fix it). Classification uses a lightweight heuristic (keyword matching + file reference counting) rather than a model call, to minimize latency for simple queries.

2. **Context Assembly.** For the classified task, the system assembles relevant context: file summaries from the repo-map for referenced files, the full content of files mentioned in the request, recent git diff (if any), and relevant symbol definitions. The total context is token-counted and, if it exceeds the budget (default: 60% of context window), trimmed using the symbol-graph ranking to keep the most relevant symbols.

3. **Plan Generation (for complex tasks).** Multi-file changes trigger a planning phase using DeepSeek V4 Pro. The plan specifies which files to modify, what changes to make in each, and what tests to run. The plan is presented to the user for approval before execution — this is a safety-critical step that prevents the model from making unwanted changes.

4. **Execution.** The plan is translated into a sequence of tool calls (read, edit, write, run_command, test). Each tool call passes through the permission system (Section 4.4) before execution.

### 4.2.2 Diff Review and Application

The agent uses SEARCH/REPLACE blocks as the primary edit primitive, based on converging industry evidence that this format achieves the best balance of LLM-friendliness and application reliability [^10^]. A SEARCH/REPLACE block specifies the exact text to find and the exact text to replace it with:

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

The patch engine implements 4-tier matching: exact match (character-for-character), whitespace-insensitive match (ignoring leading/trailing whitespace differences), indentation-preserving match (normalizing indentation levels), and fuzzy match (using similar's text diff algorithms [^11^]). This fallback chain means the patch succeeds even when the model produces slightly inaccurate whitespace or when the file has changed subtly since the model read it.

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

The main screen uses a constraint-based layout system via `ratatui` [^12^]. The layout is responsive: on screens wider than 120 columns, it shows five panels; on narrower screens, panels collapse or become tab-switchable.

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

The permission system is the most critical safety component. Analysis of Claude Code's source code reveals that users approve approximately 93% of permission prompts, making interactive confirmation behaviorally unreliable as a sole safety mechanism [^13^]. The system must maintain safety independently of human vigilance.

Five permission modes are implemented, ordered from most restrictive to least:

| Mode | Description | Auto-approves | Still asks |
|------|-------------|--------------|------------|
| `read_only` | Agent can only read files and run safe commands | Nothing | All write operations |
| `ask` | Standard interactive mode (default) | Read, Grep, Glob | Edit, Write, Bash |
| `accept_edits` | File edits auto-approved; destructive commands still ask | Read, Edit, Write | Bash(rm\*), Bash(git push\*), Bash(sudo\*) |
| `auto` | Heuristic-based: command risk scorer decides | Low-risk operations | Medium/high-risk operations |
| `yolo` | Minimal prompting; deny rules still enforced | Most operations matching allowlist | Anything matching denylist |

The rule evaluation engine uses deny-first matching: deny rules are checked first, then ask rules, then allow rules, then the mode default. A deny rule always takes precedence over an allow rule, even when the allow is more specific [^14^]. Rules use tool-pattern syntax:

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

**Session Memory** (conversation transcript). Stored as append-only JSONL in SQLite. Each entry is a message: user message, assistant message (text + tool_use blocks), tool_result, or system event (permission decision, compaction boundary). The append-only design makes sessions resumable by design — the full transcript can be replayed from any point [^15^].

**Project Memory** (cross-session knowledge). Stored in `.deepseek-code/memory/` as markdown files. Three sub-types: `conventions.md` (coding standards for this project), `learnings.md` (decisions and their outcomes), and `relationships.md` (files that commonly change together). Project memory is human-editable and committed to git, making it inspectable and version-controlled.

**User Preferences** (persistent settings). Stored in `~/.config/deepseek-code/config.toml` using the `toml` crate [^16^]. Includes default model, permission mode, keybindings, theme, API key paths, and custom endpoints. Preferences are loaded at startup and can be overridden per-session via CLI flags or the TUI settings panel.

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

These logs enable the auto-memory feature: at session end, the system summarizes the session's decisions and appends relevant learnings to `conventions.md` or `learnings.md`. The summarization is performed by DeepSeek V4 Flash, keeping the cost negligible ($0.14/M input tokens) [^17^].

### 4.4.3 Tool System

The tool system implements 15+ built-in tools, each exposed to the model via OpenAI-compatible function calling schema. DeepSeek V4 supports up to 128 functions per call [^18^], leaving ample headroom for custom tools and MCP extensions.

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

Tool execution follows a concurrency model derived from Claude Code's `partitionToolCalls()` [^19^]: read-only tools (Read, Grep, Glob, FindSymbol) execute in parallel, while state-mutating tools (Write, Edit, Bash, GitCommit) execute serially in declaration order. This parallelization reduces latency for the common pattern of "read three files, then edit one."

Streaming tool calls are implemented via `reqwest-eventsource` [^20^]: as the model streams its response, tool calls are parsed incrementally. When a tool_use block is fully parsed (name and arguments complete), execution begins immediately, even if the response is still streaming. This can reduce end-to-end latency by 30–50% for multi-tool responses.

MCP (Model Context Protocol) integration enables extending the tool set without modifying core code. The system reads `~/.config/deepseek-code/mcp.json` to discover MCP servers, registers their tools as `McpTool` invocations with the appropriate server routing, and handles authentication per the MCP specification.

### 4.4.4 Subagent Orchestration

Subagents provide context isolation: instead of polluting the main agent's context window with the full transcript of a subtask, a subagent works independently and returns only a condensed summary (typically 1,000–2,000 tokens instead of 10,000+). This yields 80–90% context savings per delegation [^21^].

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

Subagents are created via a lightweight YAML frontmatter in instruction files, following the pattern established by Claude Code's `Task` tool [^22^]:

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

The model selection strategy uses V4 Flash for subagents ($0.14/M input, high speed) and V4 Pro ($1.74/M input, maximum reasoning) for the main agent. This 12.4x cost differential means a typical session with 10 subagent delegations costs approximately the same as extending the main agent's context by 80,000 tokens — but with better isolation and cleaner context [^23^].

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
| **TUI Framework Maturity** | ratatui 0.30 (19.1k stars, IDE-capable layouts, immediate-mode) [^24^] | Bubble Tea (40.7k stars, Elm/MVU, simpler layouts) [^25^] | Ink (35.6k stars, React-based, limited IDE layouts) [^26^] | Textual (34.9k stars, async widgets, high memory) |
| **Memory Efficiency** | 30–40% less than Go, no GC pauses [^27^] | Good (GC, occasional pauses) | Poor (V8 heap, 200MB+ baseline) | Poor (highest overhead) |
| **Binary Distribution** | Single static binary, <15MB stripped | Single binary, ~10MB but requires libc | Requires Node.js runtime | Requires interpreter + deps |
| **Async + Streaming** | Native async/await + tokio, type-safe SSE | Goroutines + channels, simpler but less structured | Callbacks/Promises, native EventSource | asyncio, less mature ecosystem |
| **Code Parsing Ecosystem** | tree-sitter (100+ langs) + tantivy (sub-10ms search) [^28^] | tree-sitter Go bindings, no tantivy equivalent | Direct TS compiler API, good parsing | Excellent (ast, pylint, mypy) |
| **Git Integration** | git2 (mature, C binding) or gix (pure Rust, 2–10x faster) [^29^] | go-git (pure Go, good) | simple-git (wrapper around CLI) | GitPython (wrapper) |
| **Developer Velocity** | Slower (borrow checker, explicit types) | Fast (simple, garbage-collected) | Fast (familiar, large ecosystem) | Fastest (dynamic, REPL) |
| **Type Safety** | Compile-time guaranteed, zero-cost | Runtime + generics (limited) | Erasable (JavaScript at runtime) | Dynamic (runtime errors) |

**The verdict:** Rust wins on the dimensions that matter most for this product — TUI capability, memory efficiency, code parsing/search, and binary distribution — while its developer velocity disadvantage is mitigated by the maturity of the crate ecosystem and the fact that coding agents are long-lived infrastructure projects where compile-time safety pays dividends. Go's Bubble Tea is appealing for simpler TUIs but lacks the layout sophistication needed for an IDE-like multi-panel interface. TypeScript's Ink is limited to React-style component trees that struggle with the irregular panel geometries of a coding IDE. Python's high memory footprint makes it unsuitable for a tool intended to run alongside an editor for hours.

The combination of ratatui (constraint-based layouts) + tokio (async streaming) + tree-sitter (code parsing) + tantivy (code search) + git2/gix (native git) exists only in the Rust ecosystem. No other language provides all five of these capabilities at production maturity [^30^].

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

The TUI uses a merged event stream pattern [^31^] that combines multiple async event sources into a single `select!`-driven loop. This is the canonical architecture for tokio-based TUIs and eliminates polling overhead:

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
