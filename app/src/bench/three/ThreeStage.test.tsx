import { act, useImperativeHandle } from 'react';
import { afterEach, expect, it, vi } from 'vitest';
import { putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import type { Spine3DProps } from '@/spine-3d/view/Spine3D';
import { currentStep } from '@/view-state/current-step';
import { threeState } from '@/view-state/three-state';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { spineBuild } from '@/view-state/spine-build';
import { ThreeStage } from './ThreeStage';
const heard = vi.hoisted(() => ({ props: null as Spine3DProps | null, look: vi.fn(), wall: vi.fn(), fail: false }));
vi.mock('@/spine-3d/view/Spine3D', () => ({ Spine3D: function MockSpine(props: Spine3DProps) {
  useImperativeHandle(props.ref, () => ({ stage: () => ({ film: () => ({ set: heard.wall }) }), spine: () => null, lookFrom: heard.look, settle: () => undefined }) as never);
  heard.props = props;
  if (heard.fail) throw new Error('drawing unavailable');
  return <canvas />;
} }));
function accepted(): AcceptedSpine {
  return { key: 'test', fit: MADE_UP_CASE.fit, bones: [], input: {
    front: { marks: { film: 'front', vertebrae: [] } as never, geometry: MADE_UP_CASE.front.geometry, size: MADE_UP_CASE.front.size, markSpread: MADE_UP_CASE.front.markSpread, facingAssumed: true },
    side: null, typical: 'built-in', twistReadings: [],
  } };
}
afterEach(() => { heard.fail = false; heard.look.mockClear(); heard.wall.mockClear(); threeState.getState().clear(); spineBuild.getState().clear(); vi.restoreAllMocks(); });
it('draws only the fitted standard bones, hides film walls and drives named views without replacing the fit', () => {
  const source = accepted();
  const first = source.fit.vertebrae[0]; if (first === undefined) throw new Error('no fitted bone');
  source.fit = { ...source.fit, vertebrae: [{ ...first, level: 'C7' }, ...source.fit.vertebrae] };
  threeState.getState().clear();
  const page = putOnThePage(<ThreeStage accepted={source} />);
  const props = heard.props;
  expect(props?.view).toBe('three-quarter'); expect(props?.shadow).toBe(true);
  expect(props?.fit.vertebrae).toHaveLength(17); expect(source.fit.vertebrae).toHaveLength(18);
  expect(props?.fit.frontFilm).toBe(source.fit.frontFilm); expect(props?.films?.front?.pixels).toBeNull();
  expect(heard.wall).toHaveBeenCalledWith({ strength: 0 });
  for (const name of ['front', 'side', 'top'] as const) {
    act(() => { threeState.getState().choose(name); }); expect(heard.look).toHaveBeenLastCalledWith(name);
  }
  act(() => { heard.props?.onTurned?.(); }); expect(threeState.getState().chosen).toBeNull();
  expect(document.querySelector('[data-testid="three-stage"]')?.getAttribute('data-state')).toBe('settled');
  page.takeDown();
});
it('catches setup failures and returns to Find with a retryable failure', () => {
  vi.spyOn(console, 'error').mockImplementation(() => undefined);
  currentStep.getState().go('three'); heard.fail = true;
  const page = putOnThePage(<ThreeStage accepted={accepted()} />);
  expect(currentStep.getState().slug).toBe('find'); expect(spineBuild.getState().phase).toBe('failed');
  expect(document.querySelector('canvas')).toBeNull(); page.takeDown();
});
