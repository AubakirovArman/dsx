## 7. Agent Loop and Prompting Protocol

The agent loop orchestrates the cycle of reasoning, tool invocation, permission validation, and error recovery that transforms a user request into verified code changes. The design synthesizes the ReAct pattern [^2^] — interleaving reasoning traces with action steps — with Claude Code's `queryLoop` architecture [^1^], adapted for DeepSeek V4's dual-mode API. The result is a deterministic 14-step loop with five execution modes and a phase-specialized prompting protocol.

---

### 7.1 Agent Loop Design

#### 7.1.1 The 14-Step Loop

The loop extends Claude Code's 9-step pipeline [^1^] with stages for plan validation, diff review, and test verification. Each step is either a pure transformation or a gated side effect.

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

A **turn** consists of steps 3–14; a task may span multiple turns. The loop terminates on one of five stop conditions [^1^]: no `tool_use` blocks in the response (natural finish), max turn count exceeded (default 30), context overflow even after full compaction, explicit user abort, or hook intervention blocking execution.

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

Two tool execution paths exist. The `StreamingToolExecutor` begins executing read operations as soon as their arguments parse from the streaming response. A fallback `runTools()` path classifies calls via `partitionToolCalls()` into concurrent-safe (Read, Grep, Glob) and exclusive (Write, Edit, Bash) sets [^1^]. Read tools run in parallel; write tools run serially to prevent race conditions.

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

The recovery ladder uses named continue points rather than returns, making every transition independently testable [^1^]. Guards prevent infinite loops: one-shot compaction flags, hard caps of 3 recovery attempts per trigger type, and circuit breakers on repeated failures. Stop hooks never run on error responses, preventing "error → hook blocks → retry → error" spirals [^1^].

The `tool_use` / `tool_result` invariant is enforced structurally: every `tool_use` must have a paired `tool_result` before the next API call. On abort during streaming, the executor drains remaining requests by emitting synthetic `tool_result` blocks; the API rejects assistant messages with unmatched `tool_use` blocks [^1^].

---

### 7.2 Execution Modes

The execution mode determines the autonomy boundary between agent and user. Claude Code implements seven modes [^1^]; this design adapts five for common workflows. The mode is set at session start and overridable per-command.

**No-edit mode (7.2.1)** makes the agent read-only. Write tools (Edit, Write, Bash with side effects) are filtered from the tool schema before each model call. The model may describe changes it would make, but the permission gate intercepts write calls with `Permission::Deny`. This mode serves code review, architecture explanation, and onboarding.

**Plan-only mode (7.2.2)** generates a detailed execution plan at step 5 and pauses before any tool invocation. The plan includes files to read, files to modify, changes per file (described, not implemented), test commands, and estimated token cost. The user must approve before the agent proceeds. Appropriate for multi-file refactoring (5+ files) and architecture decisions where global coherence matters [^6^].

**Auto-approve mode (7.2.3)** is the recommended default. Non-destructive operations (Read, Grep, Glob, test runs) execute without confirmation; destructive operations (Edit, Write, risky Bash) trigger permission prompts. The permission pipeline evaluates rules in strict deny → ask → allow order [^1^]. Deny rules always take precedence, even when allow rules are more specific.

Risk classification uses pattern matching for MVP, graduating to an ML classifier in v1. Anthropic's analysis found users approve approximately 93% of permission prompts, making interactive confirmation behaviorally unreliable as the sole safety mechanism [^1^]. Automatic risk classification with tiered escalation compensates for this auto-approval tendency.

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

Five specialized system prompts are tuned to specific workflow phases. They compose dynamically at runtime: base identity + phase-specific instructions + DeepSeek configuration + filtered tool definitions [^3^]. All prompts use the OpenAI message format; DeepSeek V4 is fully API-compatible [^3^]. The system prompt is passed as the `system` parameter, not a user message, to maximize compliance probability.

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

The thinking/non-thinking split is a critical optimization. DeepSeek V4's `reasoning_content` field lets the TUI display a "thinking..." indicator with the reasoning stream, then replace it with the final answer [^3^]. This is more responsive than waiting for the complete reasoning chain. The orchestrator tracks which agent type requires which mode and sets the API parameter per turn.

At runtime, the five prompts compose into a single system message: base identity and safety rules from the main prompt, task guidance from the phase-specific prompt (planner, implementer, or reviewer), model configuration from the DeepSeek prompt, and relevant tool definitions filtered semantically to the current phase [^6^]. This composition keeps individual prompts focused while ensuring no critical safety rule is omitted.
