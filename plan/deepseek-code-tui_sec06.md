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

**States.** Streaming: status bar shows `◐ thinking...`; tool calls populate the right sidebar live as they stream in [^8^]. Code blocks render in nested `Block` widgets with tree-sitter highlighting.

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

Safety screen for code edits. Side-by-side SEARCH/REPLACE with per-hunk controls [^2^]. Below 70 columns, switches to stacked vertical layout.

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

The layout cache (default in ratatui 0.30) stores constraint calculations across frames. For static layouts like the main screen, this eliminates solver overhead; only dynamic content areas incur per-frame layout cost. Benchmarks show sub-millisecond full-frame renders at 80×24 and under 2ms at 200×60 [^3^].
