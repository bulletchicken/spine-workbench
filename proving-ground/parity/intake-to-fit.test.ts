import { layoutInput } from '../../app/src/bench/place/layout-input';
// The same intake and finder stages as Find, followed by the one adapter
// and the release fit. No saved fit or answer-key field supplies geometry.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { fitInputFor } from '../../app/src/bench/three/fit-input';
import { createCaseStore } from '../../app/src/case-state/case-store';
import { emptyCase } from '../../app/src/case-state/empty-case';
import { createFilmPixels } from '../../app/src/case-state/film-pixels';
import { ChainPass, FilmDesk, fit_from_films, initSync, make_up_spine, measure_spine, propose_layouts } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { SAMPLE_RECIPE } from '../../app/src/made-up-spines/the-sample-case';
import { filmRecordFrom } from '../../app/src/open-and-save/film-record';
import { compareLevels, type AnswerKey, type ChainAnswer, type Film, type FilmFacts, type FilmKind, type FilmMarks, type FilmPassResult, type FitFromFilms, type FrontFinder, type MadeUpSpine, type LayoutProposals, type Reading, type SpineReadings, type SpineFit, type Step, type TrainedPassResult, type VertebraPose } from '../../app/src/shapes';
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

function applyAndFit(films: Film[], marks: FilmMarks[], finder: FrontFinder): { fit: SpineFit; input: FitFromFilms } {
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
  return { fit, input };
}

function value(reading: Reading): number {
  if (reading.kind !== 'measured') throw new Error('a measured angle was expected');
  return reading.valueDeg;
}

/** Independent rotation of the answer key's endplate normal. */
function normal(pose: VertebraPose, top: boolean): [number, number, number] {
  const sign = top ? 1 : -1;
  const front = sign * pose.wedgeFront / 2; const side = sign * pose.wedgeSide / 2;
  const v: [number, number, number] = [Math.sin(side), -Math.sin(front) * Math.cos(side), Math.cos(front) * Math.cos(side)];
  const angle = Math.hypot(...pose.rotation);
  if (angle === 0) return v;
  const [x, y, z] = pose.rotation.map((r) => r / angle) as [number, number, number];
  const c = Math.cos(angle); const s = Math.sin(angle); const dot = x * v[0] + y * v[1] + z * v[2];
  return [c * v[0] + s * (y * v[2] - z * v[1]) + (1 - c) * x * dot,
    c * v[1] + s * (z * v[0] - x * v[2]) + (1 - c) * y * dot,
    c * v[2] + s * (x * v[1] - y * v[0]) + (1 - c) * z * dot];
}

function agreesWithTruth(readings: SpineReadings, key: AnswerKey) {
  const main = readings.curves.filter((c) => c.region === 'main-thoracic').toSorted((a, b) => Math.abs(value(b.cobb)) - Math.abs(value(a.cobb)))[0];
  const truth = key.angles.frontalCobb.find((c) => c.region === 'main-thoracic');
  if (main === undefined || truth === undefined) throw new Error('no main curve');
  expect(Math.abs(Math.abs(value(main.cobb)) - truth.degrees)).toBeLessThanOrEqual(6);
  const levels = key.poses.map((p) => p.level);
  const front = key.films.find((f) => f.kind === 'front');
  if (front === undefined || front.geometry.mmPerPx === null) throw new Error('no exact front geometry');
  const sx = front.geometry.mmPerPx; const sy = front.geometry.mmPerPxVertical ?? sx;
  const span = front.vertebrae.filter((v) => compareLevels(v.level, 'T5') >= 0 && compareLevels(v.level, 'T12') <= 0);
  const tilt = (index: number, top: boolean) => {
    const marks = span[index]?.marks;
    if (marks?.kind !== 'front') throw new Error('no true front marks');
    const left = top ? marks.supL : marks.infL; const right = top ? marks.supR : marks.infR;
    return Math.atan2((right.at[1] - left.at[1]) * sy, (left.at[0] - right.at[0]) * sx) * 180 / Math.PI;
  };
  let ends: [number, number, number] = [0, 0, 0];
  for (let u = 0; u < span.length; u += 1) for (let l = u; l < span.length; l += 1) {
    const angle = Math.abs(tilt(l, false) - tilt(u, true));
    if (angle > ends[2]) ends = [u, l, angle];
  }
  const upper = span[ends[0]]; const lower = span[ends[1]];
  if (upper === undefined || lower === undefined) throw new Error('no true main-curve ends');
  const index = (level: VertebraPose['level']) => {
    const at = levels.indexOf(level);
    if (at < 0) throw new Error('a fitted curve end is absent from the answer key');
    return at;
  };
  expect(Math.abs(index(main.upperEnd) - index(upper.level))).toBeLessThanOrEqual(1);
  expect(Math.abs(index(main.lowerEnd) - index(lower.level))).toBeLessThanOrEqual(1);
  for (const curve of readings.curves) expect(Math.abs(index(curve.lowerEnd) - index(curve.upperEnd))).toBeGreaterThan(1);
  const sagittal = (level: VertebraPose['level'], top: boolean) => {
    const pose = key.poses.find((p) => p.level === level);
    if (pose === undefined) throw new Error('no true pose');
    const n = normal(pose, top); return Math.atan2(n[0], n[2]) * 180 / Math.PI;
  };
  expect(Math.abs(value(readings.sagittal.kyphosisT5T12) - (sagittal('T5', true) - sagittal('T12', false)))).toBeLessThanOrEqual(8);
  expect(Math.abs(value(readings.sagittal.lordosisL1L5) - (sagittal('L5', false) - sagittal('L1', true)))).toBeLessThanOrEqual(8);
}

describe('ordinary intake to the release fit', () => {
  it('finds all four made-up films through film-pass, chain and points before fitting the standing films', () => {
    const made = make_up_spine(SAMPLE_RECIPE) as MadeUpSpine;
    const pngs = made.films;
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
      const { fit, input } = applyAndFit(opened.map(({ film }) => film), marks, 'hand-built');
      const placement = propose_layouts(layoutInput(fit, { ...emptyCase(), films: opened.map(({ film }) => film), marks })) as LayoutProposals;
      expect(placement.firstIsSketch).toBe(false);
      expect(placement.layouts[0]).toMatchObject({ upper: 'T3', lower: 'L1' });
      expect(placement.pairs.find((pair) => pair.upper === 'T3' && pair.lower === 'L1')?.versionsHolding).toBe(5);
      expect(placement.cannotTell).toContainEqual({ code: 'ends-unsettled', levels: ['T3', 'L1'] });
      if (fit.sideFilm === null) {
        expect(fit).toEqual(fit_from_films({ ...input, side: null }));
      } else {
        const readings = measure_spine({ fit, front: input.front, side: input.side, bendLeft: null, bendRight: null, versions: 4, seed: SAMPLE_RECIPE.seed }) as SpineReadings;
        agreesWithTruth(readings, made.answerKey);
      }
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
    const { fit } = applyAndFit([selected.film], [selected.result.marks], 'trained');
    expect(fit.sideFilm).toBeNull();
    expect(fit.vertebrae.every((v) => v.rmsMm.side === null && v.inferred.depth)).toBe(true);
  }, 300_000);
});
