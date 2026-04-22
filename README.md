# ⬡ Cortex

> A persistent knowledge graph and memory layer for AI agents and human teams. Obsidian meets agent memory.

**Local-first. No account. No server. No telemetry.**

---

## What it does

Cortex is a Tauri desktop app that ingests your files, builds a local knowledge graph, and exposes it as context to any AI agent via MCP (Model Context Protocol).

- **Drop any file** → entities extracted → graph built automatically
- **Visual graph canvas** — force-directed, color-coded by node type
- **Agent memory** — every agent session writes back into the graph; next session can recall what was done
- **Local MCP server** on `localhost:7340` — any Claude Code / Cursor / agent connects and queries the graph
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

- **Windows** with [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) (C++ workload)
- **Rust** — install via [rustup.rs](https://rustup.rs) (MSVC toolchain)
- **Node.js** 18+

### Install & run

```bat
git clone https://github.com/YellowElonMusk/Project-Obsidian-Knowledge-Graph.git
cd Project-Obsidian-Knowledge-Graph

npm install

rem Launch dev mode (sets up MSVC env automatically)
dev.bat
```

> **Important on Windows:** Always use `dev.bat` to launch — it calls `vcvars64.bat` before starting Tauri so the MSVC linker is on PATH.

### Production build

```bat
build.bat
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

- [ ] Local embedding model (nomic-embed via ONNX) for semantic search
- [ ] PDF text extraction via lopdf
- [ ] Timeline scrubber — replay graph state at any point in time
- [ ] Multiple vault directories
- [ ] Plugin system for custom entity extractors
- [ ] Export graph as JSON / GraphML
