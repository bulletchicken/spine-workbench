import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { filmPixels } from '@/case-state/film-pixels';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from '@/open-and-save/film-record';
import type { SpineFit } from '@/shapes';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { currentStep } from '@/view-state/current-step';
import { fitSources } from '@/view-state/fit-sources';
import { spineBuild } from '@/view-state/spine-build';
import { buildSpine, cancelSpineBuild, clearSpineBuild, hasCurrentFit } from './build-spine';
import { fitInputFor } from './fit-input';
vi.mock('@/spine-3d/bones/load-bones', () => ({ loadBones: () => Promise.resolve([{ level: 'L3' }]) }));
const story = smallStory();
beforeEach(() => {
  vi.useFakeTimers(); clearSpineBuild(); fitSources.getState().clear();
  caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films: Object.values(story.films).map((f) => filmRecordFrom(f.facts)), marks: structuredClone(story.pass.marks) } }]);
  for (const kind of ['front', 'side'] as const) fitSources.getState().kept(kind, 'hand-built', filmPixels.getState().openings[kind]);
  currentStep.getState().go('find');
});
afterEach(() => { clearSpineBuild(); vi.useRealTimers(); });
it('hands the adapter input unchanged to one job, then caches forward navigation without a case edit', async () => {
  let answer: ((fit: SpineFit) => void) | undefined;
  const fitSpine = vi.fn(() => new Promise<SpineFit>((resolve) => { answer = resolve; }));
  const c = caseStore.getState().case; const version = caseStore.getState().version;
  const pending = buildSpine({ fitSpine });
  await buildSpine({ fitSpine });
  expect(fitSpine).toHaveBeenCalledExactlyOnceWith(fitInputFor(c, fitSources.getState().films, filmPixels.getState().openings));
  expect(currentStep.getState().slug).toBe('find');
  await vi.advanceTimersByTimeAsync(299); expect(spineBuild.getState().busy).toBe(false);
  await vi.advanceTimersByTimeAsync(1); expect(spineBuild.getState().busy).toBe(true);
  answer?.(structuredClone(MADE_UP_CASE.fit)); await pending;
  expect(currentStep.getState().slug).toBe('three'); expect(hasCurrentFit(c)).toBe(true);
  expect(caseStore.getState().case).toBe(c); expect(caseStore.getState().version).toBe(version);
  currentStep.getState().go('find'); await buildSpine({ fitSpine });
  expect(currentStep.getState().slug).toBe('three'); expect(fitSpine).toHaveBeenCalledTimes(1);
  const front = c.films.find((f) => f.kind === 'front'); if (front === undefined) throw new Error('no Front');
  expect(hasCurrentFit({ ...c, films: c.films.map((f) => f === front ? { ...f, geometry: { ...f.geometry, quarterTurns: 1 } } : f) })).toBe(false);
});
it.each(['unconverged', 'rejected', 'non-finite'] as const)('keeps Find and the films on %s', async (kind) => {
  const fit = structuredClone(MADE_UP_CASE.fit);
  if (kind === 'unconverged') fit.converged = false;
  if (kind === 'non-finite') fit.rmsMm = Number.NaN;
  const c = caseStore.getState().case;
  await buildSpine({ fitSpine: () => kind === 'rejected' ? Promise.reject(new Error('fit refused')) : Promise.resolve(fit) });
  expect(currentStep.getState().slug).toBe('find'); expect(spineBuild.getState().phase).toBe('failed');
  expect(caseStore.getState().case).toBe(c); expect(spineBuild.getState().accepted).toBeNull();
});
it('discards a cancelled late answer and invalidates the cache on a new film opening', async () => {
  let answer: ((fit: SpineFit) => void) | undefined;
  const pending = buildSpine({ fitSpine: () => new Promise<SpineFit>((resolve) => { answer = resolve; }) });
  cancelSpineBuild(); answer?.(structuredClone(MADE_UP_CASE.fit)); await pending;
  expect(spineBuild.getState().accepted).toBeNull(); expect(currentStep.getState().slug).toBe('find');
  await buildSpine({ fitSpine: () => Promise.resolve(structuredClone(MADE_UP_CASE.fit)) });
  filmPixels.getState().opened('front', { pixels: new Uint16Array(), thumbnail: new Uint8Array() });
  expect(hasCurrentFit(caseStore.getState().case)).toBe(false);
});

it('never calls the fit with Side alone', async () => {
  caseStore.getState().apply([{ kind: 'remove-film', film: 'front' }]);
  const fitSpine = vi.fn();
  await buildSpine({ fitSpine });
  expect(fitSpine).not.toHaveBeenCalled();
  expect(currentStep.getState().slug).toBe('find');
  expect(spineBuild.getState().phase).toBe('idle');
});
