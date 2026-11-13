// The same intake and finder stages as Find, followed by the one adapter
// and the release fit. No saved fit or answer-key field supplies geometry.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { fitInputFor } from '../../app/src/bench/three/fit-input';
import { createCaseStore } from '../../app/src/case-state/case-store';
import { emptyCase } from '../../app/src/case-state/empty-case';
import { createFilmPixels } from '../../app/src/case-state/film-pixels';
import { ChainPass, FilmDesk, fit_from_films, initSync, make_up_spine } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { SAMPLE_RECIPE } from '../../app/src/made-up-spines/the-sample-case';
import { filmRecordFrom } from '../../app/src/open-and-save/film-record';
import { compareLevels, type ChainAnswer, type Film, type FilmFacts, type FilmKind, type FilmMarks, type FilmPassResult, type FrontFinder, type MadeUpSpine, type SpineFit, type Step, type TrainedPassResult } from '../../app/src/shapes';
import { createFitSources } from '../../app/src/view-state/fit-sources';
import { madeUpFront, TRAINED_FRONT_RECIPE } from './made-up-front';

const ENGINE = fileURLToPath(new URL('../../app/src/engine-room/wasm-build/wasm_bridge_bg.wasm', import.meta.url));
const COMMITTED_FRONT = new URL('../../app/src/spine-3d/made-up-case/front-film.png', import.meta.url);

beforeAll(async () => { initSync({ module: await WebAssembly.compile(readFileSync(ENGINE)) }); });

function finish<T>(pass: { advance(units: number): unknown; finish(): T; free(): void }): T {
  let consumed = false;
  try {
    // Same bounded engine stages as the worker, no browser clock needed.
    for (;;) { if ((pass.advance(200) as Step).done) break; }
    consumed = true;
    return pass.finish();
  } finally { if (!consumed) pass.free(); }
}

function open(png: Uint8Array, kind: FilmKind) {
  const desk = FilmDesk.open(png, kind);
  const film = filmRecordFrom(desk.facts() as FilmFacts);
  return { desk, film };
}

function applyAndFit(films: Film[], marks: FilmMarks[], finder: FrontFinder): SpineFit {
  const store = createCaseStore({ ...emptyCase(), films });
  const sources = createFitSources(); const pixels = createFilmPixels();
  const version = store.getState().version;
  const report = store.getState().apply(marks.map((marks) => ({ kind: 'replace-marks', marks, basedOn: version, cause: { by: 'auto-pass' } })));
  expect(report.outcomes.every((outcome) => outcome.kept)).toBe(true);
  for (const film of marks) sources.getState().kept(film.film, film.film === 'front' ? finder : 'hand-built', pixels.getState().openings[film.film]);
  const before = structuredClone(store.getState().case);
  const input = fitInputFor(store.getState().case, sources.getState().films, pixels.getState().openings);
  for (const film of [input.front, input.side].filter((film) => film !== null)) {
    expect(film.geometry.mmPerPx).toBeNull(); expect(film.geometry.mmPerPxVertical).toBeNull();
    expect(film.facingAssumed).toBe(true);
    expect(Object.values(film.geometry.confirmed).every((confirmed) => !confirmed)).toBe(true);
    expect(film.marks).toEqual(store.getState().case.marks.find((marks) => marks.film === film.marks.film));
  }
  const fit = fit_from_films(input) as SpineFit;
  expect(store.getState().case).toEqual(before);
  expect(fit.converged).toBe(true); expect(fit.lengths).toBe('typical-size');
  expect(fit.frontFilm.facingAssumed).toBe(true);
  if (fit.sideFilm !== null) expect(fit.sideFilm.facingAssumed).toBe(true);
  const standing = fit.vertebrae.filter((v) => compareLevels(v.level, 'T1') >= 0 && compareLevels(v.level, 'L5') <= 0);
  expect(standing.length).toBeGreaterThan(0);
  for (const { pose, sd } of fit.vertebrae) {
    expect([...pose.centre, ...pose.rotation, pose.wedgeFront, pose.wedgeSide, ...sd.centreMm, ...sd.anglesRad].every(Number.isFinite)).toBe(true);
    expect([pose.height, pose.topHalfWidth, pose.topHalfDepth, pose.bottomHalfWidth, pose.bottomHalfDepth].every((n) => Number.isFinite(n) && n > 0)).toBe(true);
  }
  console.info('intake fit', JSON.stringify({ finder, found: marks.map((m) => ({ film: m.film, levels: m.vertebrae.map((v) => v.level), flags: m.naming.flags, c7: m.landmarks.c7 })), fitted: fit.vertebrae.map((v) => v.level), converged: fit.converged, stop: fit.stopReason, iterations: fit.iterations, sideUsed: fit.sideFilm !== null, lengths: fit.lengths }));
  return fit;
}

describe('ordinary intake to the release fit', () => {
  it('finds all four made-up films through film-pass, chain and points before fitting the standing films', () => {
    const pngs = (make_up_spine(SAMPLE_RECIPE) as MadeUpSpine).films;
    const opened = pngs.map(({ png, kind }) => open(png, kind));
    try {
      const results = opened.map(({ desk, film }) => finish<FilmPassResult>(desk.start_film_pass(film)));
      const answer = finish<ChainAnswer>(ChainPass.start(results, { acquisition: 'unknown', population: 'adolescent', template: null }));
      let first = emptyCase().nextVertebraId;
      const marks = opened.map(({ desk, film }) => {
        const chain = answer.films.find((chain) => chain.film === film.kind);
        if (chain === undefined) throw new Error('the chain omitted a film');
        const marks = desk.place_points(chain, first) as FilmMarks;
        for (const v of marks.vertebrae) first = Math.max(first, v.id + 1);
        return marks;
      });
      console.info('four-film recipe', JSON.stringify(SAMPLE_RECIPE));
      applyAndFit(opened.map(({ film }) => film), marks, 'hand-built');
    } finally { for (const { desk } of opened) desk.free(); }
  }, 300_000);

  it('probes the committed Front with the trained finder and fits a real trained result alone', () => {
    const trained = (png: Uint8Array) => {
      const { desk, film } = open(png, 'front');
      try { return { film, result: finish<TrainedPassResult>(desk.start_trained_pass(film, emptyCase().nextVertebraId)) }; }
      finally { desk.free(); }
    };
    let selected: ReturnType<typeof trained>;
    try {
      selected = trained(readFileSync(COMMITTED_FRONT));
      console.info('committed Front trained probe completed', JSON.stringify(selected.result.marks.vertebrae.map((v) => v.level)));
    } catch (error) {
      console.info('committed Front trained probe refused', JSON.stringify(error));
      selected = trained(madeUpFront());
      console.info('trained Front fallback recipe', JSON.stringify(TRAINED_FRONT_RECIPE));
    }
    const fit = applyAndFit([selected.film], [selected.result.marks], 'trained');
    expect(fit.sideFilm).toBeNull();
    expect(fit.vertebrae.every((v) => v.rmsMm.side === null && v.inferred.depth)).toBe(true);
  }, 300_000);
});
