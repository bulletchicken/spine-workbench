import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { saidNow } from '@/building-blocks/toast/say';
import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { EngineFailure } from '@/engine-room/engine-client';
import { startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { smallStory } from '@/engine-room/stand-in/small-story';
import type { ErrorCode } from '@/shapes';
import { currentStep } from '@/view-state/current-step';
import { findState } from '@/view-state/find-state';
import { filmIntake } from '@/view-state/film-intake';
import { clearFilms } from './start-over';
import { chooseFilms, takeIn, takeInNextSample } from './film-intake';

vi.mock('@/choreography/timing', () => ({ durationOnPage: () => 0 }));
const samples = vi.hoisted(() => ({ files: [] as { bytes(): ArrayBuffer }[] }));
vi.mock('@/sample-films/sample-films', () => ({
  sampleFilmFiles: () => samples.files,
  sampleFilmFilesFor: async (engine: { info(): Promise<{ engineVersion: string }> }) => (await engine.info()).engineVersion === 'stand-in' ? [] : samples.files,
}));
vi.mock('@/open-and-save/pick-files', () => ({ pickFilms: vi.fn() }));
import { pickFilms } from '@/open-and-save/pick-files';

const story = smallStory();
let engine: ReturnType<typeof startStandInEngine>;
const blob = (kind: keyof typeof story.films) => new Blob([story.films[kind].file.slice(0)]);
const refused = (code: ErrorCode) => new EngineFailure({ code, message: 'made-up refusal', detail: null });
beforeEach(() => {
  engine = startStandInEngine({ story, scripts: { films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 } } });
  clearFilms(engine);
  filmIntake.getState().next(0);
  currentStep.getState().go('films');
  findState.getState().clear();
  samples.files = [];
});
afterEach(() => { engine.stop(); vi.restoreAllMocks(); });

describe('taking films in', () => {
  it('leaves Place for Films before a new film is taken in', async () => {
    currentStep.getState().go('place');
    await takeIn(engine,[blob('front')],null);
    expect(currentStep.getState().slug).toBe('films');
  });
  it('assigns a drop on Side, opens in place order, and stores each record beside its pixels', async () => {
    const opened = vi.spyOn(engine, 'openFilm');
    const landed: string[] = [];
    const done = caseStore.subscribe((state, before) => {
      if (state.case.films.length !== before.case.films.length) landed.push(state.case.films.at(-1)?.kind ?? '');
    });
    await takeIn(engine, [blob('side'), blob('front'), blob('bend-left')], 'side');
    done();
    expect(opened.mock.calls.map(([kind]) => kind)).toEqual(['front', 'side', 'bend-left']);
    expect(landed).toEqual(['front', 'side', 'bend-left']);
    expect(caseStore.getState().case.films.map((film) => film.kind)).toEqual(['front', 'side', 'bend-left']);
    expect(caseStore.getState().case.films.every((film) => film.image.path === `films/${film.kind}.png` && film.geometry.imageRight === null)).toBe(true);
    expect(Object.keys(filmPixels.getState().films)).toEqual(['front', 'side', 'bend-left']);
    expect(filmIntake.getState()).toMatchObject({ from: 'yours', reading: [], readDone: 3, readTotal: 3, notice: null });
  });
  it('keeps the previous film and pixels when the engine refuses a dropped made-up PNG outside its story', async () => {
    await takeIn(engine, [blob('front')], null);
    const record = caseStore.getState().case.films[0];
    const pixels = filmPixels.getState().films.front;
    const outsideStory = new Blob([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 0])]);
    await takeIn(engine, [outsideStory], 'front');
    expect(caseStore.getState().case.films[0]).toBe(record);
    expect(filmPixels.getState().films.front).toBe(pixels);
    expect(filmIntake.getState().notice).toEqual({ kind: 'refused', count: 1, code: 'film-unknown-format' });
  });
  it('refuses an invalid case without changing the current films or pixels', async () => {
    await takeIn(engine, [blob('side')], 'side');
    const c = caseStore.getState().case;
    const pixels = filmPixels.getState().films;
    await takeIn(engine, [new Blob([new Uint8Array([80, 75, 3, 4])])], null);
    expect(caseStore.getState().case).toBe(c);
    expect(filmPixels.getState().films).toBe(pixels);
    expect(filmIntake.getState().notice).toEqual({ kind: 'refused', count: 1, code: 'case-not-a-zip' });
  });
  it('opens the first case in a mixed drop and reports every other file as left out', async () => {
    await takeIn(engine, [blob('front')], 'front');
    const saved = await engine.saveCase(caseStore.getState().case);
    const opened = vi.spyOn(engine, 'openCase');
    await takeIn(engine, [new Blob([new Uint8Array(saved)]), blob('side'), new Blob([new Uint8Array(saved)]), new Blob([])], null);
    expect(opened).toHaveBeenCalledTimes(1);
    expect(caseStore.getState().case.films.map((film) => film.kind)).toEqual(['front']);
    expect(filmIntake.getState().notice).toEqual({ kind: 'left-out', count: 3 });
    expect(saidNow()?.line).toBe('Opened a saved case. Three files were left out.');
  });
  it('takes picker results through the same assignment path and ignores cancellation', async () => {
    vi.mocked(pickFilms).mockResolvedValueOnce([{ kind: 'film', blob: blob('side') }]);
    await chooseFilms(engine, 'side');
    expect(caseStore.getState().case.films[0]?.kind).toBe('side');
    expect(filmIntake.getState().notice).toBeNull();
    const state = filmIntake.getState();
    vi.mocked(pickFilms).mockResolvedValueOnce([]);
    await chooseFilms(engine);
    expect(filmIntake.getState()).toBe(state);
  });
  it('clears the made-up sample before a personal film arrives, without mixing sources', async () => {
    await takeInNextSample(engine);
    expect(caseStore.getState().case.films).toHaveLength(4);
    const retired = vi.spyOn(engine, 'retireFilm');
    await takeIn(engine, [blob('side')], 'side');
    expect(retired).toHaveBeenCalledTimes(4);
    expect(caseStore.getState().case.films.map((film) => film.kind)).toEqual(['side']);
    expect(Object.keys(filmPixels.getState().films)).toEqual(['side']);
    expect(filmIntake.getState().from).toBe('yours');
  });
  it('returns from Find before opening, and clears that notice at the next intake', async () => {
    await takeIn(engine, [blob('front')], null);
    currentStep.getState().go('find');
    findState.getState().searching(1);
    const original = engine.openFilm.bind(engine);
    vi.spyOn(engine, 'openFilm').mockImplementationOnce(async (...args) => {
      expect(currentStep.getState().slug).toBe('films');
      return original(...args);
    });
    await takeIn(engine, [blob('side')], 'side');
    expect(filmIntake.getState().notice).toEqual({ kind: 'search-stopped' });
    await takeIn(engine, [blob('bend-left')], 'bend-left');
    expect(filmIntake.getState().notice).toBeNull();
  });
  it('does not overlap intakes, even when the first open is held', async () => {
    const original = engine.openFilm.bind(engine);
    let release: () => void = () => undefined;
    const held = new Promise<void>((resolve) => { release = resolve; });
    const opens: string[] = [];
    vi.spyOn(engine, 'openFilm').mockImplementation(async (...args) => {
      opens.push(args[0]);
      if (opens.length === 1) await held;
      return original(...args);
    });
    const first = takeIn(engine, [blob('front')], 'front');
    const second = takeIn(engine, [blob('side')], 'side');
    await vi.waitFor(() => { expect(opens).toEqual(['front']); });
    release();
    await Promise.all([first, second]);
    expect(opens).toEqual(['front', 'side']);
    expect(caseStore.getState().case.films).toHaveLength(2);
  });
  it.each(['film-unsupported-compression', 'film-truncated', 'engine-out-of-memory', 'engine-crashed'] as const)('keeps the old film for %s, then continues reading', async (code) => {
    await takeIn(engine, [blob('front')], 'front');
    const old = caseStore.getState().case.films[0];
    vi.spyOn(engine, 'openFilm').mockRejectedValueOnce(refused(code));
    await takeIn(engine, [blob('front'), blob('side')], 'front');
    expect(caseStore.getState().case.films[0]).toBe(old);
    expect(caseStore.getState().case.films).toHaveLength(2);
    expect(filmIntake.getState().notice).toEqual({ kind: 'refused', count: 1, code });
    expect(filmIntake.getState().reading).toEqual([]);
  });
  it('retires the slot and removes its old record if display pixels fail after opening', async () => {
    await takeIn(engine, [blob('front')], 'front');
    const retire = vi.spyOn(engine, 'retireFilm');
    vi.spyOn(engine, 'displayPixels').mockRejectedValueOnce(refused('engine-crashed'));
    await takeIn(engine, [blob('front')], 'front');
    expect(retire).toHaveBeenCalledWith('front');
    expect(caseStore.getState().case.films).toEqual([]);
    expect(filmPixels.getState().films.front).toBeUndefined();
    expect(filmIntake.getState()).toMatchObject({ from: null, notice: { kind: 'refused', count: 1, code: 'engine-crashed' } });
  });
  it('lets pass-cancelled go by and resets an empty source', async () => {
    vi.spyOn(engine, 'openFilm').mockRejectedValueOnce(refused('pass-cancelled'));
    await takeIn(engine, [blob('front')], 'front');
    expect(filmIntake.getState()).toMatchObject({ from: null, reading: [], notice: null });
  });
  it('counts films without a place, and lets a refusal take precedence', async () => {
    await takeIn(engine, [blob('front'), blob('side'), blob('bend-left'), blob('bend-right'), blob('front')], null);
    expect(filmIntake.getState().notice).toEqual({ kind: 'left-out', count: 1 });
    await takeIn(engine, [blob('front'), new Blob([])], null);
    expect(filmIntake.getState().notice).toEqual({ kind: 'refused', count: 1, code: 'film-truncated' });
  });
  it('always makes the case on the stand-in, even with local samples, and goes round samples on a capable engine', async () => {
    samples.files = [{ bytes: () => story.films.front.file.slice(0) }, { bytes: () => story.films.front.file.slice(0) }];
    await takeInNextSample(engine);
    expect(filmIntake.getState()).toMatchObject({ from: 'made-up-case', nextSample: 0 });
    const info = await engine.info();
    vi.spyOn(engine, 'info').mockResolvedValue({ ...info, engineVersion: '0.1.0' });
    await takeInNextSample(engine);
    expect(filmIntake.getState()).toMatchObject({ from: 'sample-film', sample: 0, nextSample: 1 });
    expect(caseStore.getState().case.films).toHaveLength(1);
    await takeInNextSample(engine);
    expect(filmIntake.getState()).toMatchObject({ from: 'sample-film', sample: 1, nextSample: 2 });
    await takeInNextSample(engine);
    expect(filmIntake.getState()).toMatchObject({ from: 'made-up-case', sample: null, nextSample: 0 });
    expect(caseStore.getState().case.films).toHaveLength(4);
    await takeInNextSample(engine);
    expect(filmIntake.getState()).toMatchObject({ from: 'sample-film', sample: 0, nextSample: 1 });
  });
  it('finishes the drawing state with a notice if making the sample fails', async () => {
    vi.spyOn(engine, 'makeUpSpine').mockRejectedValueOnce(refused('made-up-unsatisfiable'));
    await takeInNextSample(engine);
    expect(filmIntake.getState()).toMatchObject({ making: false, reading: [], from: null, notice: { kind: 'refused', code: 'made-up-unsatisfiable', count: 1 } });
  });
});
