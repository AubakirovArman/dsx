## 8. Memory, Safety, and Patch Engine

This chapter addresses three infrastructure layers that determine whether the agent operates reliably: the memory and context system that feeds the model, the safety layer that gates tool execution, and the patch engine that transforms LLM output into verified file changes. Each layer uses concrete data structures, classification rules, and recovery mechanisms. The chapter assumes familiarity with the agent loop described in Chapter 7 and builds on its ReAct-based execution model.

### 8.1 Memory and Context System

#### 8.1.1 Tiered Memory Architecture

Relying solely on the LLM context window for memory leads to context pollution and degraded performance over long sessions. [^1^] The system implements a three-tier memory model:

**Session memory** stores the conversation transcript: user messages, assistant reasoning traces, tool calls, and tool results. This tier is ephemeral, cleared when the session ends, and checkpointed to SQLite every 5 turns for crash recovery.

**Project memory** persists across sessions and stores file summaries, architectural decisions, error patterns, and command history. It is scoped to the project root directory (identified by the containing git repository). Structured data lives in SQLite; long-form notes (decision logs, convention files) are stored as Markdown in `.deepseek/memory/` to remain human-editable and version-controllable. [^2^]

**User memory** stores global preferences: permission mode defaults, model selection, custom rules, and API key references (not the keys themselves). This tier is stored in `~/.deepseek/preferences.toml` and loaded at startup.

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

Key design decisions: `messages` stores `tool_calls` and `tool_results` as JSONB columns to preserve nested structure without a normalized sub-table that would complicate the append-only write path. `file_summaries` stores SHA-256 hashes to detect external modifications without re-reading files. `memory_index` stores structured observations (file relationships, error patterns, conventions) inserted at session end and retrieved via BM25 text search at the start of subsequent sessions. No vector embeddings are used at this layer; structured retrieval achieves ~170K tokens/year versus ~19.5M for full context replay, justifying the added complexity. [^3^]

#### 8.1.3 Context Assembly: Nine Ordered Sources

For every model call, the assembler builds messages from nine sources in strict priority order:

1. **System prompt** — base behavior rules and available tool schemas
2. **Project instructions** — `.deepseek.md` in the project root storing team conventions [^2^]
3. **Auto-memory block** — observations from prior sessions (file relationships, error patterns, decisions)
4. **File contents** — files explicitly added to the conversation
5. **Search results** — ripgrep or tree-sitter index lookups
6. **Git status** — branch, changed files, recent commits
7. **Conversation history** — compacted via the graduated pipeline if necessary
8. **Dynamic tool schemas** — filtered to relevant tools when semantic filtering is enabled
9. **User message** — the current query

Sources 1-3 form the stable prefix and benefit from prompt caching; sources 4-8 vary per turn. This split aligns with DeepSeek's automatic context caching, which reduces repeated-prefix costs by 10x on cache hits. [^4^]

When the assembled context exceeds the effective context window (`context_window - output_reserve - safety_buffer`), a graduated compaction pipeline applies in order of increasing destructiveness: per-tool-result budget capping (8K characters per result), history snipping (dropping tool results older than 20 turns), cache-aware micro-compaction (preserving cache boundaries), context collapse projection (read-time virtual view over history), and finally full auto-compaction (model-generated summary as last resort). [^5^]

### 8.2 Tool Execution Safety

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

Commands matching no pattern enter heuristic scoring: +3 for destructive keywords (`rm`, `drop`, `delete`, `truncate`, `format`, `dd`), +2 for write flags (`-f`, `--force`, `-y`, `--yes`), +2 for privilege escalation (`sudo`, `doas`), +1 for globs (`*`, `?`), and -1 for safe flags (`--dry-run`, `-n`, `--list`). Score 0 or below maps to safe, 1-2 to sensitive, 3+ to destructive. [^6^]

#### 8.2.2 Permission Rules

Rules follow deny-first evaluation: deny rules checked first, then ask, then allow. The first match wins. A deny rule always takes precedence over an allow rule, even when more specific. A broad `Bash(rm -rf *)` deny cannot be overridden by a narrow `Bash(rm -rf /tmp/build)` allow. [^7^]

Default policy: safe commands are allowed, sensitive commands prompt for confirmation, destructive commands are denied by default with per-case override allowed. This default-deny stance is essential because approximately 93% of permission prompts are approved by users in production, making interactive confirmation behaviorally unreliable as the sole safety mechanism. [^8^]

Rules are scoped at three levels (highest to lowest precedence): managed policies in `/etc/deepseek/safety.toml`, user policies in `~/.deepseek/safety.toml`, and project policies in `.deepseek/safety.toml` (committed to git). Array settings like `permissions.allow` merge across scopes. [^7^]

```toml
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

Output is truncated to 10,000 lines or 1 MB, whichever is reached first, with a marker `[... output truncated: N lines hidden]` appended. [^5^]

Secrets redaction scans all output through a DFA matcher: AWS access key IDs (`AKIA[20 chars]`), GitHub tokens (`ghp_[36 chars]`), generic API keys (`[key|token|secret|password]=[alphanumeric]{16,}`), and private key headers. Matches are replaced with `[REDACTED:<type>]`.

Working directory restriction constrains file operations to the project root and subdirectories. Path traversal attempts (e.g., `Read(/etc/passwd)`, `Write(../../outside)`) are rejected before execution. Symlinks are resolved before validation.

### 8.3 Patch Engine

#### 8.3.1 SEARCH/REPLACE Primary Format

The patch engine uses SEARCH/REPLACE blocks with 4-tier matching. Content-addressed editing (search strings) outperforms position-addressed editing (line numbers): minimal unified diff achieves ~14% pass@1 accuracy on LLM edit benchmarks, while content-aware formats like BlockDiff reach ~56% and SEARCH/REPLACE achieves ~70-80% on evolved codebases. [^9^]

```
path/to/file.rs
<<<<<<< SEARCH
    let mut config = Config::load("settings.toml");
    config.port = 8080;
=======
    let mut config = Config::load("settings.toml");
    config.port = env::var("PORT").unwrap_or(8080);
>>>>>>> REPLACE
```

**Tier 1 — Exact match.** Literal byte-for-byte comparison. Succeeds for ~60-70% of edits. [^10^]

**Tier 2 — Whitespace-insensitive match.** Leading and trailing whitespace normalized per line. Adds ~10-15% cumulative coverage. [^10^]

**Tier 3 — Indentation-preserving match.** Relative indentation preserved but absolute level allowed to shift. Handles mixed indentation or code copied from different nesting levels. Adds ~5-10% cumulative coverage. [^11^]

**Tier 4 — Fuzzy match.** Levenshtein similarity between SEARCH block and candidate regions, searching outward from estimated anchor locations. Accepted if similarity exceeds 0.75 and the match is unique (second-best must score at least 0.15 lower). [^12^]

If all tiers fail, the engine returns a `SearchNotFound` error with the file path, SEARCH block, similarity scores of best and second-best candidates, and a suggested correction. For files under ~400 lines with extensive edits (>50% of file), the engine falls back to full-file rewrite. [^13^]

#### 8.3.2 Edit Pipeline

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

**Apply.** Edits written to disk after re-checking the file hash against cache. Hash mismatch aborts with `FileChangedError`. [^14^]

**Format.** Project formatter runs once per file, deferred until all edits in a multi-file transaction are applied to avoid line-number invalidation. [^15^]

**Test.** Relevant tests execute (convention-based: `src/foo.rs` maps to `tests/test_foo.rs`). Failures return error output to the LLM; pipeline loops to Generate.

**Commit.** On success, a git commit is created. On failure after Apply, `git reset --hard` to the pre-edit checkpoint. [^16^]

#### 8.3.3 Core Data Structures

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

The protocol: (1) create a git checkpoint by committing the current working tree; (2) validate all SEARCH/REPLACE blocks across all files (tiers 1-4); abort if any block fails; (3) apply edits bottom-up by line number within each file to minimize line-shift effects; (4) run the code formatter once per modified file after all edits are applied; (5) execute tests; (6) on pass, create a final commit; on fail, `git reset --hard` to checkpoint and return errors to the LLM. [^16^]

Bottom-up ordering is critical for multi-hunk edits. If two hunks target lines 50 and 150, applying line 150 first ensures that insertions or deletions around line 50 do not shift the second hunk's location. This eliminates the "line drift" problem that causes sequential edit strategies to fail ~15-20% of the time. [^17^]

Git serves as the transaction log. The checkpoint commit is the atomic restore point. This trades fine-grained partial rollback for simplicity: `git reset --hard` restores edited files and any side effects from build processes or auto-generated files. [^16^] Checkpoints apply to every edit, creating linear history traversable via `/undo` (executes `git revert` on the most recent agent commit). Checkpoints are garbage-collected after 7 days or when exceeding 50 per session, with older checkpoints squashed into an archive commit to prevent repository bloat.
