import { afterEach, expect, it, vi } from 'vitest';
import { spineBuild } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { emptyCase } from '@/case-state/empty-case';
import step from './step';
vi.mock('@/bench/three/build-spine',()=>({hasCurrentFit:()=>true}));
afterEach(()=>{spineBuild.getState().clear();threeState.getState().clear();});
it('keeps Place unavailable until both the pending and running move have ended',()=>{
  const accepted={key:'test',movePending:true} as never;
  spineBuild.getState().accept(accepted);
  expect(step.isReady(emptyCase())).toBe(false);
  spineBuild.setState({accepted:{key:'test',movePending:false} as never});threeState.getState().move(true);
  expect(step.isReady(emptyCase())).toBe(false);
  threeState.getState().move(false);expect(step.isReady(emptyCase())).toBe(true);
});
