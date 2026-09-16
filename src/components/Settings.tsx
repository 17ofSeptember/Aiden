import {useState} from 'react';
import {api, download} from '../api';
import type {Dsp, Profile, Snapshot, Workspace} from '../types';
interface Recording {id: string; name: string; rate: number; count: number}
export default function Settings({workspace, snapshot, update, command, run}: {
  workspace: Workspace; snapshot: Snapshot | null; update: (fn: (w: Workspace) => Workspace) => unknown;
  command: (op: string, data?: unknown) => unknown; run: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
}) {
  const [dsp, setDsp] = useState(workspace.dsp);
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [notice, setNotice] = useState('');
  const refresh = () => run(async () => setRecordings(await api<Recording[]>('recordings')));
  const importFile = async (file: File, profile: boolean) => {
    if (file.size > 20_000_000) { setNotice('Import is limited to 20 MB.'); return; }
    await run(async () => {
      const data = JSON.parse(await file.text());
      if (profile) {
        if (data.version !== 1 || !data.profile) throw new Error('Expected a version 1 profile export.');
        const imported: Profile = data.profile;
        const w = await api<Workspace>('load');
        imported.id = crypto.randomUUID();
        imported.enabled = false;
        imported.examples = imported.examples.map(e => ({...e, id: crypto.randomUUID()}));
        await api('import', {...w, profiles: [...w.profiles, imported]});
      } else await api('import', data);
    });
    await command('load');
  };
  return <div className="two-column"><section className="panel"><h2>Signal processing</h2><p>Changing DSP settings requires clearing or exporting trained profiles first. Reconnect the source after changing its sample rate.</p><div className="form-grid">{Object.entries(dsp).map(([key,value]) => <label key={key}>{key.replaceAll('_',' ')}<input type="number" step="any" value={value} onChange={e => setDsp({...dsp, [key]: e.target.valueAsNumber} as Dsp)}/></label>)}</div><button className="primary" onClick={() => update(w => ({...w, dsp, profiles: w.profiles.map(p => ({...p, dsp}))}))}>Apply DSP configuration</button><h3>Safety</h3><p>Monitoring starts off. Simulation and playback are restricted to Test Graph. Global emergency stop: <kbd>Ctrl + Shift + F12</kbd>.</p></section>
    <section className="panel"><h2>Data and recordings</h2><div className="button-row"><button onClick={() => run(async () => download('workspace.aiden.json', await api('export')))}>Export workspace</button><button onClick={() => run(async () => setNotice(`Backup saved: ${await api<string>('backup')}`))}>Back up database</button></div><label>Import / restore workspace<input type="file" accept=".json" onChange={e => { const f = e.target.files?.[0]; if (f && window.confirm('Replace the current workspace? A database backup will be created first.')) void importFile(f, false); e.target.value = ''; }}/></label><label>Import profile<input type="file" accept=".json" onChange={e => { const f = e.target.files?.[0]; if (f && window.confirm('Imported profiles may not match your electrode placement. Import disabled for review?')) void importFile(f, true); e.target.value = ''; }}/></label>
    <h3>Raw signal recordings</h3><div className="button-row"><button disabled={snapshot?.recording != null} onClick={() => command('record_start', {name: `Session ${new Date().toLocaleString()}`})}>Record signal</button><button disabled={snapshot?.recording == null} onClick={() => command('record_stop')}>Stop and save recording</button><button onClick={() => void refresh()}>Refresh recordings</button></div><p>{snapshot?.recording != null ? `${snapshot.recording} samples buffered (maximum 600000)` : 'Recording off'}</p>{recordings.map(r => <div className="recording" key={r.id}><span>{r.name} · {r.rate} Hz · {r.count} samples</span><button onClick={() => command('connect', {kind: 'playback', id: r.id})}>Replay in Test mode</button></div>)}{notice && <p role="status">{notice}</p>}</section></div>;
}
