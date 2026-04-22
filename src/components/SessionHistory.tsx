import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GraphNode, parseMetadata, timeAgo, NODE_COLORS } from '../types';

interface Props {
  onSelectNode: (node: GraphNode) => void;
}

export function SessionHistory({ onSelectNode }: Props) {
  const [sessions, setSessions] = useState<GraphNode[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<GraphNode[]>('get_recent_sessions', { limit: 30 })
      .then(setSessions)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-32 text-muted text-xs">
        Loading sessions…
      </div>
    );
  }

  if (sessions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-32 gap-2 text-muted">
        <span className="text-2xl opacity-30">🔄</span>
        <span className="text-xs">No sessions yet</span>
        <span className="text-xs opacity-60">Agent memory will appear here</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="px-3 pt-3 pb-2 text-xs font-semibold text-muted uppercase tracking-wider">
        Session History
      </div>
      <div className="flex-1 overflow-y-auto px-2 pb-2 space-y-1.5">
        {sessions.map((s, i) => {
          const meta = parseMetadata(s.metadata);
          const color = NODE_COLORS[s.node_type] ?? '#6366f1';
          const isFirst = i === 0;

          return (
            <button
              key={s.id}
              onClick={() => onSelectNode(s)}
              className="w-full text-left rounded p-2.5 transition-all hover:bg-white/5 group relative"
              style={{
                background: isFirst ? `${color}11` : 'transparent',
                border: `1px solid ${isFirst ? color + '44' : '#2d2d4e'}`,
              }}
            >
              {/* Timeline dot */}
              <div className="flex items-start gap-2">
                <div className="flex flex-col items-center mt-1">
                  <span
                    className="w-2 h-2 rounded-full shrink-0"
                    style={{
                      background: color,
                      boxShadow: isFirst ? `0 0 6px ${color}88` : undefined,
                    }}
                  />
                  {i < sessions.length - 1 && (
                    <div
                      className="w-px flex-1 mt-1"
                      style={{ background: '#2d2d4e', minHeight: 12 }}
                    />
                  )}
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-0.5">
                    <span
                      className="text-xs px-1 py-0.5 rounded uppercase tracking-wider font-medium"
                      style={{ background: color + '22', color, border: `1px solid ${color}44` }}
                    >
                      {s.node_type === 'agent_memory' ? 'agent' : 'session'}
                    </span>
                    <span className="text-xs text-muted ml-auto shrink-0">
                      {timeAgo(s.created_at)}
                    </span>
                  </div>

                  <p className="text-xs font-medium text-slate-300 group-hover:text-white transition-colors truncate">
                    {s.label}
                  </p>

                  {!!meta.project && (
                    <p className="text-xs text-muted mt-0.5">
                      project: {String(meta.project)}
                    </p>
                  )}

                  {s.content && (
                    <p className="text-xs text-muted mt-1 line-clamp-2 leading-relaxed">
                      {s.content.slice(0, 120)}
                    </p>
                  )}
                </div>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
