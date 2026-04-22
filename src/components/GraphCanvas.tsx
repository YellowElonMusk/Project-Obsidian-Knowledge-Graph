import { useRef, useCallback, useEffect, useMemo } from 'react';
import ForceGraph2D from 'react-force-graph-2d';
import { GraphNode, GraphEdge, NODE_COLORS, NODE_SIZES, ForceNode, ForceLink } from '../types';

interface Props {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNodeId: string | null;
  onNodeClick: (node: GraphNode) => void;
  onBackgroundClick: () => void;
  width: number;
  height: number;
}

export function GraphCanvas({
  nodes,
  edges,
  selectedNodeId,
  onNodeClick,
  onBackgroundClick,
  width,
  height,
}: Props) {
  const graphRef = useRef<any>(null);

  // Transform to force-graph format
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const graphData = useMemo((): { nodes: any[]; links: any[] } => ({
    nodes: nodes.map(n => ({ ...n })),
    links: edges.map(e => ({
      source: e.source_id,
      target: e.target_id,
      relationship: e.relationship,
      weight: e.weight,
    })) as ForceLink[],
  }), [nodes, edges]);

  // Focus selected node
  useEffect(() => {
    if (selectedNodeId && graphRef.current) {
      const node = nodes.find(n => n.id === selectedNodeId);
      if (node) {
        const fn = graphData.nodes.find(n => n.id === selectedNodeId);
        if (fn?.x != null && fn?.y != null) {
          graphRef.current.centerAt(fn.x, fn.y, 600);
          graphRef.current.zoom(2.5, 600);
        }
      }
    }
  }, [selectedNodeId, nodes, graphData.nodes]);

  const drawNode = useCallback((node: ForceNode, ctx: CanvasRenderingContext2D, globalScale: number) => {
    const { id, label, node_type } = node;
    const x = node.x ?? 0;
    const y = node.y ?? 0;
    const color = NODE_COLORS[node_type] ?? '#888';
    const baseSize = NODE_SIZES[node_type] ?? 6;
    const size = baseSize / Math.max(0.5, globalScale * 0.5);
    const isSelected = id === selectedNodeId;

    // Outer glow for selected
    if (isSelected) {
      ctx.beginPath();
      ctx.arc(x, y, size + 4, 0, 2 * Math.PI);
      ctx.fillStyle = color + '33';
      ctx.fill();
      ctx.beginPath();
      ctx.arc(x, y, size + 2, 0, 2 * Math.PI);
      ctx.fillStyle = color + '66';
      ctx.fill();
    }

    // Node circle
    ctx.beginPath();
    ctx.arc(x, y, size, 0, 2 * Math.PI);
    ctx.fillStyle = color;
    ctx.shadowColor = color;
    ctx.shadowBlur = isSelected ? 12 : 4;
    ctx.fill();
    ctx.shadowBlur = 0;

    // Border
    ctx.strokeStyle = isSelected ? '#fff' : color + 'aa';
    ctx.lineWidth = isSelected ? 2 : 0.5;
    ctx.stroke();

    // Label (only when zoomed in enough)
    if (globalScale > 0.8) {
      const fontSize = Math.max(8, 10 / globalScale);
      ctx.font = `${fontSize}px 'JetBrains Mono', monospace`;
      ctx.fillStyle = isSelected ? '#fff' : '#cbd5e1';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      const maxLen = Math.floor(40 / globalScale);
      const displayLabel = label.length > maxLen ? label.slice(0, maxLen) + '…' : label;
      ctx.fillText(displayLabel, x, y + size + fontSize);
    }
  }, [selectedNodeId]);

  const drawLink = useCallback((link: any, ctx: CanvasRenderingContext2D) => {
    const start = link.source;
    const end = link.target;
    if (!start?.x || !end?.x) return;

    const isConnectedToSelected =
      selectedNodeId &&
      (start.id === selectedNodeId || end.id === selectedNodeId);

    ctx.beginPath();
    ctx.moveTo(start.x, start.y);
    ctx.lineTo(end.x, end.y);
    ctx.strokeStyle = isConnectedToSelected ? '#7c3aed88' : '#2d2d4e';
    ctx.lineWidth = isConnectedToSelected ? 1.5 : 0.8;
    ctx.stroke();
  }, [selectedNodeId]);

  const handleNodeClick = useCallback((node: object) => {
    onNodeClick(node as GraphNode);
  }, [onNodeClick]);

  const handleNodeHover = useCallback((node: object | null) => {
    document.body.style.cursor = node ? 'pointer' : 'default';
  }, []);

  if (nodes.length === 0) {
    return (
      <div
        className="w-full h-full flex flex-col items-center justify-center gap-4"
        style={{ background: '#0a0a0f' }}
        onClick={onBackgroundClick}
      >
        <div className="text-6xl opacity-20 animate-float">⬡</div>
        <div className="text-muted text-center">
          <p className="text-lg font-medium text-text/40">No nodes yet</p>
          <p className="text-sm mt-1 text-muted">Drop files onto the window to ingest them</p>
        </div>
      </div>
    );
  }

  return (
    <div className="graph-canvas w-full h-full" style={{ background: '#0a0a0f' }}>
      <ForceGraph2D
        ref={graphRef}
        graphData={graphData}
        width={width}
        height={height}
        nodeLabel={() => ''}
        backgroundColor="#0a0a0f"
        nodeCanvasObject={drawNode as any}
        nodeCanvasObjectMode={() => 'replace'}
        linkCanvasObject={drawLink}
        linkCanvasObjectMode={() => 'replace'}
        onNodeClick={handleNodeClick}
        onNodeHover={handleNodeHover}
        onBackgroundClick={onBackgroundClick}
        linkDirectionalArrowLength={3}
        linkDirectionalArrowRelPos={1}
        d3AlphaDecay={0.02}
        d3VelocityDecay={0.3}
        cooldownTicks={100}
        linkDirectionalParticles={selectedNodeId ? 1 : 0}
        linkDirectionalParticleWidth={2}
        linkDirectionalParticleColor={() => '#7c3aed'}
      />
    </div>
  );
}
