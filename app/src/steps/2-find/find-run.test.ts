import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { findReplay } from '@/auto-pass/find-replay-clock';
import { shownAt } from '@/auto-pass/find-replay';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { filmPixels } from '@/case-state/film-pixels';
import { EngineFailure, type RunOptions } from '@/engine-room/engine-client';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { DEFAULT_SCRIPTS, startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { filmRecordFrom } from '@/open-and-save/film-record';
import type { AutoPassResult, FilmKind, WatchFrame } from '@/shapes';
import { currentStep } from '@/view-state/current-step';
import { filmIntake } from '@/view-state/film-intake';
import { findState } from '@/view-state/find-state';
import { fitSources } from '@/view-state/fit-sources';
import { preferences } from '@/view-state/preferences';
import { findRun } from './find-run';

vi.mock('@/choreography/timing', () => ({
  durationOnPage: (name: string) => ({ '--dur-scene': 640, '--stagger': 40, '--dur-fade': 160, '--dur-quick': 140, '--dur-light-lead': 700, '--dur-light': 4400, '--dur-hold': 560 })[name],
  numberOnPage: (name: string) => ({ '--light-h': 200, '--light-edge': 60, '--film-h': 1300 })[name],
}));
const story = smallStory();
const engine = startStandInEngine({ story, scripts: { films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 }, find: { ...DEFAULT_SCRIPTS.find, normalMs: 40, placePointsMs: 0 } } });
function put(kinds: readonly FilmKind[] = ['front', 'side', 'bend-left', 'bend-right']) {
  caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films: kinds.map((kind) => filmRecordFrom(story.films[kind].facts)) } }]);
}
function result(): AutoPassResult {
  const marks = structuredClone(story.pass.marks);
  for (const film of marks) for (const v of film.vertebrae) v.id += caseStore.getState().case.nextVertebraId;
  return { ...story.pass, caseVersion: caseStore.getState().version, marks };
}
function answer(answer: AutoPassResult = result()) {
  return vi.spyOn(engine, 'runAutoPass').mockResolvedValue(answer);
}
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
  vi.stubGlobal('window', { matchMedia: () => ({ matches: false }) });
  preferences.getState().setMotion('system', false);
  currentStep.getState().go('find');
  findRun.stop(); put(); filmIntake.getState().source('yours'); fitSources.getState().clear();
});
afterEach(() => { findRun.stop(); vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers(); });

describe('running Find', () => {
  it.each(['yours', 'sample-film', 'made-up-case'] as const)('routes %s and asks for no watch frames', async (from) => {
    filmIntake.getState().source(from);
    const expected = result(); const called = answer(expected); findRun.start(engine);
    await vi.advanceTimersByTimeAsync(760);
    expect(called.mock.calls[0]?.[0].frontFinder).toBe(from === 'made-up-case' ? 'hand-built' : 'trained');
    expect(called.mock.calls[0]?.[0].sideFinder).toBe(from === 'made-up-case' ? 'hand-built' : 'trained');
    expect(called.mock.calls[0]?.[1]).not.toHaveProperty('onFrame');
    expect(called.mock.calls[0]?.[0].options).toEqual({ acquisition: 'unknown', population: 'adolescent', template: null });
    expect(findState.getState().phase).toBe('replaying');
    expect(caseStore.getState().case.marks).toEqual(expected.marks);
    expect(caseStore.getState().history.undo).toHaveLength(0);
    for (const marks of expected.marks) expect(fitSources.getState().films[marks.film]).toEqual({ finder: (marks.film === 'front' || marks.film === 'side') && from !== 'made-up-case' ? 'trained' : 'hand-built', opening: filmPixels.getState().openings[marks.film] });
  });
  it('lands one edit per film as one action, before the replay, then clears the history', async () => {
    const expected = result(); answer(expected); const apply = vi.spyOn(caseStore.getState(), 'apply'); const fresh = vi.spyOn(caseStore.getState(), 'startAfresh');
    findRun.start(engine); await vi.advanceTimersByTimeAsync(759);
    expect(apply.mock.calls[0]?.[0]).toEqual(expected.marks.map((marks) => ({ kind: 'replace-marks', marks, basedOn: expected.caseVersion, cause: { by: 'auto-pass' } })));
    expect(fresh).toHaveBeenCalledOnce();
    expect(findReplay.getState().plan).toBeNull();
    expect(findState.getState().phase).toBe('searching');
    await vi.advanceTimersByTimeAsync(1);
    expect(findReplay.getState().playing).toBe(true);
    findReplay.getState().advance(100000);
    expect(findState.getState().phase).toBe('found');
  });
  it('waits for the reduced-motion flight and searches Front alone without removing its partners', async () => {
    preferences.getState().setMotion('reduced', false);
    const value = result(); value.marks = value.marks.filter((film) => film.film === 'front');
    const called = answer(value);
    findRun.start(engine, ['front']); await vi.advanceTimersByTimeAsync(319);
    expect(findReplay.getState().plan).toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    expect(findReplay.getState().playing).toBe(true);
    expect(called.mock.calls[0]?.[0].films.map((film) => film.kind)).toEqual(['front']);
    expect(caseStore.getState().case.films).toHaveLength(4);
    expect(findState.getState().searched).toBe(1);
  });
  it('lets a late answer go after leaving Find, and clears a paused replay after its fade', async () => {
    let finish: (value: AutoPassResult) => void = () => undefined;
    const called = vi.spyOn(engine, 'runAutoPass').mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    const late = result(); findRun.start(engine);
    currentStep.getState().go('films');
    expect(called.mock.calls[0]?.[1]?.signal?.aborted).toBe(true);
    finish(late); await vi.advanceTimersByTimeAsync(140);
    expect(caseStore.getState().case.marks).toEqual([]);
    expect(fitSources.getState().films).toEqual({});
    expect(findState.getState().phase).toBe('idle');
    currentStep.getState().go('find'); called.mockResolvedValue(result()); findRun.start(engine);
    await vi.advanceTimersByTimeAsync(760); findReplay.getState().advance(3000);
    const was = findReplay.getState(); currentStep.getState().go('films');
    expect(findReplay.getState()).toMatchObject({ playing: false, time: was.time, revealed: was.revealed });
    await vi.advanceTimersByTimeAsync(139); expect(findReplay.getState().plan).not.toBeNull();
    await vi.advanceTimersByTimeAsync(1); expect(findReplay.getState().plan).toBeNull();
  });
  it('aborts a replaced run and ignores its later progress and answer', async () => {
    let finish: (value: AutoPassResult) => void = () => undefined;
    const old = result();
    const newer = result(); const first = newer.marks[0]?.vertebrae[0]; if (first === undefined) throw new Error('no outline'); first.confidence.score = 0.65;
    const called = vi.spyOn(engine, 'runAutoPass').mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; })).mockResolvedValue(newer);
    findRun.start(engine); const oldRun = called.mock.calls[0]?.[1]; findRun.start(engine);
    expect(oldRun?.signal?.aborted).toBe(true);
    oldRun?.onProgress?.({ stage: 'confidence', fraction: 1 });
    expect(findState.getState().share).toBe(0);
    await vi.advanceTimersByTimeAsync(760); finish(old); await vi.advanceTimersByTimeAsync(0);
    expect(caseStore.getState().case.marks[0]?.vertebrae[0]?.confidence.score).toBe(0.65);
    findRun.stop();
    expect(findState.getState().phase).toBe('idle');
    expect(findReplay.getState().plan).toBeNull();
  });
  it.each(['pass-no-spine-found', 'pass-flat-film', 'engine-crashed', 'engine-out-of-memory', 'engine-out-of-sync'] as const)('ends in failed for %s', async (code) => {
    vi.spyOn(engine, 'runAutoPass').mockRejectedValue(new EngineFailure({ code, message: '', detail: null }));
    findRun.start(engine); await vi.advanceTimersByTimeAsync(0);
    expect(findState.getState()).toMatchObject({ phase: 'failed', failure: code });
    expect(findReplay.getState().plan).toBeNull();
  });
  it('lets cancellation go, and discards an answer for a Front film changed during the search', async () => {
    const called = vi.spyOn(engine, 'runAutoPass').mockRejectedValue(new EngineFailure({ code: 'pass-cancelled', message: '', detail: null }));
    findRun.start(engine); await vi.advanceTimersByTimeAsync(0);
    expect(findState.getState().failure).toBeNull();
    const late = result(); called.mockImplementation(() => {
      caseStore.getState().apply([{ kind: 'set-film', film: filmRecordFrom(story.films.front.facts) }]);
      return Promise.resolve(late);
    });
    findRun.start(engine); await vi.advanceTimersByTimeAsync(1000);
    expect(findReplay.getState().plan).toBeNull();
    expect(caseStore.getState().case.marks.some((film) => film.film === 'front')).toBe(false);
    expect(fitSources.getState().films.front).toBeUndefined();
  });
  it('records the finder actually requested even when intake changes before its reply', async () => {
    let finish: (value: AutoPassResult) => void = () => undefined;
    const expected = result();
    const called = vi.spyOn(engine, 'runAutoPass').mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    filmIntake.getState().source('made-up-case'); findRun.start(engine);
    filmIntake.getState().source('yours'); finish(expected);
    await vi.advanceTimersByTimeAsync(760);
    expect(called.mock.calls[0]?.[0].frontFinder).toBe('hand-built');
    expect(fitSources.getState().films.front?.finder).toBe('hand-built');
  });
  it('treats an answer containing only C7 on Front as no spine found', async () => {
    const value = result();
    const first = value.marks[0]?.vertebrae[0]; if (first === undefined || value.marks[0] === undefined) throw new Error('no marks');
    value.marks[0].vertebrae = [{ ...first, level: 'C7' }]; answer(value);
    findRun.start(engine); await vi.advanceTimersByTimeAsync(0);
    expect(findState.getState()).toMatchObject({ phase: 'failed', failure: 'pass-no-spine-found' });
  });
  it('replays a sparse trained sequence in the contract’s frame shapes without waiting for dense frames', async () => {
    put(['front']);
    const front = result().marks[0]; if (front === undefined) throw new Error('no marks');
    front.naming = { ...front.naming, fromTop: null, fromBottom: null, alternatives: [], flags: ['estimated'] };
    for (const v of front.vertebrae) if (v.marks.kind === 'front') { v.marks.points.pedL = { seen: 'not-visible', source: 'auto' }; v.marks.points.pedR = { seen: 'not-visible', source: 'auto' }; }
    const frames: WatchFrame[] = [
      { kind: 'stage-started', film: 'front', stage: 'evidence' },
      { kind: 'stage-started', film: 'front', stage: 'chain' },
      { kind: 'chain-locked', film: 'front', stage: 'chain', bodies: [...front.vertebrae].reverse().map((v) => {
        if (v.marks.kind !== 'front') throw new Error('not Front');
        const p = v.marks.points; return { level: v.level, sup: { a: p.supL.at, b: p.supR.at }, inf: { a: p.infL.at, b: p.infR.at }, inferred: false };
      }) },
      { kind: 'stage-started', film: 'front', stage: 'naming' },
      { kind: 'levels-named', film: 'front', stage: 'naming', naming: front.naming, levels: front.vertebrae.map((v) => v.level) },
      { kind: 'stage-started', film: 'front', stage: 'confidence' },
      { kind: 'confidence', film: 'front', stage: 'confidence', levels: front.vertebrae.map((v) => ({ level: v.level, score: v.confidence.score, band: v.confidence.band })) },
      { kind: 'outlines', film: 'front', stage: 'confidence', final: true, quads: front.vertebrae.map((v) => {
        if (v.marks.kind !== 'front') throw new Error('not Front');
        const p = v.marks.points; return { level: v.level, corners: [p.supL.at, p.supR.at, p.infR.at, p.infL.at] };
      }) },
      { kind: 'warning', film: 'front', stage: 'confidence', warning: { code: 'low-evidence', film: 'front', level: null, region: null } },
    ];
    const called = vi.spyOn(engine, 'runAutoPass').mockImplementation((input, run?: RunOptions) => {
      for (const frame of frames) run?.onFrame?.(frame);
      run?.onProgress?.({ stage: 'evidence', fraction: 1 });
      return Promise.resolve({ caseVersion: input.caseVersion, marks: [front], twoFilms: null, warnings: [{ code: 'low-evidence', film: 'front', level: null, region: null }] });
    });
    findRun.start(engine); await vi.advanceTimersByTimeAsync(0);
    expect(findState.getState().share).toBe(0.9);
    await vi.advanceTimersByTimeAsync(760);
    expect(called.mock.calls[0]?.[1]).not.toHaveProperty('onFrame');
    const clock = findReplay.getState(); if (clock.plan === null) throw new Error('no replay');
    expect(shownAt(clock.plan, clock.times, 0).count).toBe(0);
    expect(shownAt(clock.plan, clock.times, 2900).count).toBe(2);
    clock.advance(100000); expect(findState.getState().phase).toBe('found');
  });
  it('runs the dense hand-built sequence on the stand-in', async () => {
    filmIntake.getState().source('made-up-case');
    for (const film of caseStore.getState().case.films) {
      const opening = engine.openFilm(film.kind, story.films[film.kind].file, null);
      await vi.runAllTimersAsync(); await opening;
    }
    const called = vi.spyOn(engine, 'runAutoPass'); findRun.start(engine);
    await vi.advanceTimersByTimeAsync(1000);
    expect(findState.getState().phase).toBe('replaying');
    expect(called.mock.calls[0]?.[0].frontFinder).toBe('hand-built');
    findReplay.getState().advance(100000);
    expect(findState.getState().phase).toBe('found');
    engine.stop();
  });
});
