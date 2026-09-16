import {describe, expect, it} from 'vitest';
import {commit, duplicate, redo, undo, type History} from './history';
import type {Graph} from '../types';

const graph: Graph = {nodes: ['a', 'b', 'c'].map(id => ({id, kind:'Debug', label:id, x:0, y:0, params:{}})), edges:[{id:'ab',source:'a',target:'b',port:'TRIGGER'},{id:'bc',source:'b',target:'c',port:'END'}]};
describe('graph editing history', () => {
  it('duplicates internal wiring with fresh IDs and leaves external connections alone', () => {
    const copy = duplicate(graph, ['a','b']);
    expect(copy.nodes).toHaveLength(5);
    expect(copy.edges).toHaveLength(3);
    expect(copy.edges[2]).toMatchObject({source:copy.nodes[3].id,target:copy.nodes[4].id,port:'TRIGGER'});
    expect(new Set(copy.nodes.map(n => n.id)).size).toBe(5);
    expect(graph.nodes).toHaveLength(3);
  });
  it('restores edits, discards abandoned redo branches and bounds history', () => {
    const initial: History = {past:[], present:graph, future:[]};
    const changed = duplicate(graph,['a']);
    expect(undo(commit(initial,changed)).present).toEqual(graph);
    expect(redo(undo(commit(initial,changed))).present).toEqual(changed);
    expect(commit(undo(commit(initial,changed)),graph).future).toEqual([]);
    let history = initial;
    for (let i=0;i<80;i++) history=commit(history,graph);
    expect(history.past).toHaveLength(50);
  });
});
