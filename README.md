# Phronesis

<p align="center">
  <a href="https://github.com/IYEN-AI/Phronesis/stargazers"><img src="https://img.shields.io/github/stars/IYEN-AI/Phronesis?style=social" alt="GitHub stars" /></a>
  <a href="https://github.com/IYEN-AI/Phronesis/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License" /></a>
</p>

> **"To know is to be able to act."** — A filesystem-based agentic memory system where knowledge is stored as executable action units, inspired by Aristotle's Phronesis (practical wisdom) and the unity of knowledge and action.

Phronesis is an MCP server that gives AI agents **autonomous, self-managed long-term memory**. Each agent owns one Phronesis instance. Experiences are recorded as action files in a hierarchical filesystem, retrieved via hybrid search (semantic embedding + grep), and accumulated over time — never deleted.

No API key required. Ships with a local multilingual embedding model (100+ languages including Korean, Japanese, Chinese, and English).

## Why Phronesis?

Traditional agent memory systems store flat key-value pairs or vector chunks. Phronesis takes a fundamentally different approach:

- **Knowledge = Action.** Every memory file is named as an action sentence (`send_apology_with_empathy.jsonl`), making retrieval synonymous with deciding what to do.
- **Filesystem = Mind.** The directory tree *is* the agent's cognitive structure. Folder depth represents contextual specificity. The agent builds and reorganizes this structure through experience.
- **Append-only = Irreversible experience.** Like human memory, nothing is deleted. Past actions and their outcomes accumulate as JSONL trajectories, giving the agent a richer basis for future decisions.
- **Hybrid search = Two-stage reasoning.** Embedding search answers "What situation am I in?" (context), then grep answers "What should I do?" (action selection) — mirroring how humans first orient, then act.

## What shipped in v0.1

- **10 MCP tools** — full CRUD, hybrid search, evolution, habit formation, and self-reflection
- **6-Pillar MECE taxonomy** — self, perception, cognition, praxis, evolution, reflection
- **Local multilingual embedding** — fastembed MultilingualE5Small (384 dims, ONNX, no API key)
- **Optional OpenAI embedding** — set `OPENAI_API_KEY` to upgrade to `text-embedding-3-small`
- **In-process HNSW index** — instant-distance with disk persistence, rebuildable from metadata
- **Append-only JSONL** — crash-safe defensive parsing, no delete APIs
- **Soft naming validation** — warnings logged to `/reflection/warnings/` for agent self-review
- **skills.md hybrid guide** — immutable seed section + agent-appendable learning section
- **61 tests** — 47 unit + 14 integration, mapped 1:1 to acceptance criteria

## Philosophical foundation

Phronesis maps Aristotelian concepts to system components:

| Component | Role | Aristotelian parallel |
|-----------|------|-----------------------|
| Filesystem | Skeleton of thought and action | Reason (Logos) |
| Embedding search | Flexible connection of context and intent | Intuition (Nous) |
| Grep search | Precise selection of concrete actions | Practical wisdom (Phronesis) |
| Symlinks | Automation of repeated actions | Habit (Ethos) |
| Append-only | Irreversibility of experience | Arrow of time |
| Reflection tools | Self-review and structural improvement | Contemplation (Theoria) |

## Architecture

```
┌───────────────────────────────────────────────────┐
│                 MCP Protocol (stdio)               │
├───────────────────────────────────────────────────┤
│            Phronesis MCP Server (Rust)             │
│                                                    │
│  ┌─────────────┐ ┌─────────────┐ ┌──────────────┐ │
│  │  Embedding   │ │    Grep     │ │     CRUD     │ │
│  │  (local or   │ │  (filename  │ │  (append-    │ │
│  │   OpenAI)    │ │  + content) │ │   only)      │ │
│  └──────┬───────┘ └──────┬──────┘ └──────┬───────┘ │
│         └────────────────┼───────────────┘         │
│               ┌──────────┴──────────┐              │
│               │   6-Pillar FS       │              │
│               │   (.jsonl files)    │              │
│               └─────────────────────┘              │
│  ┌──────────────┐  ┌─────────────────────┐         │
│  │  HNSW Index  │  │  Warning Log        │         │
│  │  (in-memory  │  │  (/reflection)      │         │
│  │  + disk)     │  │                     │         │
│  └──────────────┘  └─────────────────────┘         │
├───────────────────────────────────────────────────┤
│  Abstraction layers:                               │
│  - EmbeddingProvider (Local / OpenAI / custom)     │
│  - VectorStore (HNSW / swappable)                  │
└───────────────────────────────────────────────────┘
```

## 6-Pillar taxonomy

```
/phronesis-root/
├── skills.md            # Usage guide (immutable seed + agent-appendable section)
├── self/                # Self-maintenance, identity, system state
├── perception/          # Situation awareness, data interpretation
├── cognition/           # Internal reasoning, knowledge processing
├── praxis/              # External actions, tool use, communication
├── evolution/           # New capability acquisition, structure expansion
└── reflection/          # Self-review, knowledge reorganization
    └── warnings/        # Naming violation logs for agent self-reflection
```

Each pillar directory contains a `.meta.jsonl` file with a short description used as the embedding source. Subdirectories are created by the agent as it accumulates experience.

## MCP tools

| Tool | Description | Category |
|------|-------------|----------|
| `embed_search` | Semantic folder search — "What situation am I in?" | Search |
| `grep_search` | Filename + content regex search — "What should I do?" | Search |
| `read_action` | Read full trajectory (all JSONL entries) of an action file | CRUD |
| `write_action` | Append a new experience entry (with naming validation) | CRUD |
| `suggest_location` | Recommend folder locations for a new action | CRUD |
| `create_folder` | Create a new context folder + register embedding | CRUD |
| `move_action` | Move file/folder + auto-update embedding index | Evolution |
| `rename_action` | Rename file + naming validation + warning log | Evolution |
| `create_habit` | Create symlink shortcut to frequently used actions | Habit |
| `get_warnings` | Query naming violation warnings for self-reflection | Reflection |

## Quick start

### Prerequisites

- Rust toolchain (1.75+)
- macOS or Linux (symlink habits require Unix)

### Build

```bash
git clone https://github.com/IYEN-AI/Phronesis.git
cd Phronesis
cargo build --release
```

### Configure

```bash
# Required: where the agent's memory filesystem lives
export PHRONESIS_DATA_ROOT="$HOME/.phronesis"

# Optional: set to use OpenAI embeddings instead of local model
# export OPENAI_API_KEY="sk-..."
# export PHRONESIS_EMBEDDING_MODEL="text-embedding-3-small"
```

Without `OPENAI_API_KEY`, Phronesis uses **MultilingualE5Small** (local ONNX model, ~100MB auto-download on first run). This supports 100+ languages out of the box.

### Run standalone

```bash
cargo run --release
```

The server communicates via MCP over stdio. On first launch it:
1. Creates the 6-Pillar directory structure
2. Seeds `skills.md` with the usage guide
3. Downloads the embedding model (if using local)
4. Builds the initial HNSW index from folder metadata

### Connect to Claude Code

Add to your Claude Code settings (`.claude/settings.json`):

```json
{
  "mcpServers": {
    "phronesis": {
      "command": "/path/to/Phronesis/target/release/phronesis",
      "env": {
        "PHRONESIS_DATA_ROOT": "/path/to/.phronesis"
      }
    }
  }
}
```

### Connect to any MCP-compatible agent

Phronesis speaks standard MCP over stdio. Any agent framework that supports MCP tool servers can connect:

```bash
# The binary reads MCP JSON-RPC from stdin, writes to stdout
/path/to/phronesis
```

## Agent usage flow

A typical interaction cycle:

```
1. Agent has an intent: "How should I respond to an angry user?"

2. embed_search("responding to angry user")
   → Returns: /praxis/communication (similarity: 0.87)

3. grep_search("praxis/communication", "angry|upset|complaint")
   → Returns: empathize_then_resolve.jsonl (3 trajectory entries)

4a. If found → read_action("praxis/communication/empathize_then_resolve.jsonl")
    → Agent references past experience and applies proven pattern

4b. If not found → suggest_location("empathize with angry user and propose solution")
    → Agent picks best folder, then:
    write_action("praxis/communication/empathize_with_angry_user.jsonl", {
      "ts": "2025-04-14T10:00:00Z",
      "situation": "User expressed frustration about service delay",
      "reasoning": "Emotional acknowledgment should precede problem-solving",
      "action": "Expressed empathy, acknowledged the delay, offered concrete resolution",
      "outcome": "User calmed down and accepted the proposed solution"
    })

5. Next time the same situation arises → step 3 finds the file immediately
```

### Reflection cycle

When the agent detects it's repeating the same action 3+ times in a session:

```
1. get_warnings()
   → Review naming violations and repeated patterns

2. Analyze: "I keep searching /praxis/communication for similar things"

3. create_folder("praxis/communication/conflict_resolution",
                  "Handling disagreements, complaints, and emotional situations")

4. move_action("praxis/communication/empathize_with_angry_user.jsonl",
               "praxis/communication/conflict_resolution/empathize_with_angry_user.jsonl")
   → Embedding index automatically updated
```

## Naming conventions

### Folders
- Lowercase with underscores, noun-oriented context
- Examples: `communication/formal_email`, `debugging/runtime_errors`

### Files
- Pattern: `verb_object_method.jsonl`
- Examples: `send_apology_with_empathy.jsonl`, `analyze_error_logs_systematically.jsonl`
- Violations produce a **warning** (returned to agent + logged) but the write still succeeds
- Warnings accumulate in `/reflection/warnings/naming_violations.jsonl` for agent self-review

## Data model

### Action files (.jsonl)

Each file is an append-only JSONL log. Each line is a complete JSON object with a mandatory `ts` field:

```jsonl
{"ts":"2025-04-14T10:00:00Z","situation":"New user connected","reasoning":"First impression matters","action":"Warm greeting with help offer","outcome":"User engaged positively"}
{"ts":"2025-04-15T14:30:00Z","situation":"Same pattern recurred","reasoning":"Previous approach worked well","action":"Applied same greeting pattern immediately","outcome":"Faster positive response"}
```

### Folder metadata (.meta.jsonl)

Each folder has a `.meta.jsonl` file. The **last line wins** as the canonical description (used for embedding):

```jsonl
{"description":"Communication actions for apology and reconciliation","created":"2025-04-14T10:00:00Z"}
{"description":"Communication for apology, reconciliation, and conflict resolution","updated":"2025-04-20T15:00:00Z"}
```

### Defensive parsing

All JSONL readers skip unparseable lines with a `tracing::warn!`. In an append-only system, only the last line can ever be corrupt (from a crash mid-write). This ensures past data is never lost.

## Tech stack

| Component | Technology | Notes |
|-----------|-----------|-------|
| Language | Rust | Performance + type safety |
| MCP SDK | rmcp 1.4 | Official MCP Rust SDK |
| Vector index | instant-distance (HNSW) | In-memory + disk serialization |
| Local embedding | fastembed (MultilingualE5Small) | ONNX, 384 dims, 100+ languages, no API key |
| Optional embedding | OpenAI text-embedding-3-small | 1536 dims, requires API key |
| Data format | JSONL | Append-only, grep-friendly, crash-safe |
| Protocol | MCP over stdio | Standard agent-tool interface |

## Design principles

1. **Append-only immutability** — No delete or modify APIs exist. Experience is irreversible. `move` and `rename` are allowed (reorganization is reflection, not deletion).
2. **Trait-driven abstraction** — `EmbeddingProvider` and `VectorStore` are traits. Swap local/OpenAI embeddings or replace HNSW without touching tool logic.
3. **Filesystem is memory** — The directory tree is the source of truth. The HNSW index is a derived, rebuildable cache.
4. **Soft validation** — Naming rule violations produce warnings + proceed with the write. Warnings are logged for agent self-reflection, not enforced as hard errors.
5. **Defensive parsing** — Crash-time partial JSONL writes are skipped on read with a logged warning. The system never loses previously committed data.

## Testing

```bash
# Unit tests (47)
cargo test

# Unit + integration tests (61 total, covers all 14 acceptance criteria)
cargo test --features test-utils
```

Integration tests are mapped 1:1 to acceptance criteria:

| Test | Acceptance criterion |
|------|---------------------|
| `ac01_embed_search` | Semantic search returns correct folder in top-K |
| `ac02_grep_search` | Grep finds files by name + content, respects max_results |
| `ac03_suggest_write` | suggest_location → write_action full cycle |
| `ac04_naming` | Naming validation: valid names pass, invalid names warn |
| `ac05_persistence` | Cross-session retrieval (write in session 1, find in session 2) |
| `ac07_move_rename` | Move/rename updates embedding index correctly |
| `ac08_habit` | Symlink creation + read-through-shortcut |
| `ac09_warning_log` | Warnings accumulate in /reflection log |
| `ac10_warning_query` | get_warnings filters by timestamp |
| `ac11_append_only` | No delete/modify API exists |
| `ac12_meta_append` | .meta.jsonl is append-only, last-line-wins |
| `ac13_bootstrap` | 6-Pillar + skills.md created on startup |
| `ac14_skills_append` | Agent can append to skills.md without modifying seed |

## Project structure

```
src/
├── main.rs                  # Entry point: bootstrap, index init, MCP server
├── lib.rs                   # Module re-exports
├── server.rs                # 10 MCP tools via #[tool_router]
├── bootstrap.rs             # 6-Pillar seed + skills.md initialization
├── config.rs                # Env-based configuration
├── error.rs                 # Error types
├── types.rs                 # Shared domain types
├── fs/
│   ├── action.rs            # Append-only action file CRUD
│   ├── meta.rs              # .meta.jsonl read/append (last-line-wins)
│   ├── naming.rs            # Naming convention validation
│   ├── warnings.rs          # Warning log append + query
│   └── skills.rs            # skills.md hybrid guide read/append
├── search/
│   ├── embedding.rs         # EmbeddingProvider trait + Local + OpenAI + Mock
│   ├── vector_store.rs      # HNSW vector store (instant-distance)
│   ├── grep.rs              # Filename + content regex search
│   └── suggest.rs           # Location suggestion via embedding similarity
└── evolution/
    ├── move_action.rs       # Move + reindex
    ├── rename_action.rs     # Rename + naming validation
    └── habit.rs             # Symlink creation (Unix)
```

## Roadmap

- [ ] Web dashboard for human observation (TypeScript frontend)
- [ ] `LocalEmbeddingProvider` model selection via config
- [ ] Health check / status MCP tool
- [ ] Warning log rotation for long-running agents
- [ ] `hnsw_rs` migration option for advanced filtering
- [ ] Cross-platform symlink support (Windows)

## License

MIT

## Credits

Inspired by Aristotle's concept of practical wisdom (Phronesis, φρόνησις) and the philosophical principle of the unity of knowledge and action (知行合一) — the idea that genuine knowledge is inseparable from the capacity to act.
