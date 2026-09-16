// @vitest-environment jsdom
import {act, cleanup, renderHook, waitFor} from '@testing-library/react';
import {afterEach, expect, it, vi} from 'vitest';
import {useWorkspace} from './useWorkspace';
import {api} from './api';
import fixture from '../fixtures/simulation-workspace.json';
import type {Workspace} from './types';

vi.mock('./api', () => ({api:vi.fn()}));
afterEach(() => {cleanup(); vi.resetAllMocks();});
it('uses layout patches and does not let an older command erase a pending move', async () => {
  let stored = structuredClone(fixture) as Workspace;
  let release: (() => void) | undefined;
  const gate = new Promise<void>(resolve => {release = resolve;});
  vi.mocked(api).mockImplementation(async (op, data) => {
    if (op === 'load') return structuredClone(stored);
    if (op === 'snapshot') return {};
    if (op === 'mark') {await gate; return true;}
    if (op === 'layout') {
      const {positions} = data as {positions:{id:string;x:number;y:number}[]};
      stored = {...stored, graph: {...stored.graph, nodes: stored.graph.nodes.map(node => ({...node, ...positions.find(position => position.id === node.id)}))}};
      return true;
    }
    return true;
  });
  const {result} = renderHook(useWorkspace);
  await waitFor(() => expect(result.current.workspace).not.toBeNull());
  const id = stored.graph.nodes[0].id;
  let command: Promise<unknown>, move: Promise<unknown>;
  act(() => {
    command = result.current.command('mark');
    move = result.current.moveNodes([{id, x: 300, y: 200}]);
  });
  expect(result.current.workspace?.graph.nodes[0]).toMatchObject({x:300,y:200});
  await act(async () => {release?.(); await command; await move;});
  expect(result.current.workspace?.graph.nodes[0]).toMatchObject({x:300,y:200});
  expect(stored.profiles).toEqual(fixture.profiles);
  expect(api).not.toHaveBeenCalledWith('save', expect.anything());
});
it('keeps rapid edits visible while native saves are pending and preserves both nodes', async () => {
  let stored = structuredClone(fixture) as Workspace;
  let release: (() => void) | undefined;
  const gate = new Promise<void>(resolve => {release=resolve;});
  let saves=0;
  vi.mocked(api).mockImplementation(async (op, data) => {
    if (op === 'load') return structuredClone(stored);
    if (op === 'snapshot') return {};
    if (op === 'save') {if (++saves===1) await gate; stored=structuredClone(data) as Workspace; return true;}
    return true;
  });
  const {result}=renderHook(useWorkspace);
  await waitFor(() => expect(result.current.workspace).not.toBeNull());
  const add=(id:string) => (w:Workspace):Workspace => ({...w,graph:{...w.graph,nodes:[...w.graph.nodes,{id,kind:'Debug',label:id,x:0,y:0,params:{}}]}});
  act(() => {void result.current.update(add('one'));});
  act(() => {void result.current.update(add('two'));});
  expect(result.current.workspace?.graph.nodes.slice(-2).map(n=>n.id)).toEqual(['one','two']);
  act(() => release?.());
  await waitFor(() => expect(saves).toBe(2));
  await waitFor(() => expect(result.current.busy).toBe(false));
  expect(stored.graph.nodes.slice(-2).map(n=>n.id)).toEqual(['one','two']);
});
it('bypasses pending saves for emergency stop and restores rejected edits', async () => {
  const stored=structuredClone(fixture) as Workspace;
  vi.mocked(api).mockImplementation(async (op) => {
    if(op==='load') return structuredClone(stored);
    if(op==='snapshot') return {};
    if(op==='save') throw new Error('Invalid graph');
    return true;
  });
  const {result}=renderHook(useWorkspace);
  await waitFor(() => expect(result.current.workspace).not.toBeNull());
  await act(async () => {await result.current.update(w=>({...w,graph:{nodes:[],edges:[]}})); result.current.stop();});
  expect(result.current.workspace).toEqual(stored);
  expect(result.current.error).toContain('Invalid graph');
  expect(api).toHaveBeenCalledWith('stop');
});
