# Cross-Dimension Insights: DeepSeek Code TUI Architecture

## Insight 1: DeepSeek V4 API is Production-Ready for Agent Tools
- **Insight**: DeepSeek V4 API has all essential capabilities for building a coding agent: tool calling (128 functions), streaming (with separate reasoning_content), 1M context, thinking/non-thinking modes, and both OpenAI/Anthropic-compatible endpoints. The pricing (Flash $0.14/M input, Pro $1.74/M input) is 10-40x cheaper than Claude, enabling aggressive agent patterns.
- **Derived From**: Dim 01 (API research), Dim 04 (agent patterns)
- **Rationale**: Confirmed tool calling, streaming reasoning separation, and dual API compatibility mean we can build a single client that supports both formats. 1M context reduces need for aggressive compaction.
- **Implications**: Enables building a feature-complete agent without waiting for API maturity. Cost advantage allows using Pro for complex reasoning without budget constraints.
- **Confidence**: HIGH

## Insight 2: SEARCH/REPLACE Block Format is the Optimal Edit Primitive
- **Insight**: Aider's SEARCH/REPLACE blocks with 4-tier matching (exact → whitespace-insensitive → indentation-preserving → fuzzy) achieve the best balance of reliability and LLM-friendliness. Line-number-based diffs fail ~86% of the time; exact string replacement fails ~15-20%. SEARCH/REPLACE is the industry converging standard.
- **Derived From**: Dim 05 (patch engine), Dim 04 (agent patterns)
- **Rationale**: Multiple independent sources (Aider, AdaEdit research, Claude Code's str_replace analysis) confirm that content-addressed editing beats position-addressed. SEARCH/REPLACE is both human-readable and machine-applyable.
- **Implications**: The patch engine should use SEARCH/REPLACE as primary format, with exact-string (Claude-style) as secondary. Avoid unified diff for LLM-generated patches.
- **Confidence**: HIGH

## Insight 3: Rust Stack Unambiguously Wins for This Use Case
- **Insight**: For a TUI coding agent, Rust provides unmatched advantages: ratatui (19k+ stars, IDE-capable layouts), tree-sitter (100+ languages), tantivy (sub-10ms search), git2/gix (native git), and type-safe async streaming. Go's Bubble Tea and TS's Ink are 2-3 years behind in TUI maturity. The 30-40% memory savings and single-binary distribution are critical for a CLI tool.
- **Derived From**: Dim 02 (Rust ecosystem), Dim 04 (agent patterns)
- **Rationale**: No other language has a TUI framework as mature as ratatui combined with code parsing (tree-sitter) and search (tantivy). The crate ecosystem is production-ready.
- **Implications**: Start with Rust immediately. No benefit to prototyping in Go/TS first — Rust's ecosystem is already mature enough.
- **Confidence**: HIGH

## Insight 4: Permission System is the Critical Safety Differentiator
- **Insight**: Claude Code's 7-mode permission system with ML-based classifier and deny-first rule evaluation represents the state-of-the-art. ~93% of permission prompts are approved by users, making interactive confirmation unreliable as the sole safety mechanism. Risk classification must be automatic with tiered escalation.
- **Derived From**: Dim 04 (agent patterns), Dim 03 (competitors)
- **Rationale**: Academic analysis of Claude Code source + user behavior data show that humans auto-approve. A command classifier with automatic allow/ask/deny is essential.
- **Implications**: Build a command risk scorer from day one. Use pattern matching + heuristics for MVP, ML classifier for v1. Default-deny for destructive operations.
- **Confidence**: HIGH

## Insight 5: Subagents Provide 80-90% Context Savings
- **Insight**: Subagent delegation with context isolation returns 1-2K token summaries instead of 10K+ full histories. This is essential for 1M context windows — even large contexts need management. Claude Code's subagent pattern (fresh context, isolated tools, git worktree optional) is the optimal design.
- **Derived From**: Dim 04 (agent patterns), Dim 01 (API capabilities)
- **Rationale**: Context pollution is a real problem even with 1M tokens. Subagents prevent it while enabling parallel work. The ROI is clear: ~80-90% context savings per delegation.
- **Implications**: Architect subagent support into the core from MVP, even if only 2-3 agent types are implemented initially. Use V4 Flash for subagents (cheap, fast) and V4 Pro for main reasoning.
- **Confidence**: HIGH

## Insight 6: Context Caching is a Cost Game-Changer
- **Insight**: DeepSeek's automatic context caching (no code changes needed) reduces repeated-prefix costs by 10x (cache hit at 1/10 price). For a coding agent that repeatedly sends project context + conversation history, this means 50-80% cost reduction in practice.
- **Derived From**: Dim 01 (API research)
- **Rationale**: Stable prefix design (system prompt + project context + examples + changing task) naturally fits the caching model. Cache lifetime of hours-to-days is sufficient for coding sessions.
- **Implications**: Design prompts with stable prefixes intentionally. Monitor cache_hit ratios. Use the same conversation structure across turns.
- **Confidence**: HIGH

## Insight 7: The Market Gap is a DeepSeek-Native Agent with Rust Performance
- **Insight**: No existing tool combines: (a) DeepSeek-native optimization, (b) Rust performance, (c) full TUI IDE-like experience, (d) model-agnostic backend, (e) open source. Claude Code is locked to Anthropic, Aider lacks TUI polish, Crush lacks DeepSeek optimization, Goose is MCP-first not TUI-first.
- **Derived From**: Dim 03 (competitors), Dim 02 (Rust ecosystem)
- **Rationale**: Competitor analysis reveals a clear gap. Tools either lock to one model family OR lack TUI sophistication OR aren't performance-optimized.
- **Implications**: Position as "the fastest, most capable coding agent for DeepSeek — and any model." Open source + Rust performance + native DeepSeek optimization is a unique value proposition.
- **Confidence**: MEDIUM (market timing risk)

## Insight 8: Streaming Tool Calls are Essential for Low Latency
- **Insight**: Modern agents stream tool calls and begin executing before the full JSON is received. This reduces perceived latency by 30-50%. DeepSeek's streaming API supports this pattern natively.
- **Derived From**: Dim 04 (agent patterns), Dim 01 (API)
- **Rationale**: User perception of speed matters more than actual speed. Streaming execution creates a responsive feel even for complex multi-step tasks.
- **Implications**: Implement streaming tool execution from MVP. Show tool calls being constructed in real-time. Update TUI live as tools stream in.
- **Confidence**: HIGH

## Insight 9: Memory Must Be Multi-Tier, Not Just Context Window
- **Insight**: Even with 1M context, relying solely on the context window for memory leads to pollution and degraded performance. The optimal pattern is: (a) session memory (conversation), (b) project memory (file summaries, decisions), (c) user preferences (persistent), (d) tool result logs. SQLite + file-based is sufficient; vector DB is unnecessary for MVP.
- **Derived From**: Dim 04 (memory systems), Dim 05 (code editing)
- **Rationale**: Claude Code's 5-layer compaction pipeline exists because context windows alone don't work. Aider's repo-map achieves better token efficiency than naive full-context approaches.
- **Implications**: Build a tiered memory system from v0.2. Use SQLite for structured data, files for summaries, context window only for active working set.
- **Confidence**: HIGH

## Insight 10: Git-Native Workflow is Non-Negotiable
- **Insight**: Every successful coding agent (Aider, Claude Code, Gemini CLI) uses git as the primary safety and audit mechanism. Atomic commits per change, pre-edit checkpoints, and automatic rollback are must-have features, not nice-to-have.
- **Derived From**: Dim 05 (code editing), Dim 03 (competitors)
- **Rationale**: Git provides free auditability, rollback, and human review. Users expect it. Agents that don't integrate git deeply feel unsafe.
- **Implications**: Git integration must be in MVP 0.1, not deferred. Every edit → automatic commit. Pre-edit checkpoint before any change. `/undo` command that does git revert.
- **Confidence**: HIGH
