import {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import {
  ReactFlow, Background, MiniMap, Controls, Handle, Position, applyNodeChanges,
  type Node, type NodeProps, type SnapGrid, type OnNodesChange, type OnEdgesChange,
  type OnConnect, type OnNodeDrag,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type {Graph, NodePosition, Profile, Snapshot} from '../types';

function ActionNode({data, selected}: NodeProps) {
  return <div className={`aiden-node ${selected ? 'selected' : ''} ${data.active ? 'firing' : ''}`}>
    <Handle type="target" position={Position.Left}/>
    <div className="node-category">{String(data.kind)}</div>
    <strong>{String(data.label)}</strong><small>{String(data.subtitle)}</small>
    <Handle type="source" position={Position.Right}/>
  </div>;
}
const nodeTypes = {aiden: ActionNode};
const SNAP_GRID: SnapGrid = [20, 20];
interface CanvasProps {
  graph: Graph; profiles: Profile[]; snapshot: Snapshot | null;
  onChange: (graph: Graph) => void; onMove: (positions: NodePosition[]) => void;
  onSelect: (id: string) => void;
}
function synchronize(graph: Graph, current: Node[]): Node[] {
  const previous = new Map(current.map(node => [node.id, node]));
  return graph.nodes.map(node => {
    const old = previous.get(node.id);
    return {...old, id: node.id, type: 'aiden',
      position: old?.dragging ? old.position : {x: node.x, y: node.y},
      data: {kind: node.kind, label: node.label, profileId: node.params.profile_id, params: node.params}};
  });
}
export default function Canvas({graph, profiles, snapshot, onChange, onMove, onSelect}: CanvasProps) {
  // Keep measurements, selection and intermediate drag positions in React Flow.
  // Only a completed gesture is persisted to the native workspace.
  const [layout, setLayout] = useState(() => synchronize(graph, []));
  const layoutRef = useRef(layout);
  const graphRef = useRef(graph);
  useEffect(() => {
    graphRef.current = graph;
    const next = synchronize(graph, layoutRef.current);
    layoutRef.current = next;
    setLayout(next);
  }, [graph]);
  const install = useCallback((nodes: Node[]) => {
    layoutRef.current = nodes;
    setLayout(nodes);
  }, []);
  const persist = useCallback((next: Graph) => {
    graphRef.current = next;
    onChange(next);
  }, [onChange]);
  const activeNodeIds = useMemo(() => new Set(snapshot?.mode !== 'Off'
    ? snapshot?.traces.slice(-5).map(trace => trace.node) : []), [snapshot]);
  const nodes = useMemo(() => layout.map(node => {
    const profile = profiles.find(item => item.id === node.data.profileId);
    const params = node.data.params as Record<string, unknown>;
    let subtitle = '';
    if (profile) subtitle = `${profile.enabled ? 'Enabled' : 'Disabled'} · ${profile.examples.filter(e => !e.negative).length} references · ${snapshot?.device ?? 'Offline'}`;
    else if (node.data.kind === 'Timer' || node.data.kind === 'Delay') subtitle = `${params.ms ?? 500} ms`;
    else if (node.data.kind === 'Key Press') subtitle = String(params.key || 'Choose a key');
    return {...node, data: {...node.data, subtitle, active: activeNodeIds.has(node.id), label: profile?.name ?? node.data.label}};
  }), [layout, profiles, snapshot?.device, activeNodeIds]);
  const edges = useMemo(() => graph.edges.map(edge => ({...edge,
    label: edge.port === 'any' ? '' : edge.port, animated: activeNodeIds.has(edge.source),
    style: {stroke: '#7446c7', strokeWidth: 2}})), [graph.edges, activeNodeIds]);
  const handleNodesChange: OnNodesChange = useCallback(changes => {
    const next = applyNodeChanges(changes, layoutRef.current);
    install(next);
    if (changes.some(change => change.type === 'remove')) {
      const ids = new Set(next.map(node => node.id));
      const current = graphRef.current;
      persist({...current, nodes: current.nodes.filter(node => ids.has(node.id)),
        edges: current.edges.filter(edge => ids.has(edge.source) && ids.has(edge.target))});
    }
    // Keyboard movement has no drag-stop callback.
    const positions = changes.flatMap(change => change.type === 'position' && change.dragging === undefined && change.position
      ? [{id: change.id, ...change.position}] : []);
    if (positions.length) onMove(positions);
  }, [install, persist, onMove]);
  const handleDragStop: OnNodeDrag = useCallback((_event, _node, dragged) => {
    const positions = dragged.map(node => ({id: node.id, ...node.position}));
    const byId = new Map(positions.map(position => [position.id, position]));
    install(layoutRef.current.map(node => {
      const position = byId.get(node.id);
      return position ? {...node, position: {x: position.x, y: position.y}, dragging: false} : node;
    }));
    onMove(positions);
  }, [install, onMove]);
  const handleEdgesChange: OnEdgesChange = useCallback(changes => {
    const removed = new Set(changes.flatMap(change => change.type === 'remove' ? [change.id] : []));
    if (removed.size) persist({...graphRef.current, edges: graphRef.current.edges.filter(edge => !removed.has(edge.id))});
  }, [persist]);
  const handleConnect: OnConnect = useCallback(connection => {
    if (!connection.source || !connection.target) return;
    persist({...graphRef.current, edges: [...graphRef.current.edges, {
      id: crypto.randomUUID(), source: connection.source, target: connection.target, port: 'TRIGGER'}]});
  }, [persist]);
  return <ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes}
    onNodeClick={(_event, node) => onSelect(node.id)} onNodesChange={handleNodesChange}
    onNodeDragStop={handleDragStop} onEdgesChange={handleEdgesChange} onConnect={handleConnect}
    fitView snapToGrid snapGrid={SNAP_GRID} deleteKeyCode="Delete" colorMode="dark">
    <Background gap={20} color="#302a33"/><MiniMap pannable zoomable nodeColor="#542998"/><Controls/>
  </ReactFlow>;
}
