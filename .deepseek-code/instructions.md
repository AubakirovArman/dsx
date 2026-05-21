# DSX Code Project Instructions

This is the DSX Code project itself — a Rust coding-agent runtime powered by DeepSeek V4.

## Build

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Architecture

- `crates/dsx-core`: Shared types and error definitions
- `crates/dsx-provider`: DeepSeek V4 API client
- `crates/dsx-agent`: Agent loop with Pro/Flash routing
- `crates/dsx-tui`: ratatui terminal workspace
- `crates/dsx-tools`: Tool definitions and registry
- `crates/dsx-patch`: SEARCH/REPLACE patch engine
- `crates/dsx-permissions`: 5-level risk classifier
- `crates/dsx-git`: Git operations
- `crates/dsx-fs`: File system with ignore support
- `crates/dsx-sandbox`: Command sandbox
- `crates/dsx-memory`: SQLite persistent storage
- `crates/dsx-session`: Session manager
- `crates/dsx-context`: Context assembly
- `crates/dsx-index`: Codebase indexing
- `crates/dsx-prompts`: System prompt builder
- `crates/dsx-mcp`: Model Context Protocol client
- `crates/dsx-eval`: Evaluation framework
- `crates/dsx-telemetry`: Telemetry and usage metering

## Key Design Decisions

1. **DeepSeek V4 is the model provider, DSX Code is the agent runtime.**
2. **SEARCH/REPLACE is the primary edit primitive** (4-tier matching).
3. **Permission-first UX**: trust over magic, audit log, rollback, command classifier.
4. **Patches, not direct writes**: model proposes, engine validates, user approves.
5. **Git-native**: every edit is a checkpoint.
6. **Rust all the way**: performance, correctness, single binary distribution.
