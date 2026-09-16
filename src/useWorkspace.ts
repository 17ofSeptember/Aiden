import {useCallback, useEffect, useRef, useState} from 'react';
import {api} from './api';
import type {NodePosition, Snapshot, Workspace} from './types';

/** Serialize mutations so a slow autosave cannot overwrite a newer training result. */
export function useWorkspace() {
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const current = useRef<Workspace | null>(null);
  const revision = useRef(0);
  const install = useCallback((value: Workspace) => {
    current.current = value;
    setWorkspace(value);
  }, []);
  const run = useCallback(<T,>(task: () => Promise<T>): Promise<T | undefined> => {
    const result = queue.current.then(async () => {
      setBusy(true);
      try { setError(''); return await task(); }
      catch (reason) { setError(String(reason)); return undefined; }
      finally { setBusy(false); }
    });
    queue.current = result;
    return result;
  }, []);
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    void api<Workspace>('load').then(w => { if (!disposed) install(w); }).catch(e => { if (!disposed) setError(String(e)); });
    const poll = async () => {
      try { const s = await api<Snapshot>('snapshot'); if (!disposed && s.mode) setSnapshot(s); }
      catch (e) { if (!disposed) setError(String(e)); }
      if (!disposed) timer = setTimeout(() => void poll(), 100);
    };
    void poll();
    return () => { disposed = true; clearTimeout(timer); };
  }, [install]);
  const command = useCallback((operation: string, data: unknown = null) => {
    const ticket = revision.current;
    return run(async () => {
    const result = await api(operation, data);
    const loaded = await api<Workspace>('load');
    if (ticket === revision.current) install(loaded);
    return result;
  });
  }, [install, run]);
  const moveNodes = useCallback((positions: NodePosition[]) => {
    const ticket = ++revision.current;
    const moved = new Map(positions.map(position => [position.id, position]));
    if (current.current) install({...current.current, graph: {...current.current.graph,
      nodes: current.current.graph.nodes.map(node => ({...node, ...moved.get(node.id)}))}});
    return run(async () => {
      try { await api('layout', {positions}); }
      finally {
        const loaded = await api<Workspace>('load');
        if (ticket === revision.current) install(loaded);
      }
    });
  }, [install, run]);
  const update = useCallback((change: (w: Workspace) => Workspace) => {
    const ticket = ++revision.current;
    if (current.current) install(change(current.current));
    return run(async () => {
    // Always read authoritative profiles: repeated training changes them in Rust.
    const latest = await api<Workspace>('load');
    const next = change(latest);
    try {
      await api('save', next);
      if (ticket === revision.current) install(next);
    } catch (reason) {
      if (ticket === revision.current) install(await api<Workspace>('load'));
      throw reason;
    }
    });
  }, [install, run]);
  const stop = useCallback(() => {
    // Emergency stop deliberately bypasses the mutation queue.
    void api('stop').catch(e => setError(String(e)));
  }, []);
  return {workspace, snapshot, error, busy, command, update, moveNodes, stop, run, current};
}
