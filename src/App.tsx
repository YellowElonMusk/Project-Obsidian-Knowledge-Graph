import { useState, useEffect, useRef, useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

import { GraphCanvas } from './components/GraphCanvas';
import { NodeDetailPanel } from './components/NodeDetail';
import { SessionHistory } from './components/SessionHistory';
import { PetWidget } from './components/PetWidget';
import { useGraph } from './hooks/useGraph';
import { GraphNode, timeAgo } from './types';
import './index.css';

type SidePanel = 'node' | 'sessions' | 'search' | null;
type View = 'graph' | 'pet';

interface ModelStatus {
  ready: boolean;
  downloading: boolean;
  progress_pct: number;
}

export default function App() {
  const graph = useGraph();
  const [view, setView] = useState<View>('graph');
  const [sidePanel, setSidePanel] = useState<SidePanel>('sessions');
  const [searchQuery, setSearchQuery] = useState('');
  const [currentProject, setCurrentProject] = useState('default');
  const [projects, setProjects] = useState<string[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [mcpPort, setMcpPort] = useState<number | null>(null);
  const [mcpConnected, setMcpConnected] = useState(false);
  const [vaultPath, setVaultPath] = useState('');
  const [dims, setDims] = useState({ w: window.innerWidth, h: window.innerHeight });
  const [lastSession, setLastSession] = useState<GraphNode | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const searchTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    graph.loadGraph();
    invoke<number>('get_mcp_port').then(setMcpPort).catch(() => {});
    invoke<string>('get_vault_path').then(setVaultPath).catch(() => {});
    invoke<string[]>('list_projects').then(setProjects).catch(() => {});
  }, []);

  useEffect(() => {
    if (graph.lastIngest) {
      invoke<string[]>('list_projects').then(setProjects).catch(() => {});
    }
  }, [graph.lastIngest]);

  // Refresh last session on project change or after ingest
  useEffect(() => {
    invoke<GraphNode | null>('get_last_session', { projectName: currentProject })
      .then(setLastSession)
      .catch(() => setLastSession(null));
  }, [currentProject, graph.lastIngest]);

  // Poll MCP connection count every 3s
  useEffect(() => {
    const poll = setInterval(() => {
      invoke<number>('get_mcp_connections')
        .then(n => setMcpConnected(n > 0))
        .catch(() => setMcpConnected(false));
    }, 3000);
    return () => clearInterval(poll);
  }, []);

  // Poll model download status every 2s until ready
  useEffect(() => {
    if (modelStatus?.ready) return;
    const poll = setInterval(() => {
      invoke<ModelStatus>('get_model_status')
        .then(setModelStatus)
        .catch(() => {});
    }, 2000);
    return () => clearInterval(poll);
  }, [modelStatus?.ready]);

  useEffect(() => {
    const update = () => setDims({ w: window.innerWidth, h: window.innerHeight });
    window.addEventListener('resize', update);
    return () => window.removeEventListener('resize', update);
  }, []);

  // Tauri file drop
  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (win as any).onFileDropEvent(async (event: any) => {
      if (event.payload.type === 'drop') {
        setIsDragOver(false);
        const paths: string[] = event.payload.paths ?? [];
        for (const p of paths) {
          await graph.ingestFile(p, currentProject);
        }
      } else if (event.payload.type === 'hover') {
        setIsDragOver(true);
      } else if (event.payload.type === 'cancel') {
        setIsDragOver(false);
      }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    }).then((u: any) => { unlisten = u; });

    return () => { unlisten?.(); };
  }, [currentProject]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        graph.selectNode(null);
        setSidePanel(null);
        graph.clearSearch();
        setSearchQuery('');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setSidePanel('search');
        setTimeout(() => searchRef.current?.focus(), 100);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
        e.preventDefault();
        handleOpenFile();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const handleOpenFile = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: 'Supported Files',
          extensions: ['md', 'txt', 'pdf', 'json', 'ts', 'tsx', 'js', 'jsx', 'py', 'rs', 'go', 'java', 'c', 'cpp', 'h', 'yaml', 'toml', 'csv'],
        }],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      for (const p of paths) {
        await graph.ingestFile(p, currentProject);
      }
    } catch (e) { console.error(e); }
  };

  const handleSearch = useCallback((q: string) => {
    setSearchQuery(q);
    if (searchTimer.current) clearTimeout(searchTimer.current);
    if (q.trim().length > 1) {
      searchTimer.current = setTimeout(() => graph.searchGraph(q), 250);
    } else {
      graph.clearSearch();
    }
  }, []);

  const handleNodeClick = useCallback((node: GraphNode) => {
    graph.selectNode(node);
    setSidePanel('node');
  }, []);

  const handleBgClick = useCallback(() => {
    graph.selectNode(null);
    if (sidePanel === 'node') setSidePanel(null);
  }, [sidePanel]);

  const panelWidth = sidePanel ? 300 : 0;
  const bannerHeight = lastSession ? 32 : 0;
  const canvasW = dims.w - panelWidth;
  const canvasH = dims.h - 48 - 24 - bannerHeight; // toolbar + statusbar + banner

  if (view === 'pet') {
    return (
      <div className="fixed inset-0 flex items-end justify-end p-4 pointer-events-none">
        <div className="pointer-events-auto">
          <PetWidget
            nodeCount={graph.nodes.length}
            edgeCount={graph.edges.length}
            isBusy={graph.isBusy}
            lastIngest={graph.lastIngest}
            onOpenFull={() => setView('graph')}
            onFileDrop={(paths) => paths.forEach(p => graph.ingestFile(p, currentProject))}
          />
        </div>
      </div>
    );
  }

  return (
    <div
      className={`flex flex-col w-screen h-screen overflow-hidden ${isDragOver ? 'drop-active' : ''}`}
      style={{ background: '#0a0a0f' }}
    >
      {/* Toolbar */}
      <header
        className="flex items-center gap-2 px-3 shrink-0 border-b"
        style={{ height: 48, borderColor: '#2d2d4e', background: '#12121e' }}
      >
        <span className="text-lg" style={{ color: '#a855f7' }}>⬡</span>
        <span className="font-bold text-sm tracking-wide text-slate-100">Cortex</span>

        <div className="flex items-center gap-1 ml-3">
          <span className="text-xs text-muted">project:</span>
          <select
            value={currentProject}
            onChange={e => setCurrentProject(e.target.value)}
            className="text-xs rounded px-2 py-1 outline-none cursor-pointer"
            style={{ background: '#1a1a2e', border: '1px solid #2d2d4e', color: '#e2e8f0' }}
          >
            <option value="default">default</option>
            {projects.filter(p => p !== 'default').map(p => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-3 ml-2 text-xs text-muted">
          <span>{graph.nodes.length} nodes</span>
          <span>{graph.edges.length} edges</span>
        </div>

        {graph.isBusy && (
          <div className="flex items-center gap-1.5 ml-2">
            <span className="w-1.5 h-1.5 rounded-full bg-purple-400 animate-pulse" />
            <span className="text-xs text-purple-400">ingesting…</span>
          </div>
        )}
        {graph.lastIngest && !graph.isBusy && (
          <span className="text-xs text-green-400 ml-1">
            ✓ {graph.lastIngest.title} (+{graph.lastIngest.nodes_added})
          </span>
        )}
        {graph.error && (
          <span className="text-xs text-red-400 ml-1 truncate max-w-48" title={graph.error}>
            ✕ {graph.error}
          </span>
        )}

        <div className="flex-1" />

        {/* MCP connection status badge + model download progress */}
        {mcpPort && (
          <div className="flex flex-col gap-0.5">
            <div
              className="flex items-center gap-1.5 px-2 py-1 rounded text-xs"
              style={{
                background: mcpConnected ? '#064e3b22' : '#1a1a2e',
                border: `1px solid ${mcpConnected ? '#064e3b44' : '#2d2d4e'}`,
                color: mcpConnected ? '#6ee7b7' : '#64748b',
              }}
            >
              <span
                className="w-1.5 h-1.5 rounded-full"
                style={{ background: mcpConnected ? '#34d399' : '#475569' }}
              />
              {mcpConnected ? 'Agent connected' : 'Waiting for agent'}
            </div>
            {modelStatus && !modelStatus.ready && modelStatus.downloading && (
              <div
                className="relative h-0.5 rounded-full overflow-hidden mx-2"
                style={{ background: '#2d2d4e' }}
                title={`Downloading model: ${modelStatus.progress_pct}%`}
              >
                <div
                  className="absolute inset-y-0 left-0 rounded-full transition-all"
                  style={{ width: `${modelStatus.progress_pct}%`, background: '#a855f7' }}
                />
              </div>
            )}
          </div>
        )}

        <button
          onClick={() => { setSidePanel(sidePanel === 'search' ? null : 'search'); setTimeout(() => searchRef.current?.focus(), 100); }}
          className="text-xs px-2 py-1 rounded hover:bg-white/5 transition-colors"
          style={{ color: sidePanel === 'search' ? '#a855f7' : '#64748b', border: '1px solid #2d2d4e' }}
        >⌕ Search</button>

        <button
          onClick={() => setSidePanel(sidePanel === 'sessions' ? null : 'sessions')}
          className="text-xs px-2 py-1 rounded hover:bg-white/5 transition-colors"
          style={{ color: sidePanel === 'sessions' ? '#a855f7' : '#64748b', border: '1px solid #2d2d4e' }}
        >↻ Sessions</button>

        <button
          onClick={handleOpenFile}
          className="text-xs px-2 py-1 rounded hover:bg-white/5 transition-colors"
          style={{ color: '#64748b', border: '1px solid #2d2d4e' }}
        >+ Ingest</button>

        <button
          onClick={() => setView('pet')}
          className="text-xs px-2 py-1 rounded hover:bg-white/5 transition-colors"
          style={{ color: '#64748b', border: '1px solid #2d2d4e' }}
          title="Memory Orb"
        >◴ Memory Orb</button>
      </header>

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Graph area */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Last session banner */}
          {lastSession && (
            <div
              className="shrink-0 flex items-center gap-2 px-3 cursor-pointer hover:bg-white/5 transition-colors"
              style={{
                height: 32,
                background: '#0d0d1a',
                borderBottom: '1px solid #2d2d4e',
              }}
              onClick={() => setSidePanel('sessions')}
            >
              <span className="text-green-400 text-xs">●</span>
              <span className="text-xs" style={{ color: '#94a3b8' }}>
                {lastSession.label}
              </span>
              <span className="text-xs text-muted">— {timeAgo(lastSession.created_at)}</span>
              <span className="text-xs text-muted ml-auto">expand ▾</span>
            </div>
          )}

          {/* Canvas */}
          <div className="flex-1 relative overflow-hidden">
            <GraphCanvas
              nodes={graph.nodes}
              edges={graph.edges}
              selectedNodeId={graph.selectedNode?.id ?? null}
              onNodeClick={handleNodeClick}
              onBackgroundClick={handleBgClick}
              width={canvasW}
              height={canvasH}
            />

            {graph.nodes.length === 0 && !isDragOver && (
              <div className="absolute inset-0 flex flex-col items-center justify-center gap-6">
                <div className="text-center">
                  <p
                    className="text-4xl font-bold mb-3"
                    style={{ color: 'rgba(226,232,240,0.9)' }}
                  >
                    Your agents forget everything.
                  </p>
                  <p className="text-2xl font-semibold" style={{ color: '#a855f7' }}>
                    Cortex remembers.
                  </p>
                </div>
                <div className="flex items-center gap-3">
                  <button
                    onClick={handleOpenFile}
                    className="text-sm px-4 py-2 rounded transition-colors hover:bg-white/10"
                    style={{ border: '1px solid #2d2d4e', color: '#94a3b8' }}
                  >
                    Drop files to start
                  </button>
                  <button
                    onClick={() => {
                      if (mcpPort) {
                        navigator.clipboard.writeText(
                          `{\n  "mcpServers": {\n    "cortex": {\n      "command": "nc",\n      "args": ["127.0.0.1", "${mcpPort}"]\n    }\n  }\n}`
                        ).catch(() => {});
                      }
                    }}
                    className="text-sm px-4 py-2 rounded transition-colors"
                    style={{ background: '#7c3aed', color: 'white' }}
                    title="Copies MCP config to clipboard"
                  >
                    Connect your agent →
                  </button>
                </div>
                <p className="text-xs text-muted">
                  or press{' '}
                  <kbd className="px-1 py-0.5 rounded text-xs" style={{ background: '#2d2d4e', color: '#94a3b8' }}>
                    Ctrl+O
                  </kbd>
                  {' '}to open files
                </p>
              </div>
            )}
          </div>
        </div>

        {/* Side panel */}
        {sidePanel && (
          <div
            className="shrink-0 border-l flex flex-col overflow-hidden"
            style={{ width: 300, borderColor: '#2d2d4e', background: '#12121e' }}
          >
            {sidePanel === 'search' && (
              <div className="flex flex-col h-full">
                <div className="p-3 border-b" style={{ borderColor: '#2d2d4e' }}>
                  <input
                    ref={searchRef}
                    type="text"
                    value={searchQuery}
                    onChange={e => handleSearch(e.target.value)}
                    placeholder="Search graph…"
                    className="w-full text-xs outline-none px-3 py-2 rounded"
                    style={{ background: '#0a0a0f', border: '1px solid #2d2d4e', color: '#e2e8f0' }}
                    autoFocus
                  />
                </div>
                <div className="flex-1 overflow-y-auto p-2">
                  {graph.searchResults === null ? (
                    <p className="text-xs text-muted text-center mt-8">Type to search</p>
                  ) : graph.searchResults.length === 0 ? (
                    <p className="text-xs text-muted text-center mt-8">No results</p>
                  ) : (
                    <div className="space-y-1">
                      {graph.searchResults.map(n => (
                        <button
                          key={n.id}
                          onClick={() => { handleNodeClick(n); setSidePanel('node'); }}
                          className="w-full text-left rounded p-2.5 hover:bg-white/5 transition-colors"
                          style={{ border: '1px solid #2d2d4e' }}
                        >
                          <div className="text-xs font-medium text-slate-300 truncate">{n.label}</div>
                          <div className="text-xs text-muted mt-0.5">{n.node_type}</div>
                          {n.content && (
                            <div className="text-xs text-muted mt-1 line-clamp-2">
                              {n.content.slice(0, 100)}
                            </div>
                          )}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}

            {sidePanel === 'node' && (
              <NodeDetailPanel
                detail={graph.nodeDetail}
                node={graph.selectedNode}
                onSelectNode={handleNodeClick}
                onClose={() => { setSidePanel(null); graph.selectNode(null); }}
              />
            )}

            {sidePanel === 'sessions' && (
              <SessionHistory onSelectNode={handleNodeClick} />
            )}
          </div>
        )}
      </div>

      {/* Status bar */}
      <div
        className="shrink-0 flex items-center px-3 gap-3 text-xs text-muted border-t"
        style={{ height: 24, borderColor: '#2d2d4e', background: '#0d0d1a' }}
      >
        {vaultPath && (
          <span>
            vault: <code style={{ color: 'rgba(96,165,250,0.6)' }}>
              {vaultPath.split(/[/\\]/).slice(-2).join('/')}
            </code>
          </span>
        )}
        {modelStatus && !modelStatus.ready && (
          <span style={{ color: '#a855f7' }}>
            {modelStatus.downloading
              ? `Downloading semantic model… ${modelStatus.progress_pct}%`
              : 'Loading semantic model…'}
          </span>
        )}
        <span className="ml-auto">
          Cortex v0.1 · Drop files to ingest · Ctrl+K search
        </span>
      </div>
    </div>
  );
}
