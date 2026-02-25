# Claude Code — Project Instructions

## Project: joplin-smart-search
> Status: **MVP backend + frontend complete.** Search bug fixed (see Decisions below).

---

## Claude Flow Integration

All context, findings, and agent results **must** go through Claude Flow memory.
No MEMORY.md file — Claude Flow SQLite is the single source of truth.

**At session start**, retrieve key context:
```
mcp__claude-flow__memory_retrieve  key=project:context   namespace=project
mcp__claude-flow__memory_retrieve  key=project:file-paths namespace=project
mcp__claude-flow__memory_retrieve  key=project:distribution namespace=project
```

| Action | Tool |
|--------|------|
| Store findings | `mcp__claude-flow__memory_store` |
| Retrieve context | `mcp__claude-flow__memory_retrieve` |
| Semantic search | `mcp__claude-flow__memory_search` |
| Check system health | `mcp__claude-flow__system_status` |
| List active agents | `mcp__claude-flow__agent_list` |
| Swarm health | `mcp__claude-flow__swarm_status` |

### Memory Key Conventions
- `project:context` — high-level project state and status
- `project:specs` — requirements and feature specs
- `task:<name>` — task-specific notes and progress
- `agent:<id>:result` — results from completed agents
- `debug:<issue>` — debugging notes and solutions

---

## Agent Rules

1. **Before spawning any agent:** check `mcp__claude-flow__agent_list` — max **3 active** at once.
2. If 3 are active, wait for one to finish before spawning another.
3. Always store agent results to memory before terminating an agent.
4. Use `mcp__claude-flow__swarm_status` to monitor swarm health.

---

## Development Workflow

- Read existing code before suggesting changes.
- Prefer editing existing files over creating new ones.
- Keep solutions minimal — no over-engineering.
- No speculative features, helpers, or abstractions.
- No auto-commits unless explicitly requested.

---

## Tech Stack

| Layer | Choice |
|-------|--------|
| Shell | Tauri 2 |
| Backend | Rust |
| Frontend | React 19 + TypeScript |
| State | Zustand |
| Styling | Tailwind CSS 4 |
| Embeddings | `fastembed 4.9.1` — **bge-small-en-v1.5** via ONNX Runtime, no Python, no GPU, cross-platform |
| Vector index | `ruvector-core 2.0.4` (HNSW) |
| Data source | Joplin local SQLite (read-only) |
| Platform | Linux (AppImage), Windows (.exe) |

---

## MVP Scope (nothing more)

1. Read Joplin SQLite → embed all notes → build HNSW index
2. Semantic search by typed concept → ranked results (title + similarity score)
3. Click result → open note in Joplin via `joplin://` deep-link
4. File watcher → re-embed changed notes → keep index current

**Out of scope for MVP:** LLM synthesis, cloud sync, mobile.

---

## Hard Constraints

- **Read-only** — never modify the Joplin database
- **No Python, no GPU** — fully local and offline-capable
- **Model auto-downloaded** — `Xenova/bge-small-en-v1.5` (~33MB) is fetched from HuggingFace
  on first run and cached in `app_data_dir`. Internet required once only.

---

## Key Implementation Details

- **Embed text** = `"{title}\n\n{body}"` — not body alone; empty-body notes need the title
- **Cache dir** = `app_data_dir()` passed to fastembed; never relative/cwd-dependent
- **Tauri commands** are named without prefix (e.g. `search_notes`, `get_note`) — the `cmd_`
  convention in the code conventions section below is aspirational; existing commands don't use it
- **`is_ready = true`** is only set if `embedding_pipeline.is_some()` — if model load fails,
  `index_status.error` is set and the UI shows the error instead

---

## Decisions & Why

| Decision | Reason |
|----------|--------|
| `fastembed` instead of `ruvector-onnx-embeddings` | `ruvector-onnx-embeddings 0.1.0` fails to compile — accesses `session.inputs` as a public field but `ort 2.0.0-rc.9` changed it to a method. `fastembed` uses the same ort with correct API. |
| Model downloaded, not bundled | ONNX Runtime pre-built binaries are fetched at compile time via `ort-download-binaries`; the model weights (~80MB) are fetched at runtime. Bundling 80MB into the binary is possible but not needed for MVP. |
| CachyOS compatibility | No issues. `ort-download-binaries` fetches standard x86_64 binaries; CachyOS's custom kernel/BORE scheduler has no effect on userspace Rust binaries. |

---

## UI Spec (dark theme, 960×640)

```
┌─────────────────────────────────────────────────────────────┐
│  [ search input (full width) ]          [index status]      │  ← header
├────────────┬────────────────────────────────────────────────┤
│ note title │                                                 │
│ 0.92       │   Note preview                                  │
│────────────│   (matched concepts highlighted)                │
│ note title │                                                 │
│ 0.87       │                                                 │
│────────────│                                                 │
│ ...        │                        [ Open in Joplin ]       │
└────────────┴────────────────────────────────────────────────┘
  280px sidebar        detail panel
```

Style: shadcn-flavored — muted borders, rounded corners, subtle secondary bg, oklch palette.
Selected sidebar row: left accent bar + highlighted background.

---

## Code Conventions

- **Rust:** standard `rustfmt` formatting, `clippy` clean
- **TypeScript:** strict mode, no `any`
- **Components:** shadcn-style, co-locate styles with component
- **Naming:** `snake_case` in Rust, `camelCase`/`PascalCase` in TS
- **No speculative abstractions** — build for MVP first
