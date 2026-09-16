import {useState} from 'react';
import {Activity, Cable, GitBranch, Settings as SettingsIcon, Shield, Square, GraduationCap, Info} from 'lucide-react';
import {useWorkspace} from './useWorkspace';
import {api} from './api';
import WorkspaceEditor from './graph/WorkspaceEditor';
import Device from './components/Device';
import Training from './components/Training';
import Settings from './components/Settings';
import Signal from './components/Signal';
import type {Profile} from './types';

const pages = [{name:'Workspace', icon:GitBranch},{name:'Device',icon:Cable},{name:'Signal',icon:Activity},{name:'Training',icon:GraduationCap},{name:'Settings',icon:SettingsIcon},{name:'Diagnostics',icon:Shield},{name:'About',icon:Info}];
export default function App() {
  const {workspace, snapshot, error, busy, command, update, moveNodes, stop, run} = useWorkspace();
  const [page, setPage] = useState('Workspace');
  const [profileId, setProfileId] = useState('');
  const [name, setName] = useState('New muscle action');
  const profile = workspace?.profiles.find(p => p.id === profileId) ?? workspace?.profiles[0];
  const train = (id: string) => { setProfileId(id); setPage('Training'); };
  const navigate = (next: string) => {
    const training = snapshot?.training;
    if (next !== 'Training' && training?.profile_id && ['Capture','Train','Negative','Validate'].includes(training.stage)) {
      void command('training', {id: training.profile_id, action: 'Idle'}).then(result => {
        if (result !== undefined) setPage(next);
      });
    } else setPage(next);
  };
  const createProfile = () => void command('create_profile', {name}).then(p => { if (p) train((p as Profile).id); });
  return <div className="app"><header className="topbar"><div className="brand"><img src="/logo.png" alt="Aiden logo"/><div><strong>AIDEN</strong><small>Local signal. Deliberate action.</small></div></div><div className={`mode ${snapshot?.mode === 'Live' ? 'live' : ''}`}>{snapshot?.mode === 'Live' ? 'COMPUTER CONTROL ON' : snapshot?.mode === 'Test' ? 'TEST GRAPH · NO OS INPUT' : 'MONITORING OFF'}</div><div className="button-row"><button disabled={!workspace || busy} onClick={() => command('monitor', {mode:'Test'})}>Test Graph</button><button disabled={!workspace || busy || snapshot?.source !== 'hardware'} onClick={() => { if (window.confirm('Enable real mouse, keyboard and approved system actions? Press Ctrl+Shift+F12 to stop at any time.')) void command('monitor', {mode:'Live',confirmed:true}); }}>Enable control</button><button className="danger stop" onClick={stop}><Square size={14}/>Emergency stop</button></div></header>
    <nav>{pages.map(({name,icon:Icon}) => <button key={name} className={page === name ? 'active' : ''} onClick={() => navigate(name)}><Icon size={17}/>{name}</button>)}</nav>
    {snapshot?.recognition_status && <div className="control-feedback" role="status">{snapshot.recognition_status}</div>}
    {(error || snapshot?.error) && <div role="alert" className="error-banner">{error || snapshot?.error}<button onClick={() => command('acknowledge')}>Acknowledge</button></div>}
    {snapshot?.diagnostics.recovered === true && <div className="recovery-banner">The previous session ended unexpectedly. The last committed workspace has been recovered; monitoring remains off. Export or back up your data in Settings.</div>}
    <main>{!workspace ? <section className="panel loading"><h1>Opening Aiden</h1><p>{error ? 'The native backend is unavailable. Launch the desktop application with npm run tauri dev, then review the error above.' : 'Loading your local workspace...'}</p></section> : <>
      {page === 'Workspace' && <WorkspaceEditor workspace={workspace} snapshot={snapshot} change={graph => void update(w => ({...w, graph}))} moveNodes={moveNodes} command={command} train={train} createProfile={createProfile}/>}
      {page === 'Device' && <Device snapshot={snapshot} command={command}/>}
      {page === 'Signal' && <section className="panel"><div className="section-title"><h1>Signal monitor</h1><button onClick={() => command('mark')}>Mark event</button></div><Signal points={snapshot?.points ?? []} onset={workspace.dsp.onset}/><p>Envelope: {snapshot?.envelope.toFixed(4) ?? '—'} · Noise: {snapshot?.noise.toFixed(4) ?? '—'} · {snapshot?.rate} Hz</p><h3>Last completed segment scores</h3>{snapshot?.scores.map(([id,score]) => <p key={id}>{workspace.profiles.find(p => p.id === id)?.name ?? id}: {(score*100).toFixed(1)}%</p>)}</section>}
      {page === 'Training' && <><div className="page-toolbar"><select aria-label="Action profile" value={profile?.id ?? ''} onChange={e => setProfileId(e.target.value)}>{workspace.profiles.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}</select><input aria-label="New profile name" maxLength={100} value={name} onChange={e => setName(e.target.value)}/><button onClick={createProfile}>Create profile</button><button onClick={() => command('load')}>Refresh saved examples</button></div>{profile ? <Training profile={profile} snapshot={snapshot} save={p => void update(w => ({...w, profiles:w.profiles.map(existing => existing.id === p.id ? {...p, examples:existing.examples, validation:existing.validation} : existing)}))} command={command}/> : <section className="panel"><h1>Create your first action</h1><p>Connect a source on the Device page, then create a profile to capture its reference signal.</p></section>}</>}
      {page === 'Settings' && <Settings workspace={workspace} snapshot={snapshot} update={update} command={command} run={run}/>}
      {page === 'Diagnostics' && <section className="panel"><div className="section-title"><h1>Diagnostics</h1><button onClick={() => run(() => navigator.clipboard.writeText(JSON.stringify(snapshot?.diagnostics, null, 2)))}>Copy diagnostics</button></div><dl>{Object.entries(snapshot?.diagnostics ?? {}).map(([key,value]) => <div key={key}><dt>{key}</dt><dd>{String(value)}</dd></div>)}</dl><h2>Native graph events</h2><div className="trace-list">{snapshot?.traces.slice().reverse().map((t,i) => <div key={`${t.time_ms}-${i}`}><span>{t.time_ms} ms</span><span>{workspace.graph.nodes.find(n => n.id === t.node)?.label ?? t.node}</span><span>{t.kind}</span><span>{t.result}</span></div>)}</div></section>}
      {page === 'About' && <section className="panel about"><img src="/logo.png" alt="Aiden logo"/><h1>Aiden</h1><p>Created by Sam (17ofSeptember)</p><p><a href="https://github.com/17ofSeptember" onClick={e => { e.preventDefault(); void run(async () => { await navigator.clipboard.writeText('https://github.com/17ofSeptember'); }); }}>github.com/17ofSeptember (copy link)</a></p><p>awrynetwork@gmail.com</p><p>Deterministic muscle-action recognition. Runs locally without AI, cloud processing, analytics, or telemetry.</p><p>Not a medical device. Not intended for diagnosis or measuring health conditions. AD8232 signals are used only for human-computer interaction.</p><small>Version 0.1.1 · Windows / Linux X11 · Wayland injection unavailable</small></section>}
    </>}</main><footer><span className={snapshot?.device === 'Connected' ? 'connected' : ''}>● {snapshot?.device ?? 'Backend unavailable'}</span><span>{snapshot?.rate ?? 0} Hz</span><span>{busy ? 'Saving / processing...' : 'Local workspace'}</span><span>Last match: {snapshot?.last_match || 'None'}</span><span>Ctrl + Shift + F12 to stop</span><button onClick={() => void api('stop').catch(() => stop())}>Stop monitoring</button></footer>
  </div>;
}
