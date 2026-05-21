## 9. Subagents, Config, and Distribution

### 9.1 Subagent System

The primary motivation for subagent delegation is context isolation, not parallelism. When a parent agent delegates a focused subtask—"review all test coverage for the auth module"—the child's full conversation history (which may exceed 10,000 tokens) stays contained. Only a 1,000–2,000 token summary returns to the parent, yielding an 80–90% context savings per delegation [^5^]. Claude Code's subagent pattern implements this through fresh context windows, isolated tool pools, and optional git worktree isolation; this architecture is the baseline for DeepSeek Code TUI's design [^1^].

#### 9.1.1 MVP Subagents

The MVP ships four subagent roles that cover the core code-modification workflow. Each role maps to a specific system prompt and a restricted toolset, ensuring that the model stays within its lane.

| Role | Purpose | Model | Toolset | Max Turns |
|------|---------|-------|---------|-----------|
| `Lead` | Main orchestrator; handles user-facing reasoning and coordinates delegation | V4 Pro | All tools + `delegate` | 50 |
| `Planner` | Task decomposition; outputs file-level change plans without executing | V4 Flash | `Read`, `Grep`, `Glob` | 15 |
| `Implementer` | Executes SEARCH/REPLACE blocks against specified files | V4 Flash | `Read`, `Edit`, `Write`, `Bash(git)` | 30 |
| `Reviewer` | Code review of proposed or committed changes | V4 Flash | `Read`, `Grep`, `Glob`, `Bash(git diff)` | 20 |

The `Lead` agent runs on V4 Pro because it performs complex multi-step reasoning, resolves ambiguities in user intent, and decides when to delegate. All other MVP agents run on V4 Flash ($0.14/M input tokens vs. Pro at $1.74/M) [^1^], keeping delegation costs low even at high volume. The `Planner` has no write tools—its job is to read the codebase, understand constraints, and emit a structured plan that the `Implementer` executes. This separation prevents the common failure mode where a planning model prematurely edits files before the plan is complete.

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

Subagents cannot spawn nested subagents. This flat hierarchy prevents exponential context fragmentation and keeps the delegation graph inspectable. Each subagent invocation is a fresh instance with opt-in memory scope via the `memory` field in its definition [^1^].

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

Every subagent definition follows a consistent template derived from Claude Code's YAML frontmatter pattern [^1^]:

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

The parent appends this summary (typically 500–1,500 tokens) in place of the 10,000+ token full transcript, achieving the documented 80–90% savings [^5^].

### 9.2 Configuration Files

Configuration uses a two-layer hierarchy: global settings in `~/.config/deepseek-code/config.toml` apply across all projects, while project-specific settings in `.deepseek-code/project.toml` override them for the current repository. This mirrors Claude Code's 4-level permission hierarchy (managed → user → project → local) [^1^] but collapses the two system-level scopes into one for simplicity in the MVP.

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

The `deny_first` flag implements the deny-first rule evaluation that Claude Code's permission system uses, where a broad deny ("deny all `rm -rf`") cannot be overridden by a narrow allow [^1^]. This prevents the most common permission escalation bug: an overly permissive allow rule accidentally permitting a dangerous operation.

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

The `[instructions]` section implements the CLAUDE.md pattern: project-specific context is injected as user-level context (probabilistic compliance) rather than system prompt (deterministic compliance), which the model treats as strong suggestions [^1^]. The `[[instructions.directory]]` entries support per-directory rules with path-based scoping, allowing different conventions for `src/routes/` versus `src/db/`.

#### 9.2.3 Configuration Precedence

Settings resolve in strict precedence, highest to lowest:

1. Command-line flags (`--model`, `--permission-mode`)
2. Environment variables (`DEEPSEEK_CODE_MODEL`, `DEEPSEEK_CODE_API_KEY`)
3. Project config (`.deepseek-code/project.toml`)
4. Global config (`~/.config/deepseek-code/config.toml`)
5. Built-in defaults

Array fields like `permissions.allow` and `permissions.deny` merge across scopes rather than replace. If the global config denies `Bash(rm -rf *)` and the project config allows `Bash(rm -rf /tmp/*)`, the deny rule still blocks because deny-first evaluation applies at merge time [^1^]. This prevents a project-level configuration from silently weakening safety rules established at the global level.

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
