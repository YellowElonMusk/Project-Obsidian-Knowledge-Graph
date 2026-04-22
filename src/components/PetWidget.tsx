import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GraphNode, IngestSummary, timeAgo } from '../types';

interface Props {
  nodeCount: number;
  edgeCount: number;
  isBusy: boolean;
  lastIngest: IngestSummary | null;
  onOpenFull: () => void;
  onFileDrop: (paths: string[]) => void;
}

export function PetWidget({ nodeCount, edgeCount, isBusy, lastIngest, onOpenFull }: Props) {
  const [lastSession, setLastSession] = useState<GraphNode | null>(null);
  const [hovering, setHovering] = useState(false);
  const [mcpPort, setMcpPort] = useState<number | null>(null);

  useEffect(() => {
    invoke<number>('get_mcp_port').then(setMcpPort).catch(() => {});
    invoke<GraphNode | null>('get_last_session', { projectName: 'default' })
      .then(setLastSession)
      .catch(() => {});
  }, [lastIngest]);

  return (
    <div
      className="relative flex flex-col"
      onMouseEnter={() => setHovering(true)}
      onMouseLeave={() => setHovering(false)}
    >
      {/* Main orb button */}
      <button
        onClick={onOpenFull}
        className="relative w-14 h-14 rounded-full flex items-center justify-center transition-all duration-300"
        style={{
          background: isBusy
            ? 'radial-gradient(circle, #a855f7, #7c3aed)'
            : 'radial-gradient(circle, #4c1d95, #1e1b4b)',
          boxShadow: isBusy
            ? '0 0 20px #7c3aed, 0 0 40px #7c3aed66'
            : hovering
            ? '0 0 12px #7c3aed88'
            : '0 0 6px #7c3aed44',
          border: '1.5px solid #7c3aed66',
          animation: isBusy ? 'spin-slow 4s linear infinite' : undefined,
        }}
        title="Open Cortex"
      >
        <span
          className="text-2xl"
          style={{
            filter: isBusy ? 'brightness(1.5)' : 'brightness(0.8)',
            transition: 'filter 0.3s',
          }}
        >
          ⬡
        </span>

        {/* Activity indicator */}
        {isBusy && (
          <span
            className="absolute -top-0.5 -right-0.5 w-3 h-3 rounded-full border-2"
            style={{
              background: '#10b981',
              borderColor: '#0a0a0f',
              animation: 'pulse 1s ease-in-out infinite',
            }}
          />
        )}
      </button>

      {/* Hover tooltip */}
      {hovering && (
        <div
          className="absolute bottom-16 left-1/2 -translate-x-1/2 w-56 rounded-lg p-3 text-xs space-y-2 pointer-events-none z-50"
          style={{
            background: '#12121e',
            border: '1px solid #2d2d4e',
            boxShadow: '0 8px 32px rgba(0,0,0,0.6)',
          }}
        >
          {/* Stats */}
          <div className="flex gap-3 pb-2" style={{ borderBottom: '1px solid #2d2d4e' }}>
            <div className="text-center">
              <div className="font-bold text-base text-purple-400">{nodeCount}</div>
              <div className="text-muted">nodes</div>
            </div>
            <div className="text-center">
              <div className="font-bold text-base text-blue-400">{edgeCount}</div>
              <div className="text-muted">edges</div>
            </div>
            {mcpPort && (
              <div className="text-center">
                <div className="font-bold text-base text-green-400">:{mcpPort}</div>
                <div className="text-muted">MCP</div>
              </div>
            )}
          </div>

          {/* Last ingest */}
          {lastIngest && (
            <div className="text-green-400">
              ✓ Ingested: {lastIngest.title}
              <span className="text-muted ml-1">
                (+{lastIngest.nodes_added} nodes)
              </span>
            </div>
          )}

          {/* Last session */}
          {lastSession ? (
            <div>
              <div className="text-muted mb-0.5">Last session:</div>
              <div className="text-slate-300 line-clamp-2">{lastSession.label}</div>
              <div className="text-muted mt-0.5">{timeAgo(lastSession.created_at)}</div>
            </div>
          ) : (
            <div className="text-muted">No sessions yet</div>
          )}

          {/* Status */}
          {isBusy && (
            <div className="flex items-center gap-1.5 text-purple-400">
              <span className="animate-pulse">●</span>
              <span>Agent active…</span>
            </div>
          )}

          {/* MCP hint */}
          {mcpPort && (
            <div
              className="pt-2 text-muted"
              style={{ borderTop: '1px solid #2d2d4e' }}
            >
              MCP: <code className="text-blue-400">localhost:{mcpPort}</code>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
