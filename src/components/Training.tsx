import {useEffect, useState} from 'react';
import {download} from '../api';
import Signal from './Signal';
import type {Profile, Snapshot} from '../types';

export default function Training({profile, snapshot, save, command}: {
  profile: Profile; snapshot: Snapshot | null; save: (profile: Profile) => void;
  command: (operation: string, data?: unknown) => unknown;
}) {
  const [draft, setDraft] = useState(profile);
  const [variant, setVariant] = useState('Default');
  const [cropStart, setCropStart] = useState(0);
  const [cropEnd, setCropEnd] = useState(500);
  useEffect(() => setDraft(profile), [profile]);
  const status = snapshot?.training.profile_id === profile.id ? snapshot.training : null;
  const action = (action: string, extra: Record<string, unknown> = {}) => command('training', {id: profile.id, action, variant, ...extra});
  const count = profile.examples.filter(e => !e.negative).length;
  const setting = (key: 'sensitivity'|'threshold'|'ambiguity'|'cooldown_ms'|'min_ms'|'max_ms', label: string, step: number) => <label>{label}<input type="number" step={step} value={draft[key]} onChange={e => setDraft({...draft, [key]: e.target.valueAsNumber})}/></label>;
  return <div className="training-layout"><section className="panel"><div className="section-title"><h2>{profile.name}</h2><span className="badge">{status?.stage ?? 'Idle'} · Control off during training</span></div>
    <Signal points={status?.stage === 'Review' ? status.capture : snapshot?.points ?? []} onset={profile.dsp.onset}/>
    <p>Relax for the baseline. Click Read, wait three seconds, perform one contraction, then click Done. Review and accept the reference before training repetitions.</p>
    <div className="metrics"><div><strong>{count}</strong>references saved</div><div><strong>{status?.accepted ?? 0}</strong>accepted this session</div><div><strong>{status?.rejected ?? 0}</strong>rejected</div><div><strong>{status ? (status.score * 100).toFixed(1) + '%' : '—'}</strong>template similarity</div></div>
    <label>Signal variant<input maxLength={80} value={variant} onChange={e => setVariant(e.target.value)}/></label>
    <div className="button-row"><button className="primary" onClick={() => action('Capture')}>Read reference</button><button onClick={() => action('Done')}>Done</button><button disabled={!count} onClick={() => action('Train')}>Train repetitions</button><button onClick={() => action('Paused')}>Pause</button><button disabled={!count} onClick={() => action('Train')}>Resume</button><button onClick={() => action('Idle')}>Stop training</button></div>
    {status?.stage === 'Capture' && <p role="status">Recording begins after three seconds. Relax first, then perform the action. The waveform above shows incoming samples.</p>}
    {status?.candidate && <section className="candidate"><h3>Candidate review</h3><p>{status.candidate.duration_ms.toFixed(0)} ms · Peak {status.candidate.peak.toFixed(4)} · RMS {status.candidate.rms.toFixed(4)}</p><div className="button-row"><button onClick={() => action('Accept')}>Accept candidate</button><button onClick={() => action('Reject')}>Reject candidate</button><button onClick={() => action('Undo')}>Undo last accepted</button></div></section>}
    {status?.stage === 'Review' && <div className="button-row"><label>Start sample<input type="number" min={0} value={cropStart} onChange={e => setCropStart(e.target.valueAsNumber)}/></label><label>End sample (exclusive)<input type="number" min={10} value={cropEnd} onChange={e => setCropEnd(e.target.valueAsNumber)}/></label><button onClick={() => action('Crop', {start: cropStart, end: cropEnd})}>Adjust segment</button></div>}
    <details><summary>Negative examples and validation</summary><p>Negative mode records detected non-target contractions. Use a separate validation run to count detections, then mark missed and false activations yourself.</p><div className="button-row"><button onClick={() => action('Negative')}>Train non-target movements</button><button disabled={!count} onClick={() => action('Validate')}>Validate</button><button onClick={() => action('Missed')}>Mark missed action</button><button onClick={() => action('False')}>Mark false activation</button></div><p>Detected: {profile.validation.detected} · Missed: {profile.validation.missed} · False: {profile.validation.false_triggers}. Counts are user-reviewed; they are not an accuracy guarantee.</p></details>
    <details><summary>Training examples ({profile.examples.length})</summary><div className="example-list">{profile.examples.map(e => <div key={e.id}><svg viewBox="0 0 128 40" aria-label="Normalized example envelope"><polyline fill="none" stroke="#b897ff" strokeWidth="1.5" points={e.shape.map((v,i) => `${i*2},${38-v*35}`).join(' ')}/></svg><span>{e.variant} · {e.negative ? 'Negative' : 'Reference'} · {e.duration_ms.toFixed(0)} ms · {e.peak.toFixed(3)}</span><button onClick={() => action('Delete', {example_id: e.id})}>Delete example</button></div>)}</div></details>
  </section><aside className="panel"><h2>Profile settings</h2><label>Name<input maxLength={100} value={draft.name} onChange={e => setDraft({...draft, name: e.target.value})}/></label><label>Description<textarea value={draft.description} onChange={e => setDraft({...draft, description: e.target.value})}/></label><label>Electrode placement notes<textarea value={draft.notes} onChange={e => setDraft({...draft, notes: e.target.value})}/></label><label><input type="checkbox" checked={draft.enabled} onChange={e => setDraft({...draft, enabled: e.target.checked})}/>Enabled</label><label>Trigger mode<select value={draft.mode} onChange={e => setDraft({...draft, mode: e.target.value as Profile['mode']})}>{['Discrete','Hold','Continuous'].map(v => <option key={v}>{v}</option>)}</select></label>
    {setting('threshold','Match threshold',.01)}{setting('ambiguity','Ambiguity margin',.01)}{setting('sensitivity','Sensitivity',.05)}{setting('cooldown_ms','Cooldown (ms)',10)}{setting('min_ms','Minimum duration (ms)',10)}{setting('max_ms','Maximum duration (ms)',10)}
    <button className="primary" onClick={() => save({...draft, examples: profile.examples})}>Save profile settings</button><button onClick={() => download(`${profile.name}.aiden-profile.json`, {version: 1, profile})}>Export profile</button><button className="danger" onClick={() => { if (window.confirm('Delete all training data for this profile? Node wiring will be preserved.')) action('Clear', {confirmed: true}); }}>Clear training data</button>
  </aside></div>;
}
