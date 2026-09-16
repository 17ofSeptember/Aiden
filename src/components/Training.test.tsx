// @vitest-environment jsdom
import {cleanup, fireEvent, render, screen} from '@testing-library/react';
import {afterEach, expect, it, vi} from 'vitest';
import Training from './Training';
import fixture from '../../fixtures/simulation-workspace.json';
import type {Profile, Snapshot} from '../types';
vi.mock('./Signal', () => ({default:()=>null}));
afterEach(() => {cleanup(); vi.restoreAllMocks();});
it('routes capture, review and training actions to the selected native profile', () => {
  const profile=structuredClone(fixture.profiles[0]) as Profile;
  const command=vi.fn();
  const save=vi.fn();
  const snapshot={training:{profile_id:profile.id,stage:'Review',candidate:profile.examples[0],capture:[],accepted:0,rejected:0,score:1},points:[]} as unknown as Snapshot;
  render(<Training profile={profile} snapshot={snapshot} command={command} save={save}/>);
  for(const [label,action] of [['Read reference','Capture'],['Done','Done'],['Accept candidate','Accept'],['Train repetitions','Train'],['Pause','Paused'],['Resume','Train']]) {
    fireEvent.click(screen.getByRole('button',{name:label}));
    expect(command).toHaveBeenLastCalledWith('training',{id:profile.id,action,variant:'Default'});
  }
  fireEvent.change(screen.getByLabelText('Name'),{target:{value:'Forearm'}});
  fireEvent.click(screen.getByRole('button',{name:'Save profile settings'}));
  expect(save).toHaveBeenCalledWith(expect.objectContaining({name:'Forearm',examples:profile.examples}));
});
it('requires confirmation to clear examples and disables training without a reference', () => {
  const profile={...structuredClone(fixture.profiles[0]),examples:[]} as Profile;
  const command=vi.fn();
  const confirm=vi.spyOn(window,'confirm').mockReturnValue(false);
  render(<Training profile={profile} snapshot={null} command={command} save={vi.fn()}/>);
  expect((screen.getByRole('button',{name:'Train repetitions'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'Clear training data'}));
  expect(command).not.toHaveBeenCalled();
  confirm.mockReturnValue(true);
  fireEvent.click(screen.getByRole('button',{name:'Clear training data'}));
  expect(command).toHaveBeenCalledWith('training',expect.objectContaining({action:'Clear',confirmed:true}));
});
