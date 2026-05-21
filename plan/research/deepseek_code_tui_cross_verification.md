# Cross-Verification: DeepSeek Code TUI Architecture

## High Confidence (Confirmed by 2+ independent sources)

| Finding | Sources | Confidence |
|---------|---------|------------|
| DeepSeek V4 Pro: 1.6T params, 49B active, 1M context, $1.74/M input | Official API docs, pricing page, multiple 3rd party analyses | HIGH |
| DeepSeek V4 Flash: 284B params, 13B active, 1M context, $0.14/M input | Official API docs, pricing page | HIGH |
| Tool calling: OpenAI-style, max 128 functions, streaming supported | Official API docs, integration guides | HIGH |
| Thinking mode: enabled by default, reasoning_effort controls depth | Official API docs, SDK examples | HIGH |
| Streaming: reasoning_content separate from content in delta | Official docs, reverse-engineering docs, SDK examples | HIGH |
| Context caching: automatic, 1/10 price for cache hits | Official docs, pricing page | HIGH |
| Legacy names deprecated July 24, 2026 | Official changelog, multiple sources | HIGH |
| Anthropic-compatible endpoint: https://api.deepseek.com/anthropic | Official docs, quickstart | HIGH |
| SEARCH/REPLACE blocks most reliable edit format | Aider docs, AdaEdit research, Claude Code analysis | HIGH |
| Rust ratatui is leading TUI framework (19k+ stars) | crates.io, GitHub, production usage | HIGH |
| Claude Code: 7 permission modes, ML classifier | Source code analysis (VILA Lab paper), official docs | HIGH |
| Subagents provide 80-90% context savings | Claude Code architecture, Anthropic docs | HIGH |
| Git-native workflow essential for safety | Aider, Claude Code, Gemini CLI all implement this | HIGH |
| Roo Code team shut down April 2026, Kilo Code is successor | Official announcements, VS Code marketplace | HIGH |
| Goose is MCP-first Rust application | Linux Foundation, GitHub, documentation | HIGH |

## Medium Confidence (Single authoritative source)

| Finding | Source | Confidence |
|---------|--------|------------|
| 75% discount on V4 Pro until May 31, 2026 | Official pricing page (noted as "extended") | MEDIUM |
| Rate limits are dynamic (no fixed numbers published) | Official docs | MEDIUM |
| Cache lifetime: hours to days | Official caching guide | MEDIUM |
| V4 Pro Max scores 80.6% on SWE-bench Verified | 3rd party benchmarks (morphllm.com) | MEDIUM |
| Crush uses Bubble Tea + LSP integration | GitHub, blog posts | MEDIUM |
| 93% of permission prompts approved by users | VILA Lab paper analysis | MEDIUM |

## Low Confidence / Unverified

| Finding | Issue |
|---------|-------|
| Exact RPM/TPM limits for V4 | Not published by DeepSeek |
| Whether thinking mode works reliably with tool calling in complex multi-turn scenarios | Limited production testing reported |
| Whether 384K max output is achievable in practice | Theoretical limit, real-world may be lower |
| Whether cache hit rate is predictable | "Best-effort" according to docs |

## Conflict Zones

| Conflict | Resolution |
|----------|------------|
| Edit format: Claude uses str_replace, Aider uses SEARCH/REPLACE, Gemini uses diff | Use SEARCH/REPLACE as primary (best research support), str_replace as fallback |
| Subagent model: Claude uses isolated context, OpenCode uses SQLite sessions, Goose uses MCP | Hybrid: isolated context + SQLite persistence (best of both) |
| TUI vs CLI priority: Claude Code is CLI-first, Crush is TUI-first, Aider is CLI-only | Offer both: CLI mode + TUI mode (differentiated approach) |
