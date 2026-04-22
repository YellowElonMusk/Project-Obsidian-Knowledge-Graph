export type NodeType =
  | 'file'
  | 'concept'
  | 'person'
  | 'task'
  | 'decision'
  | 'session'
  | 'code'
  | 'agent_memory';

export interface GraphNode {
  id: string;
  label: string;
  node_type: NodeType;
  content?: string;
  file_path?: string;
  created_at: number;
  updated_at: number;
  metadata?: string;
}

export interface GraphEdge {
  id: string;
  source_id: string;
  target_id: string;
  relationship: string;
  weight: number;
  created_at: number;
  metadata?: string;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface NodeDetail {
  node: GraphNode;
  neighbors: GraphNode[];
  edges: GraphEdge[];
}

export interface IngestSummary {
  nodes_added: number;
  edges_added: number;
  title: string;
}

export interface ForceNode extends GraphNode {
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
  fx?: number;
  fy?: number;
  [key: string]: unknown;
}

export interface ForceLink {
  source: string | ForceNode;
  target: string | ForceNode;
  relationship: string;
  weight: number;
}

export const NODE_COLORS: Record<NodeType, string> = {
  file: '#3b82f6',
  concept: '#8b5cf6',
  person: '#10b981',
  task: '#f59e0b',
  decision: '#ef4444',
  session: '#6366f1',
  code: '#06b6d4',
  agent_memory: '#ec4899',
};

export const NODE_SIZES: Record<NodeType, number> = {
  file: 8,
  concept: 6,
  person: 7,
  task: 5,
  decision: 7,
  session: 9,
  code: 5,
  agent_memory: 8,
};

export function parseMetadata(raw?: string): Record<string, unknown> {
  if (!raw) return {};
  try { return JSON.parse(raw); } catch { return {}; }
}

export function formatTs(ms: number): string {
  return new Date(ms).toLocaleString();
}

export function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  const m = Math.floor(diff / 60000);
  const h = Math.floor(m / 60);
  const d = Math.floor(h / 24);
  if (d > 0) return `${d}d ago`;
  if (h > 0) return `${h}h ago`;
  if (m > 0) return `${m}m ago`;
  return 'just now';
}
