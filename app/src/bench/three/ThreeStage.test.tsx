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
  useImperativeHandle(props.ref, () => ({ stage: () => ({ size: () => ({ width: 1000, height: 700 }), bounds: () => ({ low: [0, 0, 0], high: [100, 500, 100] }), setView: () => undefined, film: () => ({ set: heard.wall }) }), spine: () => null, lookFrom: heard.look, settle: () => undefined }) as never);
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
    act(() => { threeState.getState().choose(name); }); expect(heard.look).toHaveBeenLastCalledWith(expect.objectContaining({ perspective: 0 }));
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

it('names a pointed bone, shares its level and keeps the chip following movement on that bone', () => {
  const page = putOnThePage(<ThreeStage accepted={accepted()} />);
  act(() => { heard.props?.onPoint?.('T8'); });
  expect(heard.props?.highlighted).toEqual(['T8']);
  expect(document.documentElement.hasAttribute('data-links')).toBe(true);
  const canvas = page.host.querySelector('canvas'); if (canvas === null) throw new Error('no canvas');
  act(() => { canvas.dispatchEvent(new MouseEvent('pointermove', { bubbles: true, clientX: 100, clientY: 200 })); });
  const chip = document.body.querySelector<HTMLElement>('.chip');
  expect(chip?.textContent).toBe('T8'); expect(chip?.style.left).toBe('100px');
  act(() => { canvas.dispatchEvent(new MouseEvent('pointermove', { bubbles: true, clientX: 150, clientY: 210 })); });
  expect(chip?.style.left).toBe('150px');
  act(() => { heard.props?.onPoint?.(null); }); expect(chip?.classList.contains('show')).toBe(false);
  page.takeDown();
});
