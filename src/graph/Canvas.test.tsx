// @vitest-environment jsdom
import {act, cleanup, render} from '@testing-library/react';
import {afterEach, expect, it, vi} from 'vitest';
import type {ReactFlowProps, Node} from '@xyflow/react';
import Canvas from './Canvas';
import fixture from '../../fixtures/simulation-workspace.json';
import type {Workspace} from '../types';
let flow: ReactFlowProps;
vi.mock('@xyflow/react', async () => {
  const original = await vi.importActual<typeof import('@xyflow/react')>('@xyflow/react');
  return {...original, ReactFlow: (props: ReactFlowProps) => {flow = props; return null;}};
});
afterEach(cleanup);
it('keeps nodes and measured dimensions through dragging and persists once on drop', () => {
  const workspace = structuredClone(fixture) as Workspace;
  const onChange = vi.fn(), onMove = vi.fn();
  const props = {graph: workspace.graph, profiles: workspace.profiles, snapshot: null, onChange, onMove, onSelect: vi.fn()};
  const {rerender} = render(<Canvas {...props}/>);
  const id = workspace.graph.nodes[0].id;
  act(() => flow.onNodesChange?.([
    {id, type: 'dimensions', dimensions: {width: 185, height: 90}},
    {id, type: 'select', selected: true},
    {id, type: 'position', position: {x: 120, y: 140}, dragging: true},
  ]));
  rerender(<Canvas {...props} graph={structuredClone(workspace.graph)}/>);
  expect(flow.nodes).toHaveLength(workspace.graph.nodes.length);
  expect(flow.nodes?.[0]).toMatchObject({position: {x: 120, y: 140}, measured: {width: 185, height: 90}, selected: true, dragging: true});
  expect(onChange).not.toHaveBeenCalled();
  expect(onMove).not.toHaveBeenCalled();
  const node = {...flow.nodes![0], position: {x: 200, y: 220}} as Node;
  act(() => flow.onNodeDragStop?.(new MouseEvent('mouseup'), node, [node]));
  expect(onMove).toHaveBeenCalledExactlyOnceWith([{id, x: 200, y: 220}]);
  expect(flow.nodes?.[0]).toMatchObject({position: {x: 200, y: 220}, dragging: false});
  expect(flow.nodes).toHaveLength(workspace.graph.nodes.length);
});
it('deletes only the selected node and its incident edges', () => {
  const workspace = structuredClone(fixture) as Workspace;
  const onChange = vi.fn();
  render(<Canvas graph={workspace.graph} profiles={workspace.profiles} snapshot={null} onChange={onChange} onMove={vi.fn()} onSelect={vi.fn()}/>);
  const id = workspace.graph.nodes[0].id;
  act(() => flow.onNodesChange?.([{id, type: 'remove'}]));
  const next = onChange.mock.calls[0][0];
  expect(next.nodes).toHaveLength(workspace.graph.nodes.length - 1);
  expect(next.nodes.some((node: Node) => node.id === id)).toBe(false);
  expect(next.edges.every((edge: {source:string;target:string}) => edge.source !== id && edge.target !== id)).toBe(true);
});
