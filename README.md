# ⬡ Cortex

Your AI agents forget everything after each session. **Cortex remembers.**

Cortex is a local knowledge graph that stores what your agents did, what files they touched, and what decisions were made — then surfaces that context back to them on the next session via MCP (Model Context Protocol). No more re-explaining the codebase. No more "as I mentioned last time."

> Like Obsidian for agent memory. Drop files, build a graph, connect your agent — all local, no cloud.

**Local-first. No account. No server. No telemetry.**

---

## Quick connect

Add to your Claude Code / Cursor MCP config:

```json
{
  "mcpServers": {
    "cortex": {
      "command": "nc",
      "args": ["127.0.0.1", "7340"]
    }
  }
}
```

Then start Cortex, drop in your project files, and your agent can immediately call `graph_search`, `get_project_context`, `write_agent_memory`, and more.

---

## What it does

- **Drop any file** → entities extracted → graph built automatically
- **Visual graph canvas** — force-directed, color-coded by node type
- **Agent memory** — every agent session writes back into the graph; next session can recall what was done
- **Local MCP server** on `localhost:7340` — any Claude Code / Cursor / agent connects and queries the graph
- **Semantic search** — nomic-embed-text-v1.5 (ONNX, ~70MB, downloaded on first launch) adds cosine similarity fallback when keyword search returns sparse results
- **Vault mirror** — every ingested file gets a `.md` copy for human readability and git-compatibility

---

## Tech stack

| Layer | Tech |
|-------|------|
| Desktop shell | Tauri v2 (Rust + WebView2) |
| Frontend | React 19 + TypeScript + Tailwind CSS |
| Bundler | Vite |
| Graph DB | SQLite via rusqlite (bundled) + FTS5 full-text search |
| Graph viz | react-force-graph-2d |
| Agent API | MCP JSON-RPC 2.0 over TCP |

---

## Getting started

### Prerequisites

- **Rust** via [rustup.rs](https://rustup.rs)
- **Node.js** 18+
- Platform-specific: see [DEVELOPMENT.md](./DEVELOPMENT.md) for macOS (Xcode CLT), Linux (webkit2gtk + build-essential), and Windows (VS 2022 Build Tools) setup.

### Install & run

```sh
git clone https://github.com/YellowElonMusk/Project-Obsidian-Knowledge-Graph.git
cd Project-Obsidian-Knowledge-Graph

npm install
npm run tauri:dev
```

> **Windows:** Use `dev.bat` instead — it calls `vcvars64.bat` before launching Tauri to ensure the MSVC linker is on PATH.

### Production build

```sh
npm run tauri:build
```

---

## Features

### Drag & drop ingestion
Drop `.md`, `.txt`, `.pdf`, `.json`, `.ts`, `.py`, `.rs`, and any code file onto the window. Cortex parses, chunks, and extracts:
- **Headings** → concept nodes
- **Wiki links** (`[[...]]`) + markdown links → reference edges
- **Checklist tasks** + `TODO/FIXME` → task nodes
- **Decision sentences** → decision nodes
- **Proper names** → person nodes
- **Code blocks** → code nodes

### Knowledge graph view
- Force-directed canvas (react-force-graph-2d)
- Click any node to see full content + connected neighbors
- Ctrl+K to search across all ingested content (FTS5 BM25 ranking)
- Session history panel — scrollable timeline of all agent activity

### Agent memory layer (MCP)
The MCP server on `localhost:7340` exposes 5 tools:

| Tool | Description |
|------|-------------|
| `graph_search(query)` | Full-text search across all nodes |
| `get_project_context(project_name)` | All nodes for a project |
| `write_agent_memory(session_id, action, result, ...)` | Agent writes back to graph |
| `get_last_session(project_name)` | What the agent did last time |
| `list_projects()` | All known projects |

See [MCP_USAGE.md](./MCP_USAGE.md) for connection details and example JSON-RPC calls.

### Pet widget
Click ◴ in the toolbar to collapse to a floating orb that shows graph stats, last session summary, and MCP port — without leaving your screen.

---

## Project structure

```
cortex/
├── src-tauri/src/
│   ├── lib.rs          # Tauri commands + tray icon
│   ├── graph.rs        # SQLite graph ops (nodes, edges, FTS5)
│   ├── ingest.rs       # File parsing + entity extraction
│   └── mcp_server.rs   # MCP JSON-RPC 2.0 TCP server
├── src/
│   ├── App.tsx                 # Main UI + drag-drop + keyboard shortcuts
│   ├── types.ts                # Shared TypeScript types
│   ├── components/
│   │   ├── GraphCanvas.tsx     # Force graph visualization
│   │   ├── NodeDetail.tsx      # Node detail side panel
│   │   ├── SessionHistory.tsx  # Agent session timeline
│   │   └── PetWidget.tsx       # Floating compact widget
│   └── hooks/useGraph.ts       # State + Tauri invoke bindings
├── dev.bat             # Windows dev launcher
├── build.bat           # Windows production build
└── MCP_USAGE.md        # How to connect AI agents
```

---

## Data storage

All data lives locally:

| Path | Contents |
|------|----------|
| `%APPDATA%\com.cortex.app\cortex.db` | SQLite graph database |
| `%APPDATA%\com.cortex.app\vault\` | Markdown mirror of ingested files |

The vault folder is human-readable and git-friendly — same trust signal as Obsidian.

---

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+K` | Open search |
| `Ctrl+O` | Open file picker |
| `Escape` | Deselect / close panels |

---

## Privacy

- Zero network calls during normal use
- All processing runs locally (SQLite FTS5, regex entity extraction)
- MCP server binds to `127.0.0.1` only — not accessible from network
- No telemetry, no analytics, no account required

---

## Roadmap

- [x] Local embedding model (nomic-embed-text-v1.5 via ONNX) for semantic search
- [ ] PDF text extraction via lopdf
- [ ] Timeline scrubber — replay graph state at any point in time
- [ ] Multiple vault directories
- [ ] Plugin system for custom entity extractors
- [ ] Export graph as JSON / GraphML
