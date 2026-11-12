import { act, useLayoutEffect } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { revealPlan } from '@/auto-pass/find-replay';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { currentStep } from '@/view-state/current-step';
import { findState } from '@/view-state/find-state';
import { preferences } from '@/view-state/preferences';
import { selection } from '@/view-state/selection';
import { useFindOnStage } from './find-on-stage';

vi.mock('@/choreography/timing', () => ({ durationOnPage: () => 160, numberOnPage: (name: string) => ({ '--zoom-find': 1.3, '--zoom-find-small': 1.15, '--zoom-review': 2 })[name] }));
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
let root: Root;
let latest: ReturnType<typeof useFindOnStage>;
let renders: number;
function Probe() { const finding = useFindOnStage(); useLayoutEffect(() => { latest = finding; renders += 1; }); return null; }

beforeEach(() => {
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }));
  preferences.getState().setMotion('system', false);
  findReplay.getState().clear(); findState.getState().clear(); selection.getState().hover(null);
  currentStep.getState().go('find');
  const story = smallStory();
  caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films: Object.values(story.films).map((film) => filmRecordFrom(film.facts)), marks: structuredClone(story.pass.marks) } }]);
  root = createRoot(document.createElement('div')); renders = 0;
  act(() => { root.render(<Probe />); });
});
afterEach(() => { act(() => { root.unmount(); }); vi.unstubAllGlobals(); vi.useRealTimers(); });

describe('Find on the stage', () => {
  it('shows no outlines while idle, searching or failed, and keeps the whole film', () => {
    for (const phase of ['idle', 'searching', 'failed'] as const) {
      act(() => { findState.setState({ phase }); });
      expect(Object.values(latest.films).every((film) => film.outlines.every((m) => !m.shown))).toBe(true);
      expect(latest.films.front?.frame).toEqual({ kind: 'whole' });
      expect(latest.replay).toBeNull();
      expect(latest.onPoint).toBeUndefined();
    }
  });
  it('reveals with the replay clock, without rendering for light frames between arrivals', () => {
    const plan = revealPlan(caseStore.getState().case.marks, {}, { height: 200, edge: 60, filmHeight: 1300 });
    if (plan === null) throw new Error('no plan');
    act(() => { findState.getState().replaying(); findReplay.getState().start(plan, { lead: 700, travel: 4400, hold: 560 }); });
    expect(latest.films.front?.outlines.some((m) => m.shown)).toBe(false);
    expect(latest.films.front?.frame).toMatchObject({ kind: 'spine', names: true, most: 1.3 });
    expect(latest.films['bend-left']?.frame).toMatchObject({ kind: 'spine', names: false, most: 1.15 });
    expect(latest.replay).toBe(findReplay.signal);
    expect(latest.films.front?.light).toEqual(plan.light.front);
    const before = renders;
    act(() => { findReplay.getState().advance(100); });
    expect(renders).toBe(before);
    act(() => { findReplay.getState().advance(2800); });
    expect(latest.films.front?.outlines.filter((m) => m.shown)).toHaveLength(2);
    expect(latest.films.side?.outlines.filter((m) => m.shown)).toHaveLength(2);
    act(() => { currentStep.getState().go('films'); });
    expect(latest.films.front?.light).toBeNull();
    expect(latest.replay).toBeNull();
    expect(latest.films.front?.frame.kind).toBe('whole');
    act(() => { currentStep.getState().go('find'); });
    act(() => { findReplay.getState().advance(10000); });
    expect(latest.marked).toBe(true);
    act(() => { findState.getState().found(); });
    expect(Object.values(latest.films).every((film) => film.outlines.every((m) => m.shown))).toBe(true);
    expect(latest.replay).toBeNull();
  });
  it('lights the same name across films, and review focuses it without a hover', () => {
    act(() => { findState.getState().found(); });
    const first = latest.films.front?.outlines[0]; if (first === undefined) throw new Error('no outline');
    act(() => { latest.onPoint?.('front', first.key); });
    for (const film of Object.values(latest.films)) expect(film.outlines.find((m) => m.key === film.on)?.name).toBe(first.name);
    act(() => { selection.getState().hover(null); findState.getState().openReview(['L5']); });
    expect(latest.reviewing).toBe(true);
    expect(latest.onPoint).toBeUndefined();
    for (const film of Object.values(latest.films)) { expect(film.on).toBeNull(); expect(film.outlines.find((m) => m.key === film.only)?.name).toBe('L5'); }
    expect(latest.films.front?.frame).toMatchObject({ kind: 'vertebra', over: 2, names: true });
    expect(latest.films.side?.frame).toMatchObject({ kind: 'vertebra', over: 2, names: true });
    expect(latest.films['bend-left']?.frame.kind).toBe('spine');
    const saved = JSON.stringify(caseStore.getState().case);
    act(() => { findState.getState().lookedAt('L5'); });
    for (const film of Object.values(latest.films)) expect(film.outlines.find((m) => m.name === 'L5')?.look).toBe('found');
    expect(JSON.stringify(caseStore.getState().case)).toBe(saved);
    act(() => { findState.getState().look(null); });
    expect(latest.films.front?.frame.kind).toBe('spine');
    act(() => { currentStep.getState().go('films'); });
    expect(latest.films.front?.frame.kind).toBe('whole');
  });
  it('fades before landing the camera under reduced motion, with the same reveals and no light', async () => {
    vi.useFakeTimers();
    const frame = document.createElement('div'); frame.className = 'frame';
    const stage = document.createElement('main'); stage.className = 'stage';
    const tiles = document.createElement('div'); tiles.className = 'tiles';
    frame.append(stage); stage.append(tiles); document.body.append(frame);
    const plan = revealPlan(caseStore.getState().case.marks, {}, { height: 200, edge: 60, filmHeight: 1300 }); if (plan === null) throw new Error('no plan');
    act(() => { preferences.getState().setMotion('reduced', false); findState.getState().replaying(); findReplay.getState().start(plan, { lead: 700, travel: 4400, hold: 560 }); findReplay.getState().advance(2900); });
    expect(tiles.classList.contains('fading')).toBe(true);
    expect(latest.films.front?.frame.kind).toBe('whole');
    await act(async () => { await vi.advanceTimersByTimeAsync(160); });
    expect(tiles.classList.contains('fading')).toBe(false);
    expect(latest.films.front?.frame.kind).toBe('spine');
    expect(latest.replay).toBeNull();
    expect(latest.films.front?.light).toBeNull();
    expect(latest.films.front?.outlines.filter((m) => m.shown)).toHaveLength(2);
    frame.remove();
  });
});
