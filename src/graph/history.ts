import type {Graph} from '../types';
export interface History {past:Graph[];present:Graph;future:Graph[]}
export function commit(h:History,g:Graph):History{return {past:[...h.past,h.present].slice(-50),present:g,future:[]};}
export function undo(h:History):History{const previous=h.past.at(-1);return previous?{past:h.past.slice(0,-1),present:previous,future:[h.present,...h.future]}:h;}
export function redo(h:History):History{const next=h.future[0];return next?{past:[...h.past,h.present],present:next,future:h.future.slice(1)}:h;}
export function duplicate(g:Graph,ids:string[]):Graph{const mapping=new Map(ids.map(id=>[id,crypto.randomUUID()]));return {nodes:[...g.nodes,...g.nodes.filter(n=>mapping.has(n.id)).map(n=>({...n,id:mapping.get(n.id)!,x:n.x+40,y:n.y+40}))],edges:[...g.edges,...g.edges.filter(e=>mapping.has(e.source)&&mapping.has(e.target)).map(e=>({...e,id:crypto.randomUUID(),source:mapping.get(e.source)!,target:mapping.get(e.target)!}))]};}
