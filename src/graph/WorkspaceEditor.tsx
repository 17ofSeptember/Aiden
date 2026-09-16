import {useRef, useState} from 'react';
import Canvas from './Canvas';
import {commit, duplicate, redo, undo, type History} from './history';
import Inspector from '../components/Inspector';
import {categories, type Graph, type GraphNode, type NodePosition, type Snapshot, type Workspace} from '../types';

export default function WorkspaceEditor({workspace, snapshot, change, moveNodes, command, train, createProfile}: {
  workspace: Workspace; snapshot: Snapshot | null; change: (graph: Graph) => void;
  moveNodes: (positions: NodePosition[]) => unknown;
  command: (operation: string, data?: unknown) => unknown; train: (id: string) => void; createProfile: () => void;
}) {
  const [selected, select] = useState('');
  const [search, setSearch] = useState('');
  const history = useRef<History>({past: [], present: workspace.graph, future: []});
  const clipboard = useRef<Graph | null>(null);
  const graph = workspace.graph;
  const node = graph.nodes.find(n => n.id === selected);
  const edit = (next: Graph) => { history.current = commit({...history.current, present: graph}, next); change(next); };
  const move = (positions: NodePosition[]) => {
    const byId = new Map(positions.map(position => [position.id, position]));
    const next = {...graph, nodes: graph.nodes.map(node => ({...node, ...byId.get(node.id)}))};
    history.current = commit({...history.current, present: graph}, next);
    void moveNodes(positions);
  };
  const add = (kind: string) => {
    if (kind === 'Muscle Action' && !workspace.profiles.length) { createProfile(); return; }
    const id = crypto.randomUUID();
    const params: Record<string, unknown> = kind === 'Muscle Action' ? {profile_id: workspace.profiles[0].id} : {};
    if (kind.startsWith('Key ') || kind === 'Shortcut') params.key = 'Space';
    if (kind === 'Mouse Down' || kind === 'Mouse Up') params.button = 'left';
    edit({...graph, nodes: [...graph.nodes, {id, kind, label: kind, x: 100 + graph.nodes.length % 4 * 240, y: 80 + Math.floor(graph.nodes.length / 4) * 140, params}]});
    select(id);
  };
  const saveNode = (value: GraphNode) => edit({...graph, nodes: graph.nodes.map(n => n.id === value.id ? value : n)});
  return <div className="editor"><aside className="library panel"><h2>Node library</h2><input aria-label="Search nodes" placeholder="Search nodes..." value={search} onChange={e => setSearch(e.target.value)}/>
    {Object.entries(categories).map(([category, kinds]) => <section key={category}><h3>{category}</h3>{kinds.filter(k => k.toLowerCase().includes(search.toLowerCase())).map(kind => <button key={kind} onClick={() => add(kind)}>＋ {kind}</button>)}</section>)}
  </aside><div className="graph-column"><div className="graph-toolbar">
    <button onClick={() => { history.current = undo({...history.current, present: graph}); change(history.current.present); }}>Undo</button>
    <button onClick={() => { history.current = redo({...history.current, present: graph}); change(history.current.present); }}>Redo</button>
    <button disabled={!node} onClick={() => edit(duplicate(graph, [selected]))}>Duplicate</button>
    <button disabled={!node} onClick={() => { clipboard.current = {nodes: graph.nodes.filter(n => n.id === selected), edges: []}; }}>Copy</button>
    <button onClick={() => { if (clipboard.current) { const source = clipboard.current; const copied = duplicate(source, source.nodes.map(n => n.id)); edit({nodes: [...graph.nodes, ...copied.nodes.slice(source.nodes.length)], edges: graph.edges}); } }}>Paste</button>
    <span>Drag between ports to connect · Delete to remove</span>
  </div><div className="graph-canvas"><Canvas graph={graph} profiles={workspace.profiles} snapshot={snapshot} onChange={edit} onMove={move} onSelect={select}/></div>
  <details className="panel connections"><summary>Connections and event routing ({graph.edges.length})</summary>{graph.edges.map(edge => <label key={edge.id}>{graph.nodes.find(n => n.id === edge.source)?.label} → {graph.nodes.find(n => n.id === edge.target)?.label}<select aria-label={`Event for ${edge.id}`} value={edge.port} onChange={e => edit({...graph, edges: graph.edges.map(v => v.id === edge.id ? {...v, port: e.target.value} : v)})}>{['TRIGGER','START','ACTIVE','END','any','true','false','reset'].map(v => <option key={v}>{v}</option>)}</select></label>)}</details>
  </div>{node ? <Inspector node={node} profiles={workspace.profiles} save={saveNode} train={train} approve={() => { if (window.confirm(`Approve this saved ${node.kind} target for this session?\n${JSON.stringify(node.params, null, 2)}`)) command('approve', {id: node.id, confirmed: true}); }}/> : <aside className="inspector panel"><h2>Build your workflow</h2><p>Add an action profile, train it, and connect its node to an output.</p><p>Select a node to edit its settings. Graph edits are saved by the native backend and stop monitoring.</p></aside>}</div>;
}
