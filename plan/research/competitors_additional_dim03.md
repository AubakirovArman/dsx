# Additional AI Coding Tools - Deep Competitive Analysis (Dimension 3)

> Research Date: July 2026
> Tools Analyzed: Roo Code, Crush, Goose, Zed AI, Kiro, Continue.dev
> Previously Covered: Claude Code, Codex CLI, Gemini CLI, OpenCode, Aider

---

## Table of Contents

1. [Roo Code](#1-roo-code)
2. [Crush](#2-crush-by-charmbracelet)
3. [Goose](#3-goose-by-blocklinux-foundation)
4. [Zed AI](#4-zed-ai)
5. [Kiro](#5-kiro-by-aws)
6. [Continue.dev](#6-continuedev)
7. [Comparative Summary](#7-comparative-summary)
8. [Key Takeaways for Our Tool](#8-key-takeaways-for-our-tool)

---

## 1. Roo Code

### Overview

Roo Code is a VS Code extension that provides autonomous AI coding capabilities. It started as "Roo Cline" -- a fork of the Cline project -- but has since evolved significantly with a rebranding to "Roo Code" reflecting a broader vision. The project hit 3 million installs before the original team announced shutdown in April 2026 to focus on "Roomote," with Kilo Code emerging as the primary fork/community successor.

**Key Identity:** VS Code extension (not CLI), originally forked from Cline, now evolved into its own ecosystem with a rich mode system.

### Architecture

- **Type:** IDE Extension (VS Code only)
- **Base:** Originally forked from Cline; moved away from LangChain dependencies
- **Distribution:** VS Code Marketplace (`RooVeterinaryInc.roo-cline`)
- **Successor:** Kilo Code (`kilocode.Kilo-Code`) -- rebuilt on OpenCode server, shares sessions between CLI and VS Code
- **Relationship to Cline:** Shared DNA but diverged significantly. Cline emphasizes simplicity and human-in-the-loop partnership; Roo Code pushes for control and flexibility with its mode system.

### Installation

```bash
# VS Code Extension Marketplace
# Search for "Roo Code" in VS Code extensions panel
# Or install via CLI:
code --install-extension RooVeterinaryInc.roo-cline

# Migration to Kilo Code (successor):
code --install-extension kilocode.Kilo-Code
```

### Key Features

1. **Mode System** -- Roo Code's signature feature:
   - **Code Mode:** Everyday coding, edits, file operations
   - **Architect Mode:** Plan systems, specs, migrations (planning without execution)
   - **Ask Mode:** Fast answers, explanations, docs (read-only, no code changes)
   - **Debug Mode:** Trace issues, add logs, isolate root causes
   - **Custom Modes:** Build specialized modes for team workflows (e.g., Code Reviewer)
   - **Orchestrator Mode** (deprecated in Kilo successor -- full-tool agents can delegate natively)

2. **MCP Server Integration** -- Extensive MCP support:
   - Global configuration via `mcp_settings.json`
   - Project-level configuration via `.roo/mcp.json`
   - Supports stdio, HTTP, and SSE transports
   - Auto-detection of project-level MCP configs
   - Tool-level permission controls (`alwaysAllow` for specific tools)

3. **Boomerang Tasks** -- For complex workflows that need to break out of normal flow

4. **Context Condensing** -- Smart context window management

5. **Diff Mode Options** -- Experimental unified diff algorithms for file editing

6. **Checkpoints** -- Snapshot system for reverting changes (stored in-repo `.git` in Roo; moved to `~/.local/share/kilo/snapshot/` in Kilo)

7. **Codebase Indexing** -- Qdrant + embeddings pipeline for semantic search (ported to community PR in Kilo)

8. **Multi-Language Support** -- 18+ languages including English, Chinese, Japanese, Korean, etc.

### File Editing Approach

- **Primary:** Diff-based edits (not full rewrites by default)
- **Mode-dependent:** Code Mode performs edits; Architect/Ask modes are read-only
- **Options:** Supports diff mode configuration with unified diff algorithms
- **Checkpoint-based:** Snapshots before changes for rollback

### Command Execution Safety

- **Tool Groups:** Permission system organized by `read`, `edit`, `browser`, `command`, `mcp` groups
- **Successor (Kilo):** Explicit `allow`/`ask`/`deny` rules with glob patterns
- **Per-Mode Configuration:** Different modes can have different tool access levels

### Permission Model

- **VS Code-native:** Uses VS Code's extension permission system
- **Tool-Level Granularity:** Individual tools can be allowed/ask/denied
- **MCP Tool Permissions:** `alwaysAllow` array for auto-approving specific MCP tools
- **Checkpoints:** Automatic snapshots for rollback capability

### Session Management

- **VS Code Panel-based:** Sessions persist within VS Code workspace
- **Checkpoints:** Take snapshots at configurable intervals (every agent action in Roo; every user message in Kilo)
- **Session Portability (Kilo):** Sessions move between CLI and VS Code because both hit the same OpenCode server engine

### Memory/Context System

- **Memory Bank** (Roo): Stored in `.kilocode/rules/memory-bank/` -- write-protected, agent can't modify without approval
- **AGENTS.md** (Kilo successor): Open standard that Cursor and Windsurf also read; sub-directory `AGENTS.md` files supported with precedence for files under that directory
- **Project Rules:** `.roorules` files (Roo) -> `.kilo/rules/*.md` (Kilo)
- **Global Rules:** Per-mode configuration profiles with sticky models
- **Context Providers:** `@file`, `@terminal` context mentions

### Git Integration

- **Checkpoints:** Git-based snapshots stored in worktrees
- **Timeline View:** Visual diff/changelog tracking
- **Auto-commit options** (configurable)

### Subagents/MCP Support

- **MCP Servers:** Full support (stdio, HTTP, SSE transports)
- **Subagents:** Orchestrator mode (deprecated) delegated tasks to specialist modes
- **Kilo Successor:** Full-tool agents can spawn parallel subagents; Agent Manager runs each chat in its own git worktree
- **Parallel Execution:** Tool calls (file reads, greps, shell commands) run concurrently in Kilo

### Model Support

| Provider | Status |
|----------|--------|
| **DeepSeek** | **Yes** -- `deepseek-chat` and `deepseek-reasoner` via DeepSeek API; 128k context window |
| Anthropic (Claude) | Full support (Sonnet, Opus, Haiku) |
| OpenAI (GPT) | Full support (GPT-4, GPT-5 family) |
| Google Gemini | Full support (2.5 Pro with thinking budget) |
| OpenRouter | Yes |
| AWS Bedrock | Yes (Claude models with 1M context) |
| Local (Ollama) | Yes |
| **25+ Providers** | Via OpenRouter, LiteLLM, and custom configurations |

- **Per-Mode Model Selection:** Each mode can be pinned to a different model ("sticky models")
- **Auto Models:** Automatically selects best model at a given price point (Kilo feature)

### Strengths (What to Borrow)

1. **Mode System** -- Separation of concerns between planning (Architect), execution (Code), exploration (Ask), and debugging (Debug) is excellent. Reduces token waste by preventing the AI from jumping to solutions.
2. **Per-Mode Model Selection** -- Using different models for different modes (e.g., o1 for Architect, Sonnet for Code) optimizes cost and quality.
3. **MCP Integration Depth** -- Project-level + global MCP config with tool-level permissions is well-designed.
4. **Custom Modes** -- Community mode gallery enables sharing of specialized configurations.
5. **Context Condensing** -- Smart handling of context windows with automatic truncation on errors.

### Weaknesses (What to Improve)

1. **VS Code Lock-in** -- Not available as CLI or for other editors. The Kilo successor addresses this with shared CLI/VS Code engine.
2. **Cline Legacy Baggage** -- Originally a fork, some architectural decisions inherited from Cline limit flexibility.
3. **Checkpoints in Repo** -- Original Roo stored checkpoints in-repo causing the "nested-.git bug." Kilo moved them out but migration is required.
4. **Shutdown/Churn** -- Original project shutdown creates uncertainty. Kilo Code is the migration path but not seamless.
5. **Token Usage** -- Code Mode tends to rewrite entire files even for small changes unless carefully configured.

---

## 2. Crush (by Charmbracelet)

### Overview

Crush is a TUI (Terminal User Interface) AI coding agent from Charmbracelet -- the team behind Bubble Tea, Lip Gloss, Gum, and other popular terminal tools. It's essentially a fork/evolution of Open Code, with the original creator joining Charm. Built in Go, it emphasizes beautiful terminal UI and deep integration with the developer's existing terminal workflow. Over 10k GitHub stars, 35+ contributors.

**Key Identity:** TUI-first (not CLI), Go-based, multi-model with LSP integration, session-based project management.

### Architecture

- **Type:** TUI (Terminal User Interface) -- full interactive interface in terminal
- **Framework:** Built on Charmbracelet's Bubble Tea (TUI framework) + Lip Gloss (styling)
- **Language:** Go (compiled, fast startup)
- **Relationship to OpenCode:** Crush evolved from Open Code; the original creator joined Charmbracelet
- **Distribution:** Multiple package managers (Homebrew, NPM, Nix, Scoop, APT, YUM, Go install)

### Installation

```bash
# Homebrew (Recommended for macOS)
brew install charmbracelet/tap/crush

# NPM (Cross-Platform)
npm install -g @charmland/crush

# Arch Linux
yay -S crush-bin

# Nix
nix run github:numtide/nix-ai-tools#crush

# Windows
winget install charmbracelet.crush
scoop install charmbracelet/crush

# Go
go install github.com/charmbracelet/crush@latest

# FreeBSD
pkg install crush
```

### Key Features

1. **Beautiful TUI** -- Leverages Charmbracelet's Bubble Tea + Lip Gloss for an aesthetic terminal experience:
   - Separate diff window
   - Good information context display
   - Project-level session management
   - Keyboard shortcuts: `Ctrl+P` (commands), `Ctrl+G` (chat focus), `Ctrl+S` (sessions), `Ctrl+F` (file attachments)

2. **Multi-Model with Mid-Session Switching** -- Unique capability to switch LLMs mid-session while preserving context:
   - Start with GPT-5 for architecture, switch to Claude for implementation, then local model for review
   - Supports OpenAI, Anthropic APIs, custom OpenAI/Anthropic-compatible APIs
   - Includes Z.AI's GLM models (glm-4.7, glm-4.5-air)

3. **LSP Integration** -- Uses Language Server Protocol for code intelligence:
   - Diagnostics
   - References lookup
   - Symbol definitions
   - Real-time code intelligence from actual project files

4. **MCP Extensibility** -- Model Context Protocol support:
   - HTTP, stdio, SSE transports
   - Configured via `crush.json`
   - Project-level `.crushignore` for file access control

5. **Session Management** -- Per-project sessions with isolated contexts:
   - Multiple simultaneous sessions per project
   - Session context preserved when switching directories
   - Auto-loads appropriate session state

6. **Skills System** -- YAML-based skills that can be made user-invocable:
   - `user:` prefix for global skills
   - `project:` prefix for project-level skills
   - Invoked from command palette (`Ctrl+P`)

7. **CRUSH.md / AGENTS.md** -- Context files for project-specific guidance

### File Editing Approach

- **Tools Available:** `glob`, `grep`, `ls`, `view` (read), `write` (create), `edit` (modify), `patch` (apply patches), `multiedit` (multi-file)
- **LSP-Aware:** Uses LSP for understanding code structure before editing
- **Diff Preview:** Separate diff window in TUI for reviewing changes before applying

### Command Execution Safety

- **YOLO Mode** (`--yolo` flag): Bypasses all confirmation prompts
- **Tool Disablement:** Fine-grained tool-level permission control via `disabled_tools` in config
- **Disablable Tools:** `agent`, `bash`, `job_output`, `job_kill`, `download`, `edit`, `multiedit`, `lsp_diagnostics`, `lsp_references`, `lsp_restart`, `fetch`, `agentic_fetch`, `glob`, `grep`, `ls`, `sourcegraph`, `todos`, `view`, `write`, `list_mcp_resources`, `read_mcp_resource`
- **`.crushignore`:** Specify files/directories AI cannot access

### Permission Model

- **Three Modes:**
  1. Standard Mode -- Interactive with permission prompts
  2. YOLO Mode (`--yolo`) -- Bypasses all confirmation (for throwaway environments)
  3. Non-Interactive -- Single-prompt execution for scripting

- **Config-based:** Permissions managed through `crush.json` (trusted code -- any `$(...)` runs at load time)
- **Tool-Level:** Individual tools can be disabled entirely

### Session Management

- **Project-Scoped:** Maintains independent AI sessions per project
- **Directory-Aware:** Auto-loads session when switching directories
- **Multiple Sessions:** Multiple simultaneous work sessions per project
- **Auto-Compaction:** Yes, context window management
- **Logging:** `.crush/logs/crush.log` for session logging

### Memory/Context System

- **CRUSH.md / AGENTS.md:** Project-level context files
- **Session Persistence:** Context preserved across session restarts
- **Skills:** YAML-frontmatter skill files that add reusable capabilities
- **No Semantic Search:** Unlike some competitors, no vector embeddings for code search

### Git Integration

- Direct access to git commands as tools
- No explicit git checkpoint system (unlike Roo/Cline)
- Relies on user's existing git workflow

### Subagents/MCP Support

- **MCP:** Full support (HTTP, stdio, SSE)
- **Subagents:** `agent` tool for launching sub-agents
- **Configuration:** Via `crush.json` with `$schema` validation
- **Security Note:** `crush.json` is trusted code -- shell commands in config execute before UI appears

### Model Support

| Provider | Status |
|----------|--------|
| **DeepSeek** | **Yes** -- Via OpenRouter and OpenAI-compatible APIs; DeepSeek format supported in edit predictions |
| OpenAI (GPT) | Full support |
| Anthropic (Claude) | Full support |
| Google Gemini | Via OpenRouter |
| Z.AI (GLM) | Native support (glm-4.7, glm-4.5-air) |
| OpenRouter | Yes -- free models like Qwen 3 Coder |
| Local (Ollama) | Yes |
| **Custom APIs** | Any OpenAI or Anthropic-compatible API |

### Strengths (What to Borrow)

1. **TUI Experience** -- Bubble Tea + Lip Gloss produces genuinely beautiful terminal UI. The separate diff window and information context are polished.
2. **Mid-Session Model Switching** -- Unique ability to change LLMs while preserving context is powerful for cost/quality optimization.
3. **LSP Integration** -- Real code intelligence from LSPs, not just AI reasoning. Makes the agent understand actual code structure.
4. **Go Performance** -- Compiled binary, fast startup, responsive UI.
5. **Cross-Platform** -- Excellent platform coverage: macOS, Linux, Windows (PowerShell + WSL), FreeBSD, OpenBSD, NetBSD.
6. **Package Manager Ubiquity** -- Available on virtually every package manager.

### Weaknesses (What to Improve)

1. **Planning Capabilities** -- HN reports indicate "really bad planning capabilities as agent. Acts awkwardly, executes single commands instead of batch commands." This makes it slow.
2. **Token Inefficiency** -- Uses more tokens for operations than comparable tools (per HN comparison).
3. **No SSO/API Key Required** -- Unlike Claude Code, no subscription integration. Must generate API keys manually.
4. **Beta Quality** -- Relatively new, described as "much more a beta" compared to alternatives.
5. **No Semantic Search** -- No vector embeddings or semantic code search; relies on grep/glob + LSP.
6. **No Checkpoint System** -- No built-in snapshot/rollback mechanism.
7. **Security Model** -- `crush.json` executing shell commands at load time is a potential vulnerability surface.

---

## 3. Goose (by Block / Linux Foundation)

### Overview

Goose is an open-source, local-first AI agent framework released by Block (the company behind Square, Cash App) in January 2025. In December 2025, Block contributed Goose to the Linux Foundation's Agentic AI Foundation (AAIF), alongside Anthropic's MCP and OpenAI's AGENTS.md. It now operates under neutral, community-driven governance with backing from AWS, Anthropic, Google, Microsoft, and OpenAI. 30,000+ GitHub stars, 350+ contributors, 110+ releases.

**Key Identity:** MCP-first architecture, model-agnostic (25+ providers), local-first, recipe-based workflows, both CLI and Desktop.

### Architecture

- **Type:** CLI + Desktop Application (dual interface)
- **Foundation:** MCP-first -- the entire architecture is built around Model Context Protocol
- **Governance:** Apache 2.0, Linux Foundation Agentic AI Foundation (AAIF)
- **Local-First:** Runs entirely locally; all data stays on machine
- **ACP Support:** Ships as native Agent Client Protocol (ACP) agent -- can be driven from Zed, JetBrains, etc.

### Installation

```bash
# Linux
curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash

# macOS (Homebrew)
brew install block-goose-cli

# Desktop: Download from GitHub releases
# Updates via: goose update
```

### Key Features

1. **MCP-First Architecture** -- Everything is an MCP extension:
   - **Built-in Developer Extension:** `read_file`, `write_file`, `patch_file`, `execute_command`, `list_files`
   - **Built-in Memory Extension:** Persistent memory across sessions ("Remember that I prefer TypeScript")
   - **Built-in Computer Controller:** Web scraping, PDF/DOCX reading, Excel processing, automated workflows
   - **3,000+ MCP Servers** available in ecosystem

2. **Recipes** -- YAML-based reusable AI workflows (Goose's killer feature):
   - Version-controlled workflow automation
   - Parameterizable with variable substitution
   - Composable (recipes can call other recipes)
   - Can be made into slash commands (e.g., `/weekly-status`)
   - Structured JSON output for automation pipelines
   - **Philosophy:** "Rules change how the agent behaves. Recipes change what the agent does."
   - Deeplink support: `goose://recipe?config=...`

3. **25+ LLM Providers** -- Model-agnostic by design:
   - Tetrate Agent Router (built-in with $10 free credits)
   - OpenRouter (200+ models)
   - ChatGPT subscription (direct login)
   - Google Gemini, Groq (free tiers available)
   - Ollama for local models (fully offline)
   - AWS Bedrock, Azure, GCP Vertex, and more

4. **Desktop + CLI** -- Both interfaces share the same backend:
   - Desktop: GUI with theme/font customization
   - CLI: Terminal REPL for scripting and automation
   - Sessions are single, continuous conversations

5. **Visual Workflow Builder** (planned): Canvas-based drag-and-drop recipe editor

### File Editing Approach

- **Developer Extension Tools:**
  - `read_file` -- Read file contents
  - `write_file` -- Write or update files
  - `patch_file` -- Apply targeted edits (diff/patch based)
- **File Extension Memory:** Remembers file patterns and preferences

### Command Execution Safety

- **Extension-Level Permissions:** Each extension has configurable:
  - Enable/Disable toggle
  - Timeout limits
  - File access permissions
  - Tool permissions
  - Custom environment variables
- **Auto-destructive confirmation:** "Confirm deletions" setting
- **Behavior Controls:** Auto-scroll, show tool output, notification sounds

### Permission Model

- **Extension-Based:** Permissions managed per extension, not per tool
- **Settings:** Temperature, max tokens, thinking mode configurable per provider
- **No YOLO Mode:** More conservative than some alternatives -- designed for safety

### Session Management

- **Session-Based:** Single, continuous conversations between user and Goose
- **Memory Extension:** Cross-session persistence for user preferences and facts
- **No Checkpoints:** No built-in snapshot/rollback system
- **History:** Session history available in Desktop UI

### Memory/Context System

- **Memory Extension:** Explicit `remember`/`recall` tools for persistent facts
- **Recipes:** Reusable workflow definitions capture institutional knowledge
- **No Semantic Codebase Search:** Relies on MCP servers for external context

### Git Integration

- Via MCP servers (GitHub MCP, Git CLI tools)
- No built-in git checkpoint system
- Recipes can include git operations as workflow steps

### Subagents/MCP Support

- **MCP-First Design:** The entire architecture IS MCP support:
  - `stdio` transport for local tools
  - `http` transport for remote APIs
  - `sse` transport for streaming
- **Extension Marketplace:** 3,000+ MCP servers available
- **Custom Extensions:** Build and register custom MCP servers
- **No Native Subagents:** Recipes provide workflow composition instead

### Model Support

| Provider | Status |
|----------|--------|
| **DeepSeek** | **Yes** -- Via OpenRouter and custom provider configs |
| Anthropic (Claude) | Full support |
| OpenAI (GPT/Codex) | Full support + ChatGPT subscription login |
| Google Gemini | Yes (free tier available) |
| Groq | Yes (fast inference) |
| **25+ Providers** | Via Tetrate, OpenRouter, direct API |
| Local (Ollama) | Full support -- fully offline possible |

- **Temperature Control:** 0.0-1.0 per provider
- **Thinking Mode:** Configurable for Claude models
- **Free Usage Possible:** Google Gemini + Groq free tiers + Ollama local = zero cost

### Strengths (What to Borrow)

1. **MCP-First Architecture** -- The most thorough MCP integration in the ecosystem. Everything is an MCP extension, making it infinitely extensible.
2. **Recipes** -- YAML-based reusable workflows are a genuinely different approach. They provide version control, composition, parameterization, and team sharing that no other tool matches.
3. **Model Agnosticism** -- 25+ providers with easy switching means zero vendor lock-in.
4. **Linux Foundation Governance** -- AAIF ensures neutral, community-driven development with major industry backing.
5. **Local-First** -- Everything runs on-device. Full offline capability with Ollama.
6. **ACP Native** -- Works as an agent in Zed, JetBrains, and any ACP-compatible editor.
7. **Free** -- Truly free (Apache 2.0) with free-tier providers available.

### Weaknesses (What to Improve)

1. **No Semantic Code Search** -- Unlike Cursor or Kilo, no built-in vector embeddings for codebase understanding.
2. **Recipe Authoring is Manual** -- YAML editing required; visual builder still planned.
3. **No Checkpoints/Snapshots** -- No built-in rollback mechanism for file changes.
4. **Less Polished UX** -- Compared to Claude Code or Cursor, the experience requires more setup.
5. **Desktop App Limitations** -- Tauri-based desktop may have performance constraints vs native.
6. **No Planning Mode** -- Recipes are execution-focused; no built-in spec-driven planning workflow.

---

## 4. Zed AI

### Overview

Zed is a next-generation code editor built from scratch in Rust with a GPU-driven UI framework (GPUI). It's GPL-licensed (editor) + Apache 2 (UI framework). The AI layer is deeply integrated, featuring both a built-in agent panel and the Agent Client Protocol (ACP) for external agents. Zed AI is notable for being fully open-source, including the editor, the Zeta edit prediction model, and the ACP protocol.

**Key Identity:** Native IDE (not extension), Rust-based (120fps GPU rendering), open-source with built-in AI agent + ACP for external agents, Zeta edit prediction model.

### Architecture

- **Type:** Native IDE (built from scratch, not a VS Code fork)
- **Language:** Rust (entire codebase)
- **UI Framework:** GPUI -- GPU-driven, 120fps rendering
- **License:** Editor GPL, GPUI Apache 2.0
- **Platforms:** macOS, Linux (Windows in private beta as of 2025)
- **Two AI Systems:**
  1. Built-in Zed Agent (native tools + MCP)
  2. External Agent support via ACP (Claude Agent, Codex, Gemini CLI, etc.)

### Installation

```bash
# macOS
# Download from zed.dev or use Homebrew:
brew install --cask zed

# Linux
# Download from zed.dev -- AppImage and DEB packages available

# Windows
# Private beta, apply through zed.dev
```

### Key Features

1. **Agent Panel** -- Built-in AI assistant:
   - Read and modify code
   - Interact with project files
   - Multi-file editing with diff review
   - Natural language task execution
   - Fine-grained tool permissions

2. **Zeta Edit Prediction** -- Open-weight model for "predict next edit":
   - **Zeta2:** Purpose-built for Zed, trained on real open-source edits
   - Predicts multi-line changes, not just next token
   - Tab to accept, multiple follow-up edits by repeated tab
   - Uses LSP for type/symbol context understanding
   - Open weights on Hugging Face
   - **Free tier:** 2,000 predictions/month

3. **Agent Client Protocol (ACP)** -- Open standard for agent interoperability:
   - "LSP for AI agents" -- JSON-RPC over stdio
   - Connect Claude Agent, Codex, Gemini CLI to Zed
   - Session management, tool calls, file access, planning
   - Privacy: nothing touches Zed's servers when using external agents

4. **Multi-Provider Support** -- Flexibility in AI models:
   - Hosted by Zed: Claude Opus, GPT-5.4
   - Run locally: Ollama integration
   - Bring your own keys: Any provider
   - Per-buffer model selection: Different models for different files

5. **Inline Assistant** -- Highlight code, `Alt-A`, refactor in place

6. **Git Integration** -- Built-in:
   - Stage files
   - AI-written commit messages
   - Push to remote
   - Multiplayer editing support

7. **Built-in Debugger** -- DAP-based debugger for Rust, Go, Python, C/C++, JavaScript

### File Editing Approach

- **Built-in Agent Tools:**
  - File reading, searching, editing
  - Diff-aware editing
  - Multi-file operations with review
- **Edit Prediction:** Zeta predicts edits at keystroke granularity
- **External Agents (ACP):** Full tool access including edit, review, TODO lists

### Command Execution Safety

- **Tool Permissions System** -- Granular permission configuration:
  - Per-tool: auto-approve, auto-deny, or require confirmation
  - Configurable in settings
  - CVE-2025-55012 (Aug 2025) revealed permission bypass vulnerability (patched in v0.197.3)

### Permission Model

- **Three-Level Tool Permissions:**
  1. Automatically approved
  2. Automatically denied
  3. Require confirmation (case-by-case)
- **File Globs:** Permission patterns support glob matching
- **External Agents:** Privacy -- Zed never stores or trains on code for external agents

### Session Management

- **Thread-Based:** Agent Panel uses threads (sessions)
- **Multiple Agent Sessions:** Multiple sessions in same workspace
- **Undo:** Reverts changes up to last edit file tool call
- **Session History:** Available for built-in agent

### Memory/Context System

- **ACP Standardized:** Sessions, context, tool calls managed through ACP protocol
- **No Explicit Memory Tool:** Unlike Goose, no persistent memory extension
- **Context via @-mentions:** File, URL, code references
- **External Agent Context:** Depends on agent (Claude Agent has its own, etc.)

### Git Integration

- **Native:** Built into editor (not via MCP)
   - Stage, commit with AI message, push
   - Multiplayer real-time collaborative editing
- **GitHub Integration:** Via MCP or Copilot

### Subagents/MCP Support

- **MCP Servers:** Supported for adding custom tools to built-in agent
- **ACP for External Agents:** Claude Agent, Codex, Gemini CLI, GitHub Copilot via ACP
- **No Native Subagents:** External agents provide their own orchestration

### Model Support

| Provider | Status |
|----------|--------|
| **DeepSeek** | **Yes** -- Via Ollama (local), OpenAI-compatible servers; `deepseek_coder` prompt format supported in edit predictions |
| Anthropic (Claude) | Full support (Sonnet, Opus); Claude Agent via ACP |
| OpenAI (GPT) | Full support; Codex via ACP |
| Google Gemini | Full support; Gemini CLI via ACP |
| Zed's Zeta | Native (open-weight, free tier) |
| Mercury Coder | Supported (diffusion architecture) |
| Copilot NES | Supported |
| Local (Ollama) | Full support |

- **Per-Buffer Model Selection:** Choose different models for different files
- **Edit Prediction Providers:** Zeta (default), Mercury Coder, Sweep, Ollama, Copilot, Codestral

### Strengths (What to Borrow)

1. **Native Speed** -- Rust + GPU rendering at 120fps. When agents edit 50 files, "you see every change as it happens." No electron sluggishness.
2. **ACP Protocol** -- The "LSP for agents" is a genuinely important standard. Enables any agent to work in any editor. Smart strategic move.
3. **Zeta Edit Prediction** -- Open-weight model trained specifically for edit prediction, not completion. Multi-line changes with single tab. Purpose-built for real developer workflows.
4. **Open Source Everything** -- Editor, model, and protocol all open source. No vendor lock-in at any layer.
5. **Per-Buffer Model Selection** -- Granular model choice at file level, not just project level.
6. **Multiplayer Editing** -- Real-time collaborative editing built-in (differentiator from VS Code-based tools).

### Weaknesses (What to Improve)

1. **CVE History** -- CVE-2025-55012 (permission bypass for arbitrary code execution, CVSS 8.5) shows security model has had real flaws.
2. **Platform Availability** -- Windows was in private beta for a long time (planned stable release "later in 2025").
3. **Smaller Ecosystem** -- VS Code extension marketplace is massive; Zed's ecosystem is growing but smaller.
4. **New IDE Switching Cost** -- Unlike extensions, requires learning a new editor entirely.
5. **External Agent Limitations** -- Built-in agent has fewer features than dedicated tools like Claude Code; relies on ACP external agents for advanced capabilities.
6. **Edit Prediction Costs** -- Free tier limited to 2,000 predictions/month; Pro plan required for unlimited.

---

## 5. Kiro (by AWS)

### Overview

Kiro is AWS's agentic IDE built on Code OSS (the open-source base of VS Code). It represents a fundamentally different approach to AI coding: spec-driven development. Rather than jumping straight to code generation, Kiro requires structured planning before coding. It's designed to bridge the gap between "vibe coding" (rapid prototyping) and production software engineering.

**Key Identity:** Full IDE (Code OSS fork), spec-driven development, AWS/Bedrock-backed, Claude Sonnet models, hooks automation, planning-first philosophy.

### Architecture

- **Type:** Full IDE (Code OSS fork -- NOT a VS Code extension)
- **Base:** VS Code-compatible (uses Open VSX plugins, imports VS Code settings)
- **Backend:** Amazon Bedrock infrastructure
- **Models:** Claude Sonnet 4.0 (primary), Claude 3.7 (fallback)
- **Creator:** AWS Agentic AI Developer group
- **Governance:** AWS (proprietary, not open source)

### Installation

```bash
# Download from kiro.dev
# Available for macOS, Windows, Linux
# Currently in preview (free during preview)
# Supports all major platforms and programming languages
```

### Key Features

1. **Specs (Spec-Driven Development)** -- The core differentiator:
   - Converts natural language prompts into detailed requirements and system designs
   - Uses **EARS notation** (Easy Approach to Requirements Syntax)
   - Generates: user stories, design documents, Mermaid.js architecture diagrams, database schemas, API stubs, task/subtask lists with test requirements
   - Specs stay synced with evolving codebase
   - Developers can author code and ask Kiro to update specs
   - **Philosophy:** "Think before coding" -- reduces architectural drift

2. **Hooks (Agent Hooks)** -- Event-driven background automation:
   - Trigger on: file save, file create, file delete, or manual trigger
   - Examples:
     - Save React component -> auto-update test file
     - Modify API endpoint -> refresh README
     - Pre-commit -> security scan for leaked credentials
   - Stored in `.kiro/hooks/` directory with `.kiro.hook` extension
   - Can execute AI prompts or shell commands
   - Team-wide enforcement via Git

3. **Autopilot Mode** -- AI works on large tasks without constant guidance

4. **Agentic Chat** -- Ad-hoc coding tasks with:
   - File context providers
   - URL context providers
   - Documentation context providers
   - Multimodal input (upload design images, whiteboard sketches)

5. **Steering Rules** -- Guide AI behavior across projects:
   - Project-level configuration
   - Human-readable documentation that both humans and AI reference
   - Captures product vision, technical architecture, development patterns

6. **MCP Support** -- Connects to databases, APIs, external tools

### File Editing Approach

- **Spec-Guided:** Code generation follows the established spec
- **Task-Based:** Large features broken into manageable tasks from spec
- **Autonomous:** Can work on tasks without constant human approval in autopilot mode
- **Sync Back:** Code changes can trigger spec updates to keep documentation current

### Command Execution Safety

- **Steering Documentation:** AI behavior guided by human-readable project docs
- **Spec Validation:** Changes validated against established specs
- **Hooks for Quality:** Automated quality checks on file operations
- **AWS Shared Responsibility Model:** AWS manages infrastructure security; customers responsible for application-layer

### Permission Model

- **AWS IAM Integration:** GovCloud availability with IAM Identity Center
- **Credit-Based:** Consumption model across tiers (Free: 50 credits, Pro: $20/mo, Pro+: $40/mo, Power: $200/mo)
- **No Open Source:** Proprietary tool; AWS controls feature roadmap

### Session Management

- **IDE Sessions:** Conversations within the Code OSS environment
- **Spec Persistence:** Specs persist across sessions as living documents
- **No Subagent Isolation:** Single-project context (vs. Intent's isolated git worktrees)

### Memory/Context System

- **Specs as Memory:** Structured specifications serve as persistent project context
- **Steering Files:** Human-readable project context (product vision, architecture, patterns)
- **AGENTS.md Compatible:** Reads industry-standard agent instruction files
- **AWS Integration:** Deep integration with Lambda, CDK, CloudFormation, CodeCatalyst

### Git Integration

- **Native Git:** Standard Code OSS git integration
- **Hooks via Git:** Team-wide hooks committed to repo for consistency
- **No Special Checkpoint System:** Standard git workflow

### Subagents/MCP Support

- **MCP:** Standard MCP support for external tool connections
- **Single Primary Agent** + hooks (not multi-agent like Intent)
- **Hook-Based Automation** replaces subagent delegation
- **AWS Services:** Deep integration via MCP and native IDE features

### Model Support

| Provider | Status |
|----------|--------|
| **DeepSeek** | **No** -- Currently Claude-only via Amazon Bedrock; alternative models "coming soon" |
| Anthropic (Claude) | **Exclusive** -- Claude Sonnet 4.0 primary, 3.7 fallback; thinking mode not available |
| OpenAI (GPT) | Planned |
| Google Gemini | Planned |
| Amazon Bedrock | Native infrastructure |
| **Auto Mode** | Smart routing between Bedrock model tiers |

- **Limitation:** Only Claude models currently available; limited model flexibility
- **AWS Advantage:** Direct AWS-Anthropic relationship may yield custom model features or early access

### Strengths (What to Borrow)

1. **Spec-Driven Development** -- The most structured approach to AI coding. Forces planning before execution, which dramatically reduces architectural drift and "vibe coding" problems.
2. **Hooks System** -- Event-driven automation is powerful for team consistency. "When you save X, do Y" patterns enforce standards without manual oversight.
3. **EARS Notation** -- Structured requirements syntax that both humans and AI can read/write effectively.
4. **Spec Syncing** -- Specs that stay current with code changes solve the "documentation drift" problem.
5. **AWS Integration** -- Deep integration with AWS services (Lambda, CDK, CloudFormation) is valuable for AWS-centric teams.
6. **VS Code Compatibility** -- Being Code OSS-based means zero IDE switching cost for VS Code users.

### Weaknesses (What to Improve)

1. **Claude-Only** -- No DeepSeek, no GPT, no Gemini. Severe model lock-in despite AWS's model-agnostic rhetoric.
2. **No Open Source** -- Proprietary tool controlled by AWS. Community cannot contribute or fork.
3. **AWS Lock-in** -- Deep integration with AWS services creates infrastructure coupling.
4. **Preview Quality** -- Terminal integration "needs improvement" per reviews; bugs expected.
5. **Limited Community** -- Very new product; limited documentation, tutorials, community content.
6. **No Multi-Agent** -- Single primary agent vs. competitors' multi-agent orchestration.
7. **Steeper Learning Curve** -- Spec-driven approach requires learning new concepts vs. familiar chat patterns.
8. **No Thinking Mode** -- Claude thinking/reasoning mode not available, limiting complex problem-solving.

---

## 6. Continue.dev

### Overview

Continue.dev is an open-source AI coding tool that plugs into VS Code and JetBrains IDEs. It distinguishes itself through full configurability via YAML files, support for any AI model, local/offline capability, and a privacy-first approach. It's Apache 2.0 licensed and has strong enterprise appeal due to its air-gapped deployment capability.

**Key Identity:** Open-source IDE extension (VS Code + JetBrains), YAML-configured, any model, privacy-first, four modes (Chat/Edit/Agent/Autocomplete).

### Architecture

- **Type:** IDE Extension (VS Code + JetBrains IDEs)
- **License:** Apache 2.0 (fully open source)
- **Configuration:** YAML-based configuration files
- **Privacy Model:** Can be fully air-gapped with local models
- **Backend:** Intercepts code requests and sends to chosen AI provider

### Installation

```bash
# VS Code
# Search "Continue" in Extensions marketplace
# Or: code --install-extension Continue.continue

# JetBrains
# Search "Continue" in JetBrains Plugin Marketplace

# Configuration via ~/.continue/config.yaml
```

### Key Features

1. **Four Interaction Modes:**
   - **Chat Mode:** Conversational AI pair programmer with codebase context
   - **Edit Mode:** Modify specific code sections; select code, describe changes, AI applies edits preserving formatting
   - **Plan Mode:** Read-only sandbox for safe exploration -- suggests changes without touching anything. Perfect for understanding unfamiliar codebases
   - **Agent Mode:** Autonomous multi-file operations; handles complex refactoring across 80+ files

2. **Any Model Support:**
   - OpenAI, Anthropic, Google, local models
   - OpenAI-compatible APIs (AskCodi, etc.)
   - Multiple providers for different tasks
   - Switch providers per task

3. **Local/Offline Mode:**
   - Full air-gapped operation with Ollama
   - Code never leaves machine
   - Enterprise-friendly for proprietary codebases

4. **MCP Support:**
   - Connect AI models to external systems
   - Databases, documentation, APIs
   - Works in all modes (Plan, Agent)

5. **Autocomplete** -- Inline code suggestions:
   - Enhanced IntelliSense with AI power
   - Suggests as you type

6. **Context Providers** -- Rich context via `@` mentions:
   - `@web` -- Fetch content from URL
   - `@file` -- Include specific files
   - `@codebase` -- Search entire codebase
   - `@terminal` -- Include terminal output

### File Editing Approach

- **Edit Mode:** Targeted edits preserving formatting and style
- **Agent Mode Tools:**
  - `create_new_file` -- Create new files
  - `edit_existing_file` -- Modify existing files (diff-based)
  - Full read/write tool set
- **Plan Mode:** Read-only exploration before making changes
- **No Semantic Search:** No vector embeddings (relies on codebase search via providers)

### Command Execution Safety

- **Mode-Based Safety:**
  - Plan Mode: Read-only (no file creation, editing, or command execution)
  - Agent Mode: Full tool access with permission controls
  - Chat Mode: No tools (information only)
- **Tool Permission Policies:** Per-tool policies: `Automatic` (skip permission) or manual approval
- **Read-Only by Default:** Plan mode provides safe exploration environment

### Permission Model

- **Tool Policies:** Configure automatic vs. manual approval per tool
- **Mode Filtering:**
  - Chat: No tools
  - Plan: Read-only tools only
  - Agent: All tools including write/execute
- **Enterprise:** Air-gapped deployment option for strict compliance

### Session Management

- **Extension-Based:** Sessions managed within IDE
- **No Explicit Checkpoints:** No built-in snapshot system
- **History:** Chat history within IDE panel

### Memory/Context System

- **`.continue/rules/`:** Project-specific rule blocks
- **`config.yaml`:** Global configuration with model settings, custom commands
- **Context Providers:** `@file`, `@web`, `@codebase`, `@terminal` for rich context
- **No Persistent Memory:** Unlike Goose, no cross-session memory tool
- **No Vector Indexing:** No semantic search; relies on grep/glob + AI provider context

### Git Integration

- Standard IDE git integration (VS Code/JetBrains native)
- No special AI git features (no AI commit messages like Zed)
- `@diff` context provider for including diffs in chat

### Subagents/MCP Support

- **MCP Support:** Yes, works in all modes
- **No Native Subagents:** Agent mode provides autonomous operation but not subagent delegation
- **Custom Commands:** Configurable in YAML (closest thing to recipes)

### Model Support

| Provider | Status |
|----------|--------|
| **DeepSeek** | **Yes** -- Via any OpenAI-compatible API; explicitly configurable |
| OpenAI (GPT) | Full support |
| Anthropic (Claude) | Full support |
| Google Gemini | Full support |
| Local (Ollama) | Full support -- 100% offline |
| **Any Provider** | Any OpenAI-compatible API |

- **Per-Task Model Selection:** Different models for different operations
- **Local Deployment:** Full air-gapped capability

### Strengths (What to Borrow)

1. **True Open Source** -- Apache 2.0 with no corporate control. Full transparency and community governance.
2. **Privacy-First Design** -- Air-gapped deployment with local models means code never leaves the machine. Critical for enterprises.
3. **Plan Mode** -- Read-only sandbox for safe exploration is a genuinely useful safety feature. Reduces anxiety about AI making unwanted changes.
4. **YAML Configuration** -- Human-readable, version-controlled, shareable configuration.
5. **Universal Model Support** -- Any OpenAI-compatible API works. No vendor restrictions.
6. **Multi-IDE** -- Works in both VS Code and JetBrains (unlike Roo which is VS Code only).
7. **Context Providers** -- Rich `@`-mention system for bringing context into conversations.

### Weaknesses (What to Improve)

1. **Polish Gap** -- Described as a "Swiss Army Knife that sometimes fails to cut" -- less polished than paid alternatives.
2. **No Semantic Search** -- No vector embeddings or semantic codebase indexing (unlike Cursor or Cody).
3. **Agent Mode Limitations** -- Agent mode less capable than Claude Code or Cursor for complex multi-file operations.
4. **No Checkpoints** -- No built-in snapshot/rollback mechanism.
5. **Edit Mode Simplicity** -- Inline editing less sophisticated than Cursor's -- tends to work for simple edits but struggles with complex refactoring.
6. **No Persistent Memory** -- No cross-session memory (unlike Goose's memory extension).
7. **Configuration Burden** -- YAML configuration offers power but requires manual setup vs. "it just works" alternatives.

---

## 7. Comparative Summary

### Architecture Comparison

| Tool | Type | Language | License | Platforms |
|------|------|----------|---------|-----------|
| **Roo Code** | VS Code Extension | TypeScript | Open Source | VS Code only |
| **Crush** | TUI (Terminal UI) | Go | Open Source | macOS, Linux, Windows, BSD |
| **Goose** | CLI + Desktop | Rust (CLI), Tauri (Desktop) | Apache 2.0 (AAIF) | macOS, Linux, Windows |
| **Zed AI** | Native IDE | Rust | GPL (editor) + Apache 2 (UI) | macOS, Linux, Windows (beta) |
| **Kiro** | Full IDE (Code OSS fork) | TypeScript (Code OSS base) | Proprietary (AWS) | macOS, Windows, Linux |
| **Continue.dev** | IDE Extension | TypeScript | Apache 2.0 | VS Code + JetBrains |

### File Editing Approach

| Tool | Primary Method | Diff Support | Multi-File | Rollback |
|------|---------------|--------------|------------|----------|
| **Roo Code** | Diff-based edits | Yes (unified diff) | Yes | Checkpoints (git) |
| **Crush** | edit, patch, multiedit tools | Yes | Yes (multiedit) | No |
| **Goose** | patch_file, write_file | Yes (patch) | Via recipes | No |
| **Zed AI** | Diff-aware editing | Yes | Yes (agent) | Undo to last tool call |
| **Kiro** | Spec-guided generation | Yes | Yes (autopilot) | Standard git |
| **Continue.dev** | edit_existing_file tool | Yes | Yes (agent mode) | No |

### Command Execution Safety

| Tool | Permission Model | Auto-Approve | Read-Only Mode | YOLO Mode |
|------|-----------------|--------------|----------------|-----------|
| **Roo Code** | Tool groups (allow/ask/deny) | Per tool | Ask/Architect modes | No |
| **Crush** | Tool disablement + yolo flag | No | No | `--yolo` flag |
| **Goose** | Extension-level permissions | Configurable | No | No |
| **Zed AI** | Per-tool auto/deny/confirm | Configurable | No | No |
| **Kiro** | Steering rules + hooks | Via hooks | No | Autopilot mode |
| **Continue.dev** | Per-tool automatic/manual | Configurable | Plan mode | No |

### Model Support

| Tool | DeepSeek | Claude | GPT | Gemini | Local | Provider Count |
|------|----------|--------|-----|--------|-------|----------------|
| **Roo Code** | **Yes** (native) | Yes | Yes | Yes | Yes | 25+ |
| **Crush** | **Yes** (OpenRouter) | Yes | Yes | Via OpenRouter | Yes | Custom APIs |
| **Goose** | **Yes** (OpenRouter) | Yes | Yes | Yes | Yes | 25+ |
| **Zed AI** | **Yes** (Ollama) | Yes | Yes | Yes | Yes | 6+ native |
| **Kiro** | **No** (Claude only) | **Only** | Planned | Planned | No | 1 (Bedrock) |
| **Continue.dev** | **Yes** (any API) | Yes | Yes | Yes | Yes | Unlimited |

### MCP Support

| Tool | MCP Support | Transports | Subagents |
|------|------------|------------|-----------|
| **Roo Code** | Full (project + global) | stdio | Orchestrator (deprecated) |
| **Crush** | Full | stdio, HTTP, SSE | `agent` tool |
| **Goose** | **First-class** (architecture) | stdio, HTTP, SSE | Via recipes |
| **Zed AI** | Yes (for built-in agent) | stdio | Via ACP agents |
| **Kiro** | Basic | stdio | No (hooks instead) |
| **Continue.dev** | Yes | stdio | No |

### Memory/Context System

| Tool | Persistent Memory | Project Context | Semantic Search | Cross-Session |
|------|------------------|-----------------|-----------------|---------------|
| **Roo Code** | Memory Bank / AGENTS.md | `.roorules` | Qdrant (Roo) / Pending (Kilo) | Yes |
| **Crush** | CRUSH.md / AGENTS.md | Session-based | No | Per-project |
| **Goose** | Memory Extension (`remember`) | Recipes | No | Yes (memory) |
| **Zed AI** | ACP sessions | @-mentions | No (Zeta has LSP context) | Via external agents |
| **Kiro** | Specs (living docs) | Steering files | No | Specs persist |
| **Continue.dev** | No (config only) | `.continue/rules` | No | No |

### Git Integration

| Tool | AI Commit Messages | Checkpoints | Git Tools |
|------|-------------------|-------------|-----------|
| **Roo Code** | No | Git snapshots | Via tools |
| **Crush** | No | No | Direct git access |
| **Goose** | Via MCP | No | Via MCP/recipes |
| **Zed AI** | Yes (built-in) | Undo to tool call | Built-in native |
| **Kiro** | No | No | Standard Code OSS |
| **Continue.dev** | No | No | Standard IDE |

### DeepSeek Support Summary

| Tool | DeepSeek Support | Method | Notes |
|------|-----------------|--------|-------|
| **Roo Code** | **Excellent** | Native provider, 128k context | `deepseek-chat`, `deepseek-reasoner` |
| **Crush** | **Good** | OpenRouter, OpenAI-compatible APIs | `deepseek_coder` prompt format in predictions |
| **Goose** | **Good** | OpenRouter, custom providers | Via 25+ provider ecosystem |
| **Zed AI** | **Good** | Ollama local, OpenAI-compatible servers | `deepseek_coder` format in edit predictions |
| **Kiro** | **None** | Not available | Claude-only; "coming soon" for others |
| **Continue.dev** | **Excellent** | Any OpenAI-compatible API | Fully configurable |

---

## 8. Key Takeaways for Our Tool

### Features to Definitely Borrow

1. **Roo Code's Mode System** -- Code/Architect/Ask/Debug separation is brilliant. Prevents token waste and keeps AI focused. Custom modes enable team-specific workflows.

2. **Goose's Recipes** -- YAML-based reusable, version-controlled, composable workflows are unique. The "rules change behavior, recipes change actions" philosophy is correct.

3. **Goose's MCP-First Architecture** -- Building everything as MCP extensions creates infinite extensibility without core code changes.

4. **Zed's ACP Protocol** -- Supporting Agent Client Protocol means our tool can work in ANY editor. This is strategic distribution.

5. **Zed's Native Speed** -- Rust/Go performance matters. Electron-based tools feel sluggish by comparison.

6. **Kiro's Spec-Driven Development** -- Planning before coding dramatically improves output quality for complex tasks. The EARS notation approach is worth studying.

7. **Kiro's Hooks System** -- Event-driven automation (on save, on create) for team-wide consistency enforcement.

8. **Crush's Mid-Session Model Switching** -- Changing LLMs while preserving context optimizes cost/quality dynamically.

9. **Crush's LSP Integration** -- Real code intelligence from language servers makes AI understanding much more accurate.

10. **Continue.dev's Plan Mode** -- Read-only sandbox for safe exploration reduces user anxiety.

11. **Continue.dev's Privacy-First Design** -- Air-gapped deployment capability is essential for enterprise adoption.

### Anti-Patterns to Avoid

1. **Kiro's Model Lock-in** -- Being Claude-only is a critical weakness. Must support DeepSeek, GPT, and others from day one.

2. **Roo Code's Platform Lock-in** -- Being VS Code-only limits adoption. Must support multiple editors or be editor-agnostic.

3. **Zed's CVE History** -- Permission bypass vulnerabilities (CVE-2025-55012) show that permission models MUST be robust. Never auto-approve dangerous operations without explicit user opt-in.

4. **Crush's Planning Weakness** -- Executing single commands instead of batching is slow and token-inefficient. Need intelligent batching.

5. **Kiro's Closed Source** -- Proprietary tools lose to open-source in developer adoption. Apache 2.0 or similar is the right license.

6. **Continue.dev's Polish Gap** -- "Swiss Army Knife that sometimes fails to cut" -- need to nail the core experience, not just features.

7. **Roo Code's Shutdown Churn** -- Projects that abandon their user base create permanent trust damage. Linux Foundation governance (like Goose) provides stability.

### Strategic Insights

1. **The MCP-First Movement** -- Goose and the AAIF have established MCP as THE standard. Any new tool must be MCP-first or will be left behind.

2. **ACP as Distribution Channel** -- Zed's Agent Client Protocol is the emerging "LSP for agents." Supporting ACP means instant editor compatibility.

3. **Spec-Driven vs. Vibe Coding** -- Kiro proves there's demand for structured planning. But most users want BOTH: quick chat for simple tasks, specs for complex ones.

4. **The Recipe/Workflow Layer** -- Goose's recipes represent a new abstraction layer above prompts. This is where team knowledge and institutional process lives.

5. **Multi-Model is Table Stakes** -- Every tool except Kiro supports multiple models. Per-task, per-mode, and even per-file model selection is the future.

6. **Local-First for Privacy** -- Enterprise adoption requires air-gapped/local capability. Continue.dev and Goose prove this is technically feasible.

7. **TUI vs. CLI vs. IDE** -- There's no single right answer. Crush (TUI), Goose (CLI+Desktop), Zed (IDE), and Roo (extension) all succeed in different contexts. The ACP protocol bridges these worlds.
