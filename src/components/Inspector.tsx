import {useEffect, useState} from 'react';
import type {GraphNode, Profile} from '../types';

const system = ['Launch Application', 'Approved Command', 'Open File', 'Open Folder', 'Open URL'];
export default function Inspector({node, profiles, save, approve, train}: {
  node: GraphNode; profiles: Profile[]; save: (node: GraphNode) => void;
  approve: () => void; train: (id: string) => void;
}) {
  const [draft, setDraft] = useState(node);
  const [args, setArgs] = useState('');
  const [error, setError] = useState('');
  useEffect(() => { setDraft(node); setArgs(JSON.stringify(node.params.args ?? [])); setError(''); }, [node]);
  const param = (key: string, value: unknown) => setDraft(d => ({...d, params: {...d.params, [key]: value}}));
  const text = (key: string, label: string) => <label>{label}<input value={String(draft.params[key] ?? '')} onChange={e => param(key, e.target.value)}/></label>;
  const number = (key: string, label: string, fallback: number) => <label>{label}<input type="number" step="any" value={Number(draft.params[key] ?? fallback)} onChange={e => param(key, e.target.valueAsNumber)}/></label>;
  const choice = (key: string, label: string, values: string[]) => <label>{label}<select value={String(draft.params[key] ?? values[0])} onChange={e => param(key, e.target.value)}>{values.map(v => <option key={v}>{v}</option>)}</select></label>;
  const submit = () => {
    if (system.includes(node.kind)) {
      try {
        const parsed: unknown = JSON.parse(args);
        if (!Array.isArray(parsed) || !parsed.every(v => typeof v === 'string')) throw new Error('Arguments must be a JSON array of strings.');
        save({...draft, params: {...draft.params, args: parsed}});
      } catch (e) { setError(String(e)); }
    } else save(draft);
  };
  return <aside className="inspector panel"><h2>Node settings</h2><p className="eyebrow">{node.kind}</p>
    <label>Label<input maxLength={100} value={draft.label} onChange={e => setDraft({...draft, label: e.target.value})}/></label>
    {node.kind === 'Muscle Action' && <><label>Action profile<select value={String(draft.params.profile_id)} onChange={e => param('profile_id', e.target.value)}>{profiles.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}</select></label><button onClick={() => train(String(node.params.profile_id))}>Open training</button></>}
    {['Key Press','Key Down','Key Up','Shortcut'].includes(node.kind) && text('key', 'Key / shortcut')}
    {node.kind === 'Type Text' && text('text', 'Text')}
    {['Mouse Down','Mouse Up'].includes(node.kind) && choice('button', 'Button', ['left','right','middle'])}
    {['Delay','Timer','Interval','Debounce','Cooldown','Pulse','Hold','Key Press'].includes(node.kind) && number('ms', 'Duration (ms)', 500)}
    {node.kind === 'Repeat' && <>{number('count', 'Count', 2)}{number('interval', 'Interval (ms)', 100)}</>}
    {node.kind === 'Move Mouse' && <>{choice('mode', 'Movement', ['fixed','strength'])}{number('x', 'X direction (-1 to 1)', 1)}{number('y', 'Y direction (-1 to 1)', 0)}{number('speed', 'Pixels per event', 10)}{number('max_speed', 'Maximum pixels per event', 100)}{number('dead_zone', 'Strength dead zone', .05)}<p>Connect ACTIVE events for sustained movement at the native recognition update rate.</p></>}
    {['Scroll Vertical','Scroll Horizontal','Compare','Branch'].includes(node.kind) && number('value', 'Value', 1)}
    {['Compare','Branch'].includes(node.kind) && choice('operator', 'Comparison', ['gte','lt','eq'])}
    {node.kind === 'Gate' && <label><input type="checkbox" checked={draft.params.open !== false} onChange={e => param('open', e.target.checked)}/>Gate open</label>}
    {system.includes(node.kind) && <>{text('target', node.kind === 'Open URL' ? 'HTTP(S) URL' : 'Absolute target path')}<label>Arguments (JSON string array)<textarea value={args} onChange={e => setArgs(e.target.value)}/></label><p>Save first, then approve the exact target for this session.</p><button onClick={approve}>Review and approve saved target</button></>}
    {error && <p role="alert">{error}</p>}<button className="primary" onClick={submit}>Save node settings</button>
  </aside>;
}
