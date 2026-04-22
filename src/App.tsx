import { useState, useEffect, useRef, useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

import { GraphCanvas } from './components/GraphCanvas';
import { NodeDetailPanel } from './components/NodeDetail';
import { SessionHistory } from './components/SessionHistory';
import { PetWidget } from './components/PetWidget';
import { useGraph } from './hooks/useGraph';
import { GraphNode } from './types';
import './index.css';

type SidePanel = 'node' | 'sessions' | 'search' | null;
type View = 'graph' | 'pet';

export default function App() {
  const graph = useGraph();
  const [view, setView] = useState<View>('graph');
  const [sidePanel, setSidePanel] = useState<SidePanel>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentProject, setCurrentProject] = useState('default');
  const [projects, setProjects] = useState<string[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [mcpPort, setMcpPort] = useState<number | null>(null);
  const [vaultPath, setVaultPath] = useState('');
  const [dims, setDims] = useState({ w: window.innerWidth, h: window.innerHeight });
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
  const canvasW = dims.w - panelWidth;
  const canvasH = dims.h - 48 - 24; // toolbar + statusbar

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

        {mcpPort && (
          <div
            className="flex items-center gap-1.5 px-2 py-1 rounded text-xs"
            style={{ background: '#064e3b22', border: '1px solid #064e3b44', color: '#6ee7b7' }}
          >
            <span className="w-1.5 h-1.5 rounded-full bg-green-400" />
            MCP :{mcpPort}
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
          title="Compact mode"
        >◴</button>
      </header>

      {/* Main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Graph */}
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
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-6 pointer-events-none">
              <div
                className="w-32 h-32 rounded-full flex items-center justify-center"
                style={{
                  background: 'radial-gradient(circle, #4c1d9522, #0a0a0f)',
                  border: '2px dashed #2d2d4e',
                  animation: 'pulse 3s ease-in-out infinite',
                }}
              >
                <span className="text-5xl" style={{ opacity: 0.3 }}>⬡</span>
              </div>
              <div className="text-center">
                <p className="font-semibold" style={{ color: 'rgba(226,232,240,0.4)' }}>
                  Drop files to start building your graph
                </p>
                <p className="text-xs mt-2 text-muted">
                  .md .txt .pdf .json and any code file
                </p>
                <p className="text-xs mt-1 text-muted">
                  or press{' '}
                  <kbd className="px-1 py-0.5 rounded text-xs" style={{ background: '#2d2d4e', color: '#94a3b8' }}>
                    Ctrl+O
                  </kbd>
                  {' '}to open files
                </p>
              </div>
            </div>
          )}
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
        <span className="ml-auto">
          Cortex v0.1 · Drop files to ingest · Ctrl+K search
        </span>
      </div>
    </div>
  );
}
