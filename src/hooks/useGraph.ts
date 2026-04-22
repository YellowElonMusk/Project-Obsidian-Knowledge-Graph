import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GraphNode, GraphEdge, GraphData, NodeDetail, IngestSummary } from '../types';

export interface GraphState {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNode: GraphNode | null;
  nodeDetail: NodeDetail | null;
  searchResults: GraphNode[] | null;
  isLoading: boolean;
  isBusy: boolean;
  error: string | null;
  lastIngest: IngestSummary | null;
}

export function useGraph() {
  const [state, setState] = useState<GraphState>({
    nodes: [],
    edges: [],
    selectedNode: null,
    nodeDetail: null,
    searchResults: null,
    isLoading: false,
    isBusy: false,
    error: null,
    lastIngest: null,
  });

  const busyRef = useRef(false);

  const setError = (err: string | null) =>
    setState(s => ({ ...s, error: err }));

  const setBusy = (v: boolean) => {
    busyRef.current = v;
    setState(s => ({ ...s, isBusy: v }));
  };

  const loadGraph = useCallback(async () => {
    setState(s => ({ ...s, isLoading: true, error: null }));
    try {
      const data = await invoke<GraphData>('get_graph_data');
      setState(s => ({
        ...s,
        nodes: data.nodes,
        edges: data.edges,
        isLoading: false,
      }));
    } catch (e) {
      setState(s => ({ ...s, isLoading: false, error: String(e) }));
    }
  }, []);

  const ingestFile = useCallback(async (path: string, project?: string) => {
    if (busyRef.current) return;
    setBusy(true);
    setState(s => ({ ...s, error: null, lastIngest: null }));
    try {
      const summary = await invoke<IngestSummary>('ingest_file', { path, project });
      setState(s => ({ ...s, lastIngest: summary }));
      await loadGraph();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [loadGraph]);

  const ingestText = useCallback(async (content: string, title: string, project?: string) => {
    if (busyRef.current) return;
    setBusy(true);
    setState(s => ({ ...s, error: null, lastIngest: null }));
    try {
      const summary = await invoke<IngestSummary>('ingest_text', { content, title, project });
      setState(s => ({ ...s, lastIngest: summary }));
      await loadGraph();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [loadGraph]);

  const selectNode = useCallback(async (node: GraphNode | null) => {
    if (!node) {
      setState(s => ({ ...s, selectedNode: null, nodeDetail: null }));
      return;
    }
    setState(s => ({ ...s, selectedNode: node, nodeDetail: null }));
    try {
      const detail = await invoke<NodeDetail | null>('get_node_detail', { nodeId: node.id });
      setState(s => ({ ...s, nodeDetail: detail }));
    } catch (e) {
      console.error('Failed to load node detail:', e);
    }
  }, []);

  const searchGraph = useCallback(async (query: string) => {
    if (!query.trim()) {
      setState(s => ({ ...s, searchResults: null }));
      return;
    }
    try {
      const results = await invoke<GraphNode[]>('search_graph', { query });
      setState(s => ({ ...s, searchResults: results }));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const clearSearch = useCallback(() => {
    setState(s => ({ ...s, searchResults: null }));
  }, []);

  const writeMemory = useCallback(async (
    sessionId: string,
    action: string,
    result: string,
    project?: string,
    nodesTouched?: string[],
  ) => {
    try {
      const id = await invoke<string>('write_agent_memory', {
        sessionId, action, result,
        project: project ?? 'default',
        nodesTouched: nodesTouched ?? [],
      });
      await loadGraph();
      return id;
    } catch (e) {
      setError(String(e));
      return null;
    }
  }, [loadGraph]);

  return {
    ...state,
    loadGraph,
    ingestFile,
    ingestText,
    selectNode,
    searchGraph,
    clearSearch,
    writeMemory,
  };
}
