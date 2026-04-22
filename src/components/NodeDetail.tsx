import { NodeDetail as INodeDetail, GraphNode, NODE_COLORS, parseMetadata, formatTs, timeAgo } from '../types';

interface Props {
  detail: INodeDetail | null;
  node: GraphNode | null;
  onSelectNode: (node: GraphNode) => void;
  onClose: () => void;
}

const TYPE_ICONS: Record<string, string> = {
  file: '📄',
  concept: '💡',
  person: '👤',
  task: '✅',
  decision: '⚡',
  session: '🔄',
  code: '💻',
  agent_memory: '🧠',
};

export function NodeDetailPanel({ detail, node, onSelectNode, onClose }: Props) {
  if (!node) return null;

  const color = NODE_COLORS[node.node_type] ?? '#888';
  const meta = parseMetadata(node.metadata);
  const icon = TYPE_ICONS[node.node_type] ?? '●';

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div
        className="flex items-start gap-2 p-3 border-b"
        style={{ borderColor: '#2d2d4e' }}
      >
        <span className="text-lg mt-0.5">{icon}</span>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span
              className="text-xs px-1.5 py-0.5 rounded font-medium uppercase tracking-wider"
              style={{ background: color + '22', color, border: `1px solid ${color}44` }}
            >
              {node.node_type}
            </span>
            <span className="text-xs text-muted">{timeAgo(node.created_at)}</span>
          </div>
          <h2
            className="selectable font-semibold text-sm mt-1 leading-tight break-words"
            style={{ color: '#e2e8f0' }}
          >
            {node.label}
          </h2>
        </div>
        <button
          onClick={onClose}
          className="text-muted hover:text-text transition-colors shrink-0 ml-1"
          title="Close"
        >
          ✕
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {/* Metadata pills */}
        {!!meta.project && (
          <div className="flex items-center gap-1.5">
            <span className="text-xs text-muted">Project:</span>
            <span
              className="text-xs px-2 py-0.5 rounded-full"
              style={{ background: '#7c3aed22', color: '#a855f7', border: '1px solid #7c3aed44' }}
            >
              {String(meta.project)}
            </span>
          </div>
        )}

        {node.file_path && (
          <div>
            <span className="text-xs text-muted block mb-1">Source</span>
            <code
              className="selectable text-xs block truncate text-blue-400 bg-surface px-2 py-1 rounded"
              title={node.file_path}
            >
              {node.file_path}
            </code>
          </div>
        )}

        {/* Content preview */}
        {node.content && (
          <div>
            <span className="text-xs text-muted block mb-1">Content</span>
            <div
              className="selectable text-xs leading-relaxed rounded p-2 max-h-48 overflow-y-auto whitespace-pre-wrap"
              style={{
                background: '#0a0a0f',
                border: '1px solid #2d2d4e',
                color: '#94a3b8',
              }}
            >
              {node.content.length > 1200
                ? node.content.slice(0, 1200) + '…'
                : node.content}
            </div>
          </div>
        )}

        <div className="text-xs text-muted">{formatTs(node.created_at)}</div>

        {/* Connected nodes */}
        {detail && detail.neighbors.length > 0 && (
          <div>
            <span className="text-xs text-muted block mb-1.5">
              Connected ({detail.neighbors.length})
            </span>
            <div className="space-y-1">
              {detail.neighbors.map(n => {
                const nColor = NODE_COLORS[n.node_type] ?? '#888';
                const rel = detail.edges.find(
                  e => (e.source_id === node.id && e.target_id === n.id) ||
                       (e.target_id === node.id && e.source_id === n.id)
                );
                return (
                  <button
                    key={n.id}
                    onClick={() => onSelectNode(n)}
                    className="w-full text-left px-2 py-1.5 rounded flex items-center gap-2 hover:bg-white/5 transition-colors group"
                    style={{ border: '1px solid #2d2d4e' }}
                  >
                    <span
                      className="w-2 h-2 rounded-full shrink-0"
                      style={{ background: nColor }}
                    />
                    <span className="flex-1 text-xs truncate text-slate-300 group-hover:text-white transition-colors">
                      {n.label}
                    </span>
                    {rel && (
                      <span className="text-xs text-muted shrink-0">{rel.relationship}</span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
