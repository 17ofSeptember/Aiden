// @vitest-environment jsdom
import {cleanup, fireEvent, render, screen} from '@testing-library/react';
import {afterEach, expect, it, vi} from 'vitest';
import App from './App';
import {useWorkspace} from './useWorkspace';
import fixture from '../fixtures/simulation-workspace.json';
vi.mock('./useWorkspace',()=>({useWorkspace:vi.fn()}));
vi.mock('./graph/WorkspaceEditor',()=>({default:()=>null}));
afterEach(()=>{cleanup();vi.restoreAllMocks();});
it('requires hardware and confirmation for live control while stop stays accessible',()=>{
  const command=vi.fn(),stop=vi.fn();
  const state={workspace:fixture,snapshot:{mode:'Off',source:'contraction',diagnostics:{},error:''},error:'',busy:false,command,stop,update:vi.fn(),run:vi.fn()};
  vi.mocked(useWorkspace).mockReturnValue(state as unknown as ReturnType<typeof useWorkspace>);
  const {rerender}=render(<App/>);
  expect((screen.getByRole('button',{name:'Enable control'}) as HTMLButtonElement).disabled).toBe(true);
  state.snapshot.source='hardware';
  rerender(<App/>);
  const confirm=vi.spyOn(window,'confirm').mockReturnValue(false);
  fireEvent.click(screen.getByRole('button',{name:'Enable control'}));
  expect(command).not.toHaveBeenCalled();
  confirm.mockReturnValue(true);
  fireEvent.click(screen.getByRole('button',{name:'Enable control'}));
  expect(command).toHaveBeenCalledWith('monitor',{mode:'Live',confirmed:true});
  fireEvent.click(screen.getByRole('button',{name:'Emergency stop'}));
  expect(stop).toHaveBeenCalledOnce();
});
