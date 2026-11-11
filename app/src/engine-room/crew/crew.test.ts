import type { CaseFileV1, FilmMarks, ToolPreviewInput } from '@/shapes';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { EngineFailure } from '../engine-client';
import { ENGINE_INFO, PANIC_MESSAGE, fakeCrew, filmFile, outcome, pixelsOf, settle } from './test-rig';

const preview = (caseVersion: number): ToolPreviewInput => ({
  caseVersion,
  tool: 'boundary-wand',
  at: [1, 1],
  marks: {} as FilmMarks,
  newVertebraId: 0,
});
const caseNumbered = (nextVertebraId: number) => ({ nextVertebraId }) as CaseFileV1;
const wand = (caseVersion: number) => ({ caseVersion, marks: {} as FilmMarks, vertebra: 1, clicks: [] });

afterEach(() => {
  vi.useRealTimers();
});

describe('start-up', () => {
  it('boots the spine worker and one spare film worker, and reports the engine', async () => {
    const rig = fakeCrew();
    expect(rig.workers.map((worker) => worker.role)).toEqual(['spine', 'film']);
    expect(await rig.engine.info()).toEqual(ENGINE_INFO);
  });

  it('gives the first film the spare, and each later film its own worker', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    expect(rig.workers).toHaveLength(2);
    expect(rig.workers[1]?.jobsRun()).toEqual(['open-film']);
    await rig.engine.openFilm('side', filmFile('side A'), null);
    expect(rig.workers.map((worker) => worker.role)).toEqual(['spine', 'film', 'film']);
    expect(await rig.engine.displayPixels('front')).toEqual(pixelsOf('front A'));
    expect(await rig.engine.displayPixels('side')).toEqual(pixelsOf('side A'));
  });

  it('rejects the engine’s info with pass-cancelled when stopped before it is ready, and keeps it once it is', async () => {
    const early = fakeCrew();
    early.engine.stop();
    expect(await outcome(early.engine.info())).toBe('pass-cancelled');
    const late = fakeCrew();
    expect(await late.engine.info()).toEqual(ENGINE_INFO);
    late.engine.stop();
    expect(await late.engine.info()).toEqual(ENGINE_INFO);
  });

  it('refuses every call when the engine cannot be loaded', async () => {
    const failure = new EngineFailure({ code: 'engine-load-failed', message: 'no engine', detail: 'HTTP 404' });
    const rig = fakeCrew({ engine: () => Promise.reject(failure) });
    expect(await outcome(rig.engine.info())).toBe('engine-load-failed');
    expect(await outcome(rig.engine.openFilm('front', filmFile('front A'), null))).toBe('engine-load-failed');
    expect(await outcome(rig.engine.checkCase(caseNumbered(1)))).toBe('engine-load-failed');
  });

  it('refuses every call, and starts no more workers, when the engine is stale', async () => {
    const rig = fakeCrew({ info: { ...ENGINE_INFO, shapesFingerprint: 'stale' } });
    expect(await outcome(rig.engine.info())).toBe('engine-out-of-sync');
    expect(await outcome(rig.engine.openFilm('front', filmFile('front A'), null))).toBe('engine-out-of-sync');
    await settle();
    expect(rig.workers).toHaveLength(2);
  });
});

describe('queues', () => {
  it('sends a worker one job at a time, in the order asked', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const worker = rig.filmWorkers()[0];
    const pixels = rig.engine.displayPixels('front');
    const guess = rig.engine.senseGeometry('front');
    expect(worker?.jobsRun()).toEqual(['open-film', 'display-pixels']);
    await pixels;
    expect(worker?.jobsRun()).toEqual(['open-film', 'display-pixels', 'sense-geometry']);
    expect(await guess).toEqual({ name: 'front A' });
  });

  it('keeps one replaceable job waiting: a newer one replaces it, and it rejects', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const first = rig.engine.previewTool('front', preview(1));
    const second = rig.engine.previewTool('front', preview(2));
    const third = rig.engine.previewTool('front', preview(3));
    expect(await outcome(second)).toBe('pass-cancelled');
    expect(await first).toMatchObject({ caseVersion: 1 });
    expect(await third).toMatchObject({ caseVersion: 3 });
    expect(rig.filmWorkers()[0]?.jobsRun()).toEqual(['open-film', 'tool-preview', 'tool-preview']);
  });

  it('never replaces a replaceable job that is already running, nor a job of another kind', async () => {
    const rig = fakeCrew();
    await rig.engine.info();
    const running = rig.engine.checkCase(caseNumbered(1));
    const waiting = rig.engine.checkCase(caseNumbered(2));
    const unpack = rig.engine.openCase(filmFile('zip'));
    const newest = rig.engine.checkCase(caseNumbered(3));
    expect(await outcome(waiting)).toBe('pass-cancelled');
    expect((await running)[0]?.vertebraId).toBe(1);
    expect(await outcome(unpack)).toBe('answered');
    expect((await newest)[0]?.vertebraId).toBe(3);
  });
});

describe('replacing a film', () => {
  it('rejects the calls waiting and running on the old film; later calls reach the new one', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const running = rig.engine.displayPixels('front');
    const waiting = rig.engine.senseGeometry('front');
    const replaced = rig.engine.openFilm('front', filmFile('front B'), null);
    const after = rig.engine.senseGeometry('front');
    expect(await outcome(running)).toBe('pass-cancelled');
    expect(await outcome(waiting)).toBe('pass-cancelled');
    expect((await replaced).facts.width).toBe('front B'.length);
    expect(await after).toEqual({ name: 'front B' });
    expect(rig.filmWorkers()).toHaveLength(1);
  });

  it('leaves the open film as it was when the new one cannot be read', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const cutOff = rig.engine.senseGeometry('front');
    const refused = rig.engine.openFilm('front', filmFile('bad bytes'), null);
    const after = rig.engine.senseGeometry('front');
    expect(await outcome(cutOff)).toBe('pass-cancelled');
    expect(await outcome(refused)).toBe('film-unknown-format');
    expect(await after).toEqual({ name: 'front A' });
  });

  it('replaces the worker when the film it is still opening is replaced', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const opening = rig.engine.openFilm('front', filmFile('front B'), null);
    await settle(1);
    const first = rig.filmWorkers()[0];
    const replaced = rig.engine.openFilm('front', filmFile('front C'), null);
    expect(first?.terminated).toBe(true);
    expect(await outcome(opening)).toBe('pass-cancelled');
    expect((await replaced).facts.width).toBe('front C'.length);
    // The fresh worker read the open film again before the new one.
    expect(rig.filmWorkers()[0]?.jobsRun()).toEqual(['open-film', 'open-film']);
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front C' });
  });

  it('leaves the film before as the open one when a film still opening is replaced by one that cannot be read', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const opening = rig.engine.openFilm('front', filmFile('front B'), null);
    await settle(1);
    const refused = rig.engine.openFilm('front', filmFile('bad bytes'), null);
    expect(await outcome(opening)).toBe('pass-cancelled');
    expect(await outcome(refused)).toBe('film-unknown-format');
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front A' });
  });

  it('leaves calls on other films alone', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    await rig.engine.openFilm('side', filmFile('side A'), null);
    const side = rig.engine.senseGeometry('side');
    const spine = rig.engine.checkCase(caseNumbered(4));
    void rig.engine.openFilm('front', filmFile('front B'), null);
    expect(await side).toEqual({ name: 'side A' });
    expect(await outcome(spine)).toBe('answered');
  });

  it('rejects a save that includes the replaced film as a whole', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    await rig.engine.openFilm('side', filmFile('side A'), null);
    const record = (kind: 'front' | 'side') => ({ kind }) as CaseFileV1['films'][number];
    const twoFilms = { films: [record('front'), record('side')] } as CaseFileV1;
    expect(new TextDecoder().decode(await rig.engine.saveCase(twoFilms))).toBe('front=front A;side=side A');
    const saving = rig.engine.saveCase(twoFilms);
    void rig.engine.openFilm('side', filmFile('side B'), null);
    expect(await outcome(saving)).toBe('pass-cancelled');
    expect(new TextDecoder().decode(await rig.engine.saveCase(twoFilms))).toBe('front=front A;side=side B');
  });
});

describe('retiring a film', () => {
  it('rejects its calls, closes its worker, and refuses later calls on it', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const worker = rig.filmWorkers()[0];
    const running = rig.engine.displayPixels('front');
    const waiting = rig.engine.senseGeometry('front');
    rig.engine.retireFilm('front');
    expect(worker?.terminated).toBe(true);
    expect(await outcome(running)).toBe('pass-cancelled');
    expect(await outcome(waiting)).toBe('pass-cancelled');
    expect(await outcome(rig.engine.senseGeometry('front'))).toBe('engine-out-of-sync');
    await rig.engine.openFilm('front', filmFile('front B'), null);
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front B' });
  });
});

describe('crashes', () => {
  it('fails the running job, starts a fresh worker and reads the film again before the next job', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const crashing = rig.engine.regionWand('front', wand(-1));
    const waiting = rig.engine.senseGeometry('front');
    await expect(crashing).rejects.toMatchObject({ error: { code: 'engine-crashed', detail: PANIC_MESSAGE } });
    expect(await waiting).toEqual({ name: 'front A' });
    const [fresh] = rig.filmWorkers();
    expect(rig.workers.filter((worker) => worker.role === 'film')).toHaveLength(2);
    expect(fresh?.received[0]?.kind).toBe('boot');
    expect(fresh?.jobsRun()).toEqual(['open-film', 'sense-geometry']);
  });

  it('reads the kept bytes of the film that was open, never of a film that failed to open', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    expect(await outcome(rig.engine.openFilm('front', filmFile('bad bytes'), null))).toBe('film-unknown-format');
    expect(await outcome(rig.engine.regionWand('front', wand(-1)))).toBe('engine-crashed');
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front A' });
  });

  it('treats a crash during a call already cut off by a replacement as no news', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const crashing = rig.engine.regionWand('front', wand(-1));
    const replacing = rig.engine.openFilm('front', filmFile('front B'), null);
    expect(await outcome(crashing)).toBe('pass-cancelled');
    expect((await replacing).facts.width).toBe('front B'.length);
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front B' });
    // The fresh worker read the open film (A) again, then opened B.
    expect(rig.filmWorkers()[0]?.jobsRun()).toEqual(['open-film', 'open-film', 'sense-geometry']);
  });

  it('does not read a film again a second time when reading it again crashes', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('panic again 1'), null);
    expect(await outcome(rig.engine.regionWand('front', wand(-1)))).toBe('engine-crashed');
    expect(await outcome(rig.engine.senseGeometry('front'))).toBe('engine-out-of-sync');
    expect(rig.workers.filter((worker) => worker.role === 'film')).toHaveLength(3);
    await rig.engine.openFilm('front', filmFile('front B'), null);
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front B' });
  });

  it('starts a fresh worker when one fails outright', async () => {
    const rig = fakeCrew();
    await rig.engine.openFilm('front', filmFile('front A'), null);
    const worker = rig.filmWorkers()[0];
    if (worker === undefined) throw new Error('no film worker');
    worker.hold();
    const running = rig.engine.senseGeometry('front');
    await settle(2);
    worker.fail('Uncaught RangeError');
    await expect(running).rejects.toMatchObject({ error: { code: 'engine-crashed', detail: 'Uncaught RangeError' } });
    expect(await rig.engine.displayPixels('front')).toEqual(pixelsOf('front A'));
  });

  it('replaces a worker whose pass says nothing for two seconds', async () => {
    vi.useFakeTimers();
    const rig = fakeCrew();
    const opening = rig.engine.openFilm('front', filmFile('front slow'), null);
    await vi.advanceTimersByTimeAsync(10);
    await opening;
    const worker = rig.filmWorkers()[0];
    if (worker === undefined) throw new Error('no film worker');
    const pass = rig.engine.runAutoPass({ caseVersion: 1, films: [{ kind: 'front' } as CaseFileV1['films'][number]], options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId: 0 });
    const settled = outcome(pass);
    await vi.advanceTimersByTimeAsync(5);
    worker.silent = true;
    await vi.advanceTimersByTimeAsync(1999);
    expect(worker.terminated).toBe(false);
    await vi.advanceTimersByTimeAsync(2);
    expect(worker.terminated).toBe(true);
    expect(await settled).toBe('engine-crashed');
    const geometry = rig.engine.senseGeometry('front');
    await vi.advanceTimersByTimeAsync(10);
    expect(await geometry).toEqual({ name: 'front slow' });
  });
});
