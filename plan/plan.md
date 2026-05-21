# Plan: DeepSeek Code TUI — Architecture & Product Document

## Goal
Create a comprehensive architecture/product document for a next-generation TUI/CLI coding agent powered by DeepSeek V4 (Pro / Flash), on par with Claude Code, Codex CLI, Gemini CLI, OpenCode, etc.

## Stages

### Stage 1 — Deep Research (Parallel)
Load skill: `deep-research-swarm`

Two parallel research tracks:

**Track A: DeepSeek V4 API Capabilities**
- Official API docs (deepseek.com/api-docs)
- Model specs: V4 Pro, V4 Flash
- Context length, thinking/non-thinking modes
- Function/tool calling support
- Streaming (reasoning + answer)
- OpenAI-compatible API details
- Anthropic-compatible API details
- Pricing, rate limits, deprecations
- Confirmed capabilities table

**Track B: Competitor Analysis**
- Claude Code: UX, features, strengths, weaknesses
- Codex CLI: architecture, features
- Gemini CLI: Google approach
- OpenCode: community/open approach
- Aider: multi-model, git integration
- Cline/Roo Code: IDE agents
- Cursor agent mode
- Zed AI workflows
- Other relevant tools
- Competitor matrix

### Stage 2 — Document Writing
Load skill: `report-writing`

Write the full architecture document (~20 sections) based on:
- Stage 1 research findings
- Rust/TUI best practices
- Agent architecture patterns

Sections:
1. Executive Summary
2. Confirmed DeepSeek V4 Capabilities
3. Competitor Analysis
4. Product Vision & Modes
5. Core Feature Set (A-J categories)
6. Architecture (21 layers)
7. Tech Stack Decision
8. TUI Design (12 screens)
9. Agent Loop Design
10. Prompting Protocol
11. Memory/Context System
12. Tool Execution & Safety
13. Patch Engine
14. Subagents
15. Config Files
16. Installation & Distribution
17. MVP/v1/v2 Roadmap
18. Repository Structure
19. Evaluation Plan
20. Risks & Trade-offs
21. First 10 Engineering Tasks
22. What Not to Build Yet
23. Final Recommendation

### Stage 3 — Artifact Production
Load skill: `docx`
Convert final markdown to .docx for delivery.

## Output
- `/mnt/agents/output/deepseek-code-architecture.md`
- `/mnt/agents/output/deepseek-code-architecture.docx`
