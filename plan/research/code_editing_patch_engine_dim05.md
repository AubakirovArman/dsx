# Code Editing, Patch Engines, and File Manipulation Patterns in AI Coding Agents

## Table of Contents
1. [Diff/Patch Formats](#1-diffpatch-formats)
2. [File Editing Pipeline](#2-file-editing-pipeline)
3. [AST-Aware Editing](#3-ast-aware-editing)
4. [Multi-File Editing](#4-multi-file-editing)
5. [Code Search and Indexing](#5-code-search-and-indexing)
6. [Git Integration Patterns](#6-git-integration-patterns)
7. [File Watching and Change Detection](#7-file-watching-and-change-detection)

---

## 1. Diff/Patch Formats

### 1.1 Unified Diff Format (Standard)

The unified diff format (`diff -U3`) is the industry-standard text representation of code changes, used by Git and patch tools for decades.

**Structure:**
```diff
--- file.py
+++ file.py
@@ -10,7 +10,7 @@
 def some_function():
-    return "old value"
+    return "new value"
```

**How it works:** Hunk headers (`@@ -start,count +start,count @@`) specify line numbers and counts. Context lines surround the changed lines, enabling `patch` to locate changes even with minor offset drift.

**Pros:**
- Universally understood by developers and tools
- Patch tool can apply changes with fuzzy matching
- Human-readable with context
- Standardized across all version control systems

**Cons for LLM-generated edits:**
- **Fragile line numbers**: LLMs frequently generate incorrect hunk headers (line offsets). Research (Cheng et al., 2026) shows all number-indexed diff formats yield "edit accuracies far below" full-code baselines because "LLMs struggle to generate precise line numbers and offsets" -- even when source code has explicit line numbers.
- **Context line mismatches**: When code evolves between LLM reading and editing, context lines fail to match
- **Fragmented hunks**: Break syntactic coherence -- a single function change may be split across multiple hunks
- **Patch application failures**: GNU patch relies heavily on correct hunk headers; LLM-generated diffs frequently fail to apply

**Research finding**: The AdaEdit paper (Cheng et al., 2026) found that MinUniDiff (minimal unified diff) achieved only ~14% pass@1 accuracy vs. ~57% for full-code generation on edit benchmarks. Even standard UniDiff with line numbers only reached ~37.7% average accuracy.

### 1.2 Claude Code: Exact String Replacement

Claude Code uses an `Edit` tool based on **exact string replacement** (str_replace). The LLM provides the exact string to find and the exact replacement string.

**Structure:**
```
Edit file.py
<str_replace>
original_exact_string
=======
replacement_string
</str_replace>
```

**Key characteristics:**
- Requires **unique match** -- the search string must appear exactly once in the file
- Operates on raw text (not line-based)
- Handles multi-line replacements naturally
- Fails with a clear error message if the string is not found uniquely

**Pros:**
- Simple and deterministic
- No line number fragility (content-addressed, not position-addressed)
- Natural for LLMs (similar to how humans describe edits: "find X, replace with Y")
- Fast application (single string search)

**Cons:**
- Fails when the cached file content differs from actual file content
- Whitespace-sensitive -- tabs vs. spaces, trailing whitespace changes cause failures
- After auto-formatting (Prettier, Black, gofmt), cached content no longer matches
- Cannot handle ambiguous matches (same code appearing multiple times)
- Est. 15-20% of edit operations fail with "String to replace not found" on first attempt

**Mitigation strategies:**
- Claude Code auto-detects when `str_replace` fails and can fall back to `Write` (full file rewrite)
- For files under ~400 lines, rewriting the entire file is more reliable than patching
- Hash-based line addressing has been proposed as an alternative (e.g., `line 42:f1`)

### 1.3 Aider: SEARCH/REPLACE Blocks

Aider pioneered the SEARCH/REPLACE block format, using git merge-conflict-like syntax. This is the most widely adopted diff format in AI coding tools.

**Structure:**
```
file.py
<<<<<<< SEARCH
# Exact original code to find
=======
# New replacement code
>>>>>>> REPLACE
```

**How Aider applies edits** (flexible matching strategy):
1. **Exact match** -- literal string comparison
2. **Whitespace-insensitive match** -- ignores leading/trailing whitespace differences
3. **Indentation-preserving match** -- normalizes indentation but preserves structure
4. **Fuzzy match** -- uses `difflib` for similarity scoring when exact matches fail

**Edit formats supported by Aider:**

| Format | Description | Best For |
|--------|-------------|----------|
| `whole` | LLM returns complete updated file | Small files (<300 lines) |
| `diff` | SEARCH/REPLACE blocks | Most models (default for Claude/GPT) |
| `diff-fenced` | Filename inside code fence | Gemini family of models |
| `udiff` | Unified diff format | Complex multi-hunk changes |
| `editor-diff` | Streamlined version | Specific internal modes |

**Pros:**
- Content-addressed (no line numbers), so insensitive to line number drift
- SEARCH block provides verification before replacement
- Human-readable using familiar git conflict syntax
- Aider's flexible matching tolerates minor LLM imperfections
- Efficient token usage compared to full-file rewrite

**Cons:**
- SEARCH block must contain **exact** original lines -- any discrepancy causes failure
- Some models (Gemini, DeepSeek) struggle to follow the format correctly
- SEARCH block can collide with delimiter characters in source code
- Models occasionally include diff markers (`+`, `-`) inside SEARCH blocks
- Diff format accuracy limited to ~70-80% on evolved codebases

**Failure modes observed:**
- Models try to use git-diff-like format inside SEARCH blocks (including `+`/`-` markers)
- Include `@@` line markers in SEARCH blocks
- SEARCH block not found because code was reformatted between read and edit
- Multiple similar patterns cause wrong-match risk

### 1.4 OpenAI Codex CLI: Patch Format

OpenAI developed a structured patch format used by GPT-4.1 and Codex CLI:

```
*** Begin Patch
*** Update File: file.py
@@ class MyClass:
  def some_function():
-     return "old"
+     return "new"
*** End Patch
```

**Key characteristics:**
- Structured with clear begin/end markers
- Can target specific locations with class/function context
- Uses `+`/`-` markers within a known scope
- Aider adopted and enhanced this format with better error handling

### 1.5 RooCode: Middle-Out Fuzzy Matching

RooCode uses SEARCH/REPLACE blocks with an advanced matching strategy:

**Middle-Out Fuzzy Matching:**
1. Estimate search region (possibly using line number hints)
2. Search outward from the center point
3. Score similarity using Levenshtein distance
4. Select the best match above a threshold

**Indentation Preservation System:**
1. Capture original indentation style (spaces/tabs) of matched lines
2. Analyze relative indentation within the replacement block
3. Re-apply original indentation while maintaining relative structure

This is crucial for Python and other indentation-sensitive languages.

### 1.6 Structure-Aware Diff Formats (Research Frontier)

The AdaEdit research (Cheng et al., 2026) introduces **BlockDiff** and **FuncDiff** -- AST-aligned diff formats that represent changes as block-level rewrites of syntactically coherent units.

**BlockDiff:**
- Uses tree-sitter to parse code into AST
- Aligns textual diffs to code blocks (control structures, loops, functions)
- Expands anchor content progressively until contextually unique
- Merges overlapping hunks at shared parent nodes

**FuncDiff:**
- Similar to BlockDiff but operates at function-level granularity
- Broader structural stability (ignores fine-grained control structures)
- Better accuracy on capable models

**AdaEdit adaptive strategy:**
- Trains LLMs to dynamically choose between diff format and full code
- For each sample, selects the more token-efficient representation
- Format selection accuracy exceeds 90%
- Reduces latency and cost by **30%+** on long-code editing while matching full-code accuracy

**Research results:**

| Format | Avg Pass@1 (Qwen2.5-Coder-7B) |
|--------|-------------------------------|
| FullCode (baseline) | 57.07% |
| MinUniDiff | 14.07% |
| UniDiff (w/ numbers) | 37.66% |
| ContentDiff | 54.43% |
| **BlockDiff** | **55.98%** |
| **FuncDiff** | **57.32%** |
| **FuncDiff + AdaEdit** | **57.95%** |

### 1.7 Comparison Summary

| Format | Token Efficiency | Reliability | LLM Compatibility | Best For |
|--------|-----------------|-------------|-------------------|----------|
| Full file rewrite | Low (large files) | High (no patching) | All models | Files <400 lines |
| SEARCH/REPLACE (Aider) | High | Medium-High | Most models | General editing |
| str_replace (Claude Code) | High | Medium | Anthropic models | Simple replacements |
| Unified diff | Medium | Low | Poor | Human review only |
| OpenAI patch | Medium | Medium | GPT-4.1 trained | GPT-4.1/Codex |
| BlockDiff/FuncDiff | High | High (research) | Needs fine-tuning | Future production |

**Key recommendation**: For production systems today, use **SEARCH/REPLACE blocks with flexible matching** (Aider's approach) as the primary format, with **full-file rewrite fallback** for small files. Structure-aware formats (BlockDiff/FuncDiff) represent the research frontier and show promise for 30%+ cost reduction with maintained accuracy.

---

## 2. File Editing Pipeline

### 2.1 Standard Pipeline: Read -> Propose -> Validate -> Show -> Approve -> Apply -> Format -> Test

```
[Read] -> [Propose] -> [Validate] -> [Show] -> [Approve] -> [Apply] -> [Format] -> [Test]
   ^                                                  |
   |<---------------- [Fix] <---------------------------|
```

**Step 1: Read**
- Agent reads the current file content (with line numbers)
- May also read related files (imports, tests, call sites)
- Stores content in context for reasoning

**Step 2: Propose**
- LLM generates the edit in the chosen format (SEARCH/REPLACE, diff, etc.)
- Edit is proposed but not yet applied
- For multi-file changes, all proposals are generated before any are applied

**Step 3: Validate**
- Parse the generated edit to ensure it's well-formed
- Check that SEARCH blocks actually exist in the current file content
- Verify indentation consistency
- Ensure the edit doesn't introduce obvious syntax errors
- **This is the most critical step** -- most failures happen here

**Step 4: Show**
- Present the proposed change to the user (human-in-the-loop)
- Show a diff view: what will change, line by line
- Highlight added/removed/modified lines
- Some systems (Claude Code) support `--auto-accept` for trusted workflows

**Step 5: Approve**
- User reviews and approves/rejects
- Options: accept all, accept per-file, reject, or request modifications
- In CI/automated mode, approval may be conditional on test/lint passing

**Step 6: Apply**
- Write changes to disk
- For SEARCH/REPLACE: find match, verify uniqueness, perform replacement
- Track which edits succeeded and which failed
- Handle partial failures gracefully

**Step 7: Format**
- Run code formatter (Prettier, Black, gofmt, rustfmt)
- Formatting can change line numbers, which may invalidate subsequent edits
- Best practice: apply all edits first, then format once at the end

**Step 8: Test**
- Run relevant tests to verify correctness
- Run linter (pylint, eslint) to catch syntax issues
- Run type checker (mypy, tsc)
- If tests fail, agent reads errors and proposes fixes (loop back to Read)

### 2.2 Failure Modes at Each Step

| Step | Common Failures | Mitigation |
|------|----------------|------------|
| Read | File too large for context; wrong file read | Use offset/limit for large files; validate file path |
| Propose | LLM generates malformed edit; wrong format | Parser validation; format-specific linting; retry |
| Validate | SEARCH block not found; ambiguous match | Flexible matching; fuzzy search; detailed error feedback |
| Show | Diff rendering issues; encoding problems | Unified diff display; handle non-ASCII |
| Approve | User rejects; wants modifications | Conversation loop; partial acceptance |
| Apply | File modified between read and apply | File locking; content hashing; retry with re-read |
| Format | Formatter changes invalidate subsequent edits | Defer formatting until all edits applied |
| Test | Tests fail; linter errors | Auto-retry loop; error feedback to LLM |

### 2.3 How to Validate an Edit Before Applying

**Critical validation checks:**

1. **Search uniqueness**: Verify the SEARCH text appears exactly once (or use fuzzy matching score)
2. **Content recency**: Compare a hash of current file content with the content the LLM saw
3. **Syntax preservation**: After replacement, verify the file still parses (for structured languages)
4. **Indentation check**: Ensure replacement maintains consistent indentation
5. **Boundary checks**: Verify the edit doesn't break surrounding code structure

**Best practices:**
- Keep a SHA-256 hash of file content at read time; re-hash before applying
- If hash mismatch, re-read file and regenerate edit
- Validate edits in-memory before writing to disk
- For SEARCH/REPLACE, test the search first, only then perform replacement

### 2.4 Handling Indentation and Whitespace

**The whitespace problem:**
- Python is especially sensitive to indentation
- Tabs vs. spaces cause mismatches
- Auto-formatters (Prettier, Black) change whitespace after edits
- Trailing whitespace differences break exact string matching

**Solutions from production tools:**

| Tool | Approach |
|------|----------|
| Aider | 4-tier matching: exact -> whitespace-insensitive -> indentation-preserving -> fuzzy (difflib) |
| RooCode | Capture original indentation + analyze relative indentation + re-apply original style |
| AdaEdit | Normalize with Black before processing to eliminate non-semantic whitespace differences |

**Best practices:**
1. Normalize files with the project's formatter before editing
2. In SEARCH blocks, match both tabs and spaces flexibly
3. In REPLACE blocks, use the original file's indentation style
4. Defer formatting until all edits are complete
5. For Python, verify AST parsability after each edit

### 2.5 Benchmark: Editing Strategy Comparison

Research (Dev.to, 2026) benchmarked 5 strategies on a 1053-line file with 10 changes:

| Strategy | Tokens | Duration | Tool Calls | Notes |
|----------|--------|----------|------------|-------|
| Script Generation | 7,000 | 10s | 2 | Agent writes sed script; 3.5x cheaper than sequential |
| Unified Diff | 8,500 | 12s | 2 | Standard patch format |
| Sequential Edit | 25,000 | 65s | 11 | One Edit call per change; line drift issues |
| Bottom-up Edit | 25,000 | 65s | 11 | Applies from bottom to top; eliminates line drift |
| Atomic Write | 43,000 | 50s | 2 | Full file rewrite; "lost in the middle" problem |

**Decision table:**

| | 1-2 changes | 3-5 changes | 6+ changes |
|---|-------------|-------------|------------|
| < 300 lines | Edit tool | Script/Diff | Script |
| 300-1000 lines | Edit tool | Script/Diff | Script |
| > 1000 lines | Edit tool | Script | Script |

---

## 3. AST-Aware Editing

### 3.1 Tree-sitter Based Editing

Tree-sitter is an incremental parsing library that builds Concrete Syntax Trees (CSTs) from source code. It's the foundation of syntax highlighting in Neovim, Helix, Zed, and other editors.

**Why tree-sitter matters for AI coding agents:**
- Parses 100+ programming languages with a unified API
- Recovers from syntax errors gracefully (can parse incomplete code)
- Produces precise source locations (line/column for every node)
- Incremental parsing is fast for large files
- Maps cleanly between AST nodes and source text positions

**Key capabilities:**
- Extract function/class definitions and references
- Build call graphs and dependency graphs
- Identify scopes and variable bindings
- Chunk code at meaningful boundaries (functions, classes, blocks)
- Transform code by manipulating AST nodes

**Usage pattern:**
```python
from tree_sitter_language_pack import process

result = process("path/to/file.py")
for chunk in result.chunks:
    print(chunk.type)      # "function", "class", "import_block"
    print(chunk.content)   # the actual code
    print(chunk.metadata)  # language, line range, parent scope
```

### 3.2 AST Diff (Structural Diff)

**Difftastic**: A structural diff tool that parses code into syntax trees using tree-sitter and produces human-readable diffs at the expression level.

- Supports 30+ languages
- Aligns code structure rather than lines
- Highlights moved code segments and renamed variables
- Falls back to word-based text diff for unknown files
- **Limitation**: No patch generation or merging -- focused on human understanding

**astdiff**: An AST-based structural diff for JavaScript
- Uses MinHash signatures and structural fingerprinting
- Matches renamed functions and variables in minified/obfuscated code
- Tree edit distance algorithms with parallel processing

**Diff/AST**: A fine-grained source code differencing tool
- Compares ASTs node by node (not line by line)
- Based on tree edit distance (TED) algorithms
- Supports Python, Java, Verilog, Fortran, C/C++
- Exports changes as facts in XML or N-Triples

**How AST diff works:**
1. Parse both versions into ASTs
2. Compute tree edit distance between ASTs
3. Map nodes between versions using structural similarity
4. Report changes as additions, deletions, moves, and renames
5. Generate a diff that respects code structure

### 3.3 When AST-Aware Editing is Better Than Line-Based

| Scenario | Line-based diff | AST-based diff |
|----------|----------------|----------------|
| Renamed variable across function | Shows as many line changes | Shows as single rename operation |
| Reordered methods in class | Shows all lines as changed | Shows as move operations |
| Reformatted code (same logic) | Massive diff noise | No structural changes detected |
| Minified/obfuscated code | Unreadable diff | Structural matching works |
| Import statement reordering | Line changes | Ignored (not structural) |
| Complex nested expression | Hard to read | Expression-level comparison |

**Best use cases for AST-aware editing:**
- Refactoring operations (rename, extract method, move)
- Code review (understand semantic changes)
- Minified or generated code
- Multi-language codebases (unified approach)
- When formatting changes obscure semantic changes

**Limitations:**
- Slower than line-based diff (parsing overhead)
- Requires language-specific grammar support
- Tree edit distance is computationally expensive (quadratic in worst case)
- Not suitable for non-code files (documentation, configs)
- Patching infrastructure less mature than text-based patch

### 3.4 Language-Aware Refactoring

**ast-grep**: A CLI tool for AST-based structural search and replace
- Search code by AST patterns, not text
- Works across languages using tree-sitter grammars
- Refactoring with confidence (structural transformation)
- Example: find all `console.log` calls and replace with a logger

**CodeRLM**: Tree-sitter-backed code indexing for LLM agents
- Rust server indexes projects with tree-sitter
- Builds symbol table with cross-references
- API for: `init`, `structure`, `search`, `impl`, `callers`, `grep`
- Replaces glob/grep/read cycle with index-backed lookups

**Integration patterns:**
1. Parse files with tree-sitter on load/index
2. Use AST for: chunking, symbol extraction, call graph
3. Use text search for: quick lookups, grep, regex patterns
4. Use AST diff for: code review, refactoring suggestions
5. Fall back to text diff for: final edit application (more reliable)

---

## 4. Multi-File Editing

### 4.1 Atomic Multi-File Changes

An atomic change either applies completely or not at all -- no partial application.

**Why atomicity matters:**
- A refactoring that changes an interface must update all implementors
- Renaming a function requires updating all call sites
- Adding a parameter requires updating all invocations
- Partial application leaves codebase in broken state

**Implementation strategies:**

**Strategy A: Staged Application**
1. Generate all edits for all files in memory
2. Validate all edits can be applied
3. Apply all edits (or none, if any fails)
4. Rollback on failure

**Strategy B: Git-based Atomicity**
1. Create a checkpoint commit before changes
2. Apply edits file by file (allowing partial application)
3. Run tests/lint to verify
4. If verification fails: `git reset --hard` to checkpoint
5. If verification passes: keep changes, optionally squash

**Strategy C: Transaction Log**
1. Write all edits to a transaction log before applying
2. Apply edits sequentially, tracking each
3. On failure: replay inverse operations from log to rollback
4. On success: clear log

**Recommended approach**: Combine B and C -- git checkpoint for coarse rollback, transaction log for fine-grained undo within a session.

### 4.2 Dependency Ordering

When editing multiple files, the order matters:

**Approaches to ordering:**

1. **Bottom-up** (implementation first):
   - Edit leaf functions/utilities first
   - Edit callers/consumers last
   - Tests can be run incrementally
   - Risk: interface changes force re-editing

2. **Top-down** (interface first):
   - Edit public APIs/interfaces first
   - Edit implementations to match
   - Ensures consistent interfaces
   - Risk: temporary breakage during editing

3. **Dependency graph order**:
   - Build a DAG of file dependencies
   - Process in topological order
   - Most principled but requires accurate dependency analysis

4. **Test-driven order**:
   - Edit tests first (they define expected behavior)
   - Edit implementation to pass tests
   - Edit related files to fix integration issues

**Best practice**: Use dependency graph analysis (via imports, symbol references) to determine order. Within a strongly-connected component, use test-driven order.

### 4.3 Partial Failure Handling

**Common failure scenarios:**
- Edit 1/5 succeeds, Edit 2/5 fails (SEARCH not found)
- All edits apply but tests fail for edit 3
- File was modified externally between read and edit
- Formatter changed file content after edit 1, breaking edit 2

**Handling strategies:**

| Strategy | Behavior | Trade-off |
|----------|----------|-----------|
| All-or-nothing | Rollback all if any fails | Safe but may discard good work |
| Continue-on-fail | Apply what succeeds, report failures | May leave codebase inconsistent |
| Retry-failed | Re-read failed files and retry edits | Uses extra tokens/time |
| Human-escalation | Stop and ask user how to proceed | Interrupts automation |

**Best practices:**
1. Validate all edits before applying any (pre-flight check)
2. For each edit: attempt -> verify -> commit individually
3. If an edit fails: skip it, continue with others, report at end
4. Provide detailed error feedback for failed edits (show what was expected vs. what was found)
5. Allow agent to re-read failed files and regenerate edits

### 4.4 Rollback Strategies

**Git-based rollback:**
```bash
# Before any edits: create checkpoint
git stash push -m "pre-agent-checkpoint"  # or: git commit -m "checkpoint"

# Apply edits...

# If things go wrong: restore
git stash pop    # or: git reset --soft HEAD~1
```

**Claude Code checkpoint pattern:**
- Some users create `.claudecheckpoints/` directory (separate git repo)
- Auto-commit after every file edit using PostToolUse hooks
- Each checkpoint commit shows: tool used, file modified, timestamp
- Rollback: `git revert` any individual change

**Hermes agent checkpoint proposal:**
```yaml
checkpoint:
  enabled: true
  mode: "git-commit"  # or "git-stash", "file-backup"
  auto_init_repo: true
  squash_on_clean_exit: true
  max_checkpoints_per_session: 50
```

**Rollback triggers:**
- User types `/undo` or `/revert`
- SIGINT/SIGTERM during editing
- Test/lint failure after edits
- User explicitly requests revert
- File hash mismatch detected (external modification)

**Best practice workflow for multi-file changes:**
1. Read all target files and their hashes
2. Generate all edits
3. Validate all edits against current file content
4. Create git checkpoint (stash or commit)
5. Apply edits one by one (bottom-up for multi-change)
6. Run formatter once after all edits
7. Run tests/lint
8. If failure: `git reset --hard` to checkpoint
9. If success: commit with descriptive message

---

## 5. Code Search and Indexing

### 5.1 Ripgrep Integration for Fast Text Search

**ripgrep (`rg`)** is the standard search tool for AI coding agents. It's a line-oriented search tool that recursively searches directories for regex patterns.

**Why ripgrep:**
- Extremely fast (parallel search, respects .gitignore)
- Supports regex with full Unicode
- Searches compressed files, respects ignore patterns
- Cross-platform, single binary
- Default for: Claude Code, Gemini CLI, Codex CLI, Cline

**Usage patterns in AI agents:**

| Pattern | Example | Purpose |
|---------|---------|---------|
| Find function definition | `rg "^def function_name"` | Locate implementation |
| Find all references | `rg "function_name"` | Find usages |
| Find imports | `rg "from module import"` | Trace dependencies |
| Find class | `rg "class ClassName"` | Locate type definition |
| Grep with context | `rg -A 5 -B 5 "pattern"` | Get surrounding code |
| Type filter | `rg "pattern" --type py` | Language-specific search |

**Limitations:**
- Text-only (no semantic understanding)
- Regex can be brittle across coding styles
- Doesn't understand code structure (can't find "all methods of class X")
- No ranking of results -- returns all matches

### 5.2 Symbol Indexing (ctags, LSP, Tree-sitter)

**ctags:**
- Generates index of language objects (functions, classes, variables)
- Supports 40+ programming languages
- Fast, lightweight, external tool
- **Limitation**: Only provides definition locations, not usages or relationships

**LSP (Language Server Protocol):**
- Go-to-definition, find-references, workspace symbol search
- Deep language understanding via compiler-grade analysis
- Used by IDEs (VS Code, etc.)
- **Challenges for agents**: Requires language server setup, slower startup, stateful connections

**Tree-sitter based indexing (Aider's approach):**
- Parses all files with tree-sitter
- Extracts definitions and references
- Builds directed graph of symbol relationships
- Cached in SQLite with mtime-based invalidation
- Supports 40+ languages
- Deterministic and offline (no GPU/external deps needed)

**Comparison:**

| Feature | ctags | LSP | Tree-sitter |
|---------|-------|-----|-------------|
| Definitions | Yes | Yes | Yes |
| References | No | Yes | Yes |
| Call graph | No | Yes | Yes |
| Type info | Limited | Full | Limited |
| Setup | Easy | Complex | Easy |
| Speed | Fast | Medium | Fast |
| Incremental | No | Yes | Yes |
| Offline | Yes | Partial | Yes |

### 5.3 Semantic Search (Embeddings-Based)

**How it works:**
1. Chunk code into meaningful units (functions, classes)
2. Generate vector embeddings for each chunk (using code-specific models)
3. Store in vector database (FAISS, etc.)
4. At query time: embed query, find nearest neighbors

**Tools:**
- **FAISS** (Facebook AI Similarity Search): Fast nearest-neighbor search for embeddings
- **SentenceTransformers**: Generate code embeddings (`all-MiniLM-L6-v2`, code-specific models)
- **Cursor/Windsurf**: Use hybrid semantic-lexical indexing with background indexing

**Trade-offs:**

| Pros | Cons |
|------|------|
| Finds conceptually similar code | Requires GPU for embedding generation |
| Natural language queries | Index must be kept in sync with code changes |
| Cross-language similarity | "Cold start" -- initial indexing overhead |
| Handles synonyms and paraphrases | Not deterministic -- same query may yield different results |

**Hybrid approach (best practice):**
- Use semantic search for natural language queries ("find authentication code")
- Use ripgrep for exact/regex queries ("find all uses of `validateToken`")
- Use tree-sitter index for structural queries ("find all methods of class AuthService")

### 5.4 Aider's Repo-Map: Graph-Based Ranking

Aider's most innovative contribution: a **PageRank-based repository map** that automatically selects relevant context.

**How it works:**

**Phase 1: Symbol Extraction (tree-sitter)**
- Parse all files with tree-sitter
- Run `.scm` query files to extract: `@name.definition.X` (definitions), `@name.reference.X` (references)
- Cache tags in SQLite with mtime invalidation

**Phase 2: Graph Building**
- Nodes = files
- Edges = symbol references (file A references symbol defined in file B)
- Edge weights use multiple heuristics:
  - 50x boost for references FROM files currently being edited
  - 10x boost for user-mentioned identifiers
  - 10x boost for well-named identifiers (snake_case/camelCase, >=8 chars)
  - 0.1x penalty for private symbols (starting with `_`)
  - 0.1x penalty for symbols defined in >5 files (too common)

**Phase 3: Personalized PageRank**
```python
personalize = {fname: 100 / len(fnames) for fname in chat_fnames}
ranked = nx.pagerank(G, weight="weight", personalization=personalize)
```

**Phase 4: Scope-Aware Rendering**
- Binary search to fit top-ranked symbols within token budget
- Show function signatures + parent scope headers
- Elide implementation bodies with `...` markers
- Small gaps between shown lines are filled in

**Performance:**
- Repo scanning: ~240 files/second
- Graph computation: fast for typical repos
- Token efficiency: 4.3-6.5% context utilization (vs. 17.5% for Cline, 14.7% for Cursor)
- Lowest token usage among evaluated agents: 8.5k-13k tokens per task

### 5.5 File Ranking / Relevance Scoring

**Approaches to ranking files for context:**

| Approach | Used By | Mechanism | Strengths |
|----------|---------|-----------|-----------|
| Graph ranking (PageRank) | Aider | Symbol reference graph centrality | Captures transitive importance; architecture-aware |
| Lexical search | Claude Code | ripgrep on-demand | Fresh, no stale index; transparent |
| Semantic similarity | Cursor | Vector embeddings | Natural language queries; conceptual matches |
| Recency + frequency | Git | Recently modified files | Likely relevant to current work |
| Directory proximity | Multiple | Files near edited files | Related code often co-located |

**Token budget management:**
- Claude Code: No persistent index; agentic search per task (higher tokens but always fresh)
- Aider: Binary search to fit repo-map within `max(1024, min(max_input_tokens/8, 4096))`
- Cursor: Persistent hybrid index with incremental sync
- Cline: Three-tier retrieval (ripgrep + fzf + AST) with 300 result limit

### 5.6 How to Find Relevant Files in Large Codebases

**Progressive narrowing strategy:**
1. **Directory structure** (`ls`, `tree`) -- understand top-level organization
2. **File search by name** (`find`, `glob`) -- locate likely files
3. **Text search** (`ripgrep`) -- find mentions of relevant symbols
4. **Symbol search** (tree-sitter index) -- find definitions and references
5. **Semantic search** (embeddings) -- find conceptually related code
6. **Call graph traversal** (LSP/tree-sitter) -- trace execution paths

**Tips for large codebases (>10k files):**
- Use `.aiderignore` or `.claudeignore` to exclude generated files, vendor dirs
- Start with repo-map / symbol index for structural overview
- Use ripgrep with `--type` filters to narrow by language
- Read only relevant files, in chunks (offset/limit for large files)
- Cache file content hashes to detect external changes
- For very large repos, use subtree-only mode

---

## 6. Git Integration Patterns

### 6.1 Pre-Edit Checkpoint

**Purpose**: Ensure a known-good state before the agent makes any changes.

**Implementation options:**

| Method | Command | When to Use |
|--------|---------|-------------|
| Git stash | `git stash push -m "pre-agent-checkpoint"` | Has uncommitted changes |
| Git commit | `git commit -am "checkpoint before agent"` | Clean working tree |
| Branch | `git checkout -b agent/task-001` | Isolated work |
| Snapshot | Custom file copy to `.agent_backups/` | Not in git repo |

**Claude Code checkpoint hooks:**
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [{
          "type": "command",
          "command": "cd \"$CLAUDE_PROJECT_DIR\" && git add -A && git diff-index --quiet HEAD || git commit -m \"checkpoint\" 2>/dev/null || true"
        }]
      }
    ]
  }
}
```

### 6.2 Atomic Commits Per Change

**Principle**: Each logical unit of work = one commit.

**Benefits:**
- `git revert HEAD` undoes exactly one change
- `git bisect` can pinpoint problematic changes
- Clean history for code review
- Easy rollback of specific changes

**Implementation:**
```bash
# After agent completes a task:
git add -A
git commit -m "feat: add JWT rotation support

- Refactor auth middleware to accept multiple signing keys
- Update config to support array of keys
- Maintain backward compatibility with single-key config

Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

**Commit frequency recommendations:**
- **After each completed task** (for long-running agents)
- **After each file edit** (for maximum granularity/safety)
- **After test pass** (only commit verified changes)
- **Never auto-push** (always keep local until human review)

### 6.3 Automatic Commit Message Generation

**Claude Code approach:**
- Analyzes `git diff` to understand what changed
- Reads recent commit history to learn project conventions
- Generates Conventional Commits format: `type(scope): description`
- Follows rules in `CLAUDE.md` if present

**Example generated messages:**
```
feat(auth): add email and phone number format validators

Adds two new utility functions — isValidEmail and isValidPhone —
with corresponding regex patterns. Includes test coverage.

fix(api): resolve authentication bug in login flow

Fixes the issue where users were being logged out unexpectedly
after session timeout.

refactor(middleware): consolidate duplicate token validation
```

**Aider approach:**
- Adds `(aider)` to author name
- Includes the model as co-author
- Generates description from the conversation context

**Best practices for auto-generated commits:**
1. Include `Co-Authored-By` trailer for transparency
2. Follow project conventions (read from CLAUDE.md or git history)
3. Include body explaining *why*, not just *what*
4. Allow user to review/edit before committing
5. Don't auto-push -- keep commits local for review

### 6.4 Branch Management

**Recommended branch strategy for AI agents:**

```
main/master (protected)
  |
  +-- agent/task-001-refactor-auth
  |     +-- commit 1: refactor middleware
  |     +-- commit 2: update config
  |     +-- commit 3: add tests
  |
  +-- agent/task-002-fix-memory-leak
  |     +-- commit 1: identify leak source
  |     +-- commit 2: implement fix
  |
  +-- agent/task-003-add-metrics
        +-- commit 1: add instrumentation
```

**Naming convention:**
- Prefix: `agent/` or `ai/` for clear identification
- Include task number and brief description
- Lowercase with hyphens

**Workflow:**
1. Create branch from latest main: `git checkout -b agent/task-NNN-description`
2. Agent works on branch, committing as it goes
3. Run tests/lint on branch
4. Human reviews branch diff
5. Merge via PR or direct merge
6. Delete branch after merge

### 6.5 Diff Review Before Commit

**Human review workflow:**
```bash
# After agent completes edits:
git diff --stat          # Overview of changed files
git diff                 # Full diff review
git add -p               # Stage changes interactively (review each hunk)
git commit -m "..."      # Commit with generated message
```

**Automated review gates:**
```bash
# Safety checks:
git diff HEAD            # See all changes
# Scan for: hardcoded secrets, AWS keys, tokens, passwords
# Run: linters (ruff, eslint, golangci-lint)
# Run: type checkers (mypy, tsc)
# Run: tests
# If any fail: STOP, show output, do NOT commit
```

**CLAUDE.md rules for Git:**
```markdown
## Git Workflow
- Create commits after completing each logical unit of work
- Do not push to remote unless explicitly asked
- Use conventional commits (feat:, fix:, refactor:, docs:)
- Never commit: secrets, API keys, .env files, build artifacts
- Run tests before committing
- Write descriptive commit messages with body explaining why
```

---

## 7. File Watching and Change Detection

### 7.1 Detect External File Changes

**Why it matters:**
- Auto-formatter (Prettier, Black) runs on save, changing file content
- User edits file in IDE while agent is working
- Build process generates/modifies files
- Git operations change file state

**Detection mechanisms:**

| Mechanism | How | Pros | Cons |
|-----------|-----|------|------|
| Content hashing (SHA-256) | Hash file before/after | Reliable; catches any change | Requires reading file |
| File modification time (mtime) | Check `stat` timestamps | Fast; no content read | Granularity limited; may miss rapid changes |
| File system watcher (chokidar/fsnotify) | OS-level event notifications | Real-time; efficient | Platform-specific; may miss events |
| Git diff | `git diff --name-only` | Integrates with version control | Only tracks tracked files |

**Best approach for agents: Content hashing**
```python
import hashlib

def file_hash(path):
    with open(path, 'rb') as f:
        return hashlib.sha256(f.read()).hexdigest()

# Before editing:
original_hash = file_hash("file.py")

# ... LLM generates edit ...

# Before applying:
if file_hash("file.py") != original_hash:
    # File changed! Re-read and regenerate edit
    raise FileChangedError("File was modified externally")
```

### 7.2 Handle Conflicts Between Agent Edits and External Edits

**Conflict scenarios:**
1. **Formatter ran**: Black/Prettier changed whitespace; agent's SEARCH no longer matches
2. **User edited same file**: User and agent both modified the file
3. **Build process modified file**: Generated code changed
4. **Git operations**: Branch switch, pull, merge changed files

**Resolution strategies:**

| Strategy | When | How |
|----------|------|-----|
| Re-read and retry | File changed before apply | Re-read file, regenerate edit |
| Defer formatting | Formatter conflict | Apply all edits first, then format once |
| Lock files | Prevent concurrent edits | Advisory file locks |
| Queue edits | User is actively editing | Wait for user idle time |
| Snapshot comparison | Detect what changed | Compare hashes, show diff to user |

**Best practices:**

1. **Hash check before every edit**: Verify file hasn't changed since read
2. **Defer formatting**: Don't run formatter between edits in a multi-edit operation
3. **Bottom-up ordering**: For multi-change edits, apply from bottom of file to top to minimize line number shifts
4. **Format-once at the end**: Apply all raw edits, then run formatter, then run tests
5. **Graceful degradation**: If a file changed, re-read it rather than failing
6. **User notification**: Inform user when external changes are detected

**Scratch Security's approach (IDE integration):**
- Real-time change detection via multiple mechanisms:
  - Document change events (normal edits)
  - File system watcher (external/agent edits)
  - Periodic polling (fallback)
- Content hash comparison (SHA-256) to prevent duplicate processing
- Efficient TOON-format diff capture (30-50% token reduction)
- Prevents infinite loops from self-triggered changes

### 7.3 Practical Recommendations

**For agent builders:**
1. Always hash files before editing; re-check before applying
2. Support re-read-and-retry for stale content
3. Defer formatting until all edits are complete
4. Use bottom-up edit ordering for multi-change operations
5. Provide clear error messages when conflicts are detected
6. Implement graceful fallback (full-file rewrite) when SEARCH/REPLACE fails

**For users of AI coding agents:**
1. Disable auto-format-on-save while agent is working (or ensure agent handles it)
2. Let agent complete its task before making manual edits
3. Use git branches for agent work (isolated from your work)
4. Review diffs before committing agent changes
5. Run your own test suite after agent completes work

---

## Appendix: Tool Comparison Matrix

| Dimension | Claude Code | Aider | Cursor | Codex CLI | Cline |
|-----------|-------------|-------|--------|-----------|-------|
| **Edit Format** | str_replace | SEARCH/REPLACE | IDE native | OpenAI patch | SEARCH/REPLACE |
| **Search** | ripgrep (on-demand) | Repo-map (PageRank) | Hybrid (embeddings+grep) | Shell commands | ripgrep + fzf + AST |
| **Indexing** | None (agentic search) | Tree-sitter + graph | Persistent hybrid index | None | Tree-sitter (top-level) |
| **Context Strategy** | Explicit tools + Bash | Repo-map auto-context | Background indexing + explicit | Explicit tools | Plan-and-act loop |
| **Token Efficiency** | High | Highest (4.3-6.5%) | Moderate (14.7%) | Lowest | Moderate (17.5%) |
| **Git Integration** | Native (auto-commit hooks) | Native (atomic commits) | IDE integrated | Via shell | Via IDE |
| **Multi-file** | Task tool (sub-agents) | Sequential with validation | Composer (multi-file) | Sequential | Sequential |
| **Transparency** | High (full tool output) | High | Partial (aggregated) | High | High |

---

## Key Takeaways

1. **SEARCH/REPLACE blocks** (Aider's approach) with flexible matching is the most practical edit format for production LLM coding agents today -- balancing token efficiency and reliability.

2. **Full-file rewrite** remains the most reliable option for files under ~400 lines, despite higher token cost.

3. **Structure-aware diff formats** (BlockDiff, FuncDiff) from the AdaEdit research represent the frontier, promising 30%+ cost reduction with maintained accuracy when models are fine-tuned for them.

4. **Content-addressed editing** (no line numbers) is dramatically more reliable than line-number-based formats (unified diff) for LLM-generated edits.

5. **Tree-sitter + graph ranking** (Aider's repo-map) achieves the best token efficiency for codebase navigation without requiring embeddings or vector databases.

6. **Git checkpointing** (stash/commit before edits) is essential for safe agent operation -- every edit session should start from a known-good state.

7. **Validate before applying** -- pre-flight validation of all edits catches most failures before they corrupt files.

8. **Format once at the end** -- running formatters between edits is the #1 cause of edit failures in production agents.

---

*Research compiled from: Aider documentation and source code, Claude Code documentation, AdaEdit research paper (Cheng et al., 2026), "To Diff or Not to Diff" paper, Difftastic documentation, tree-sitter documentation, production agent benchmarks, and community reports from users of Claude Code, Aider, Cursor, Codex CLI, Cline, and RooCode.*
