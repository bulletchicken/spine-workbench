// When Find keeps only some vertebrae, the 3D spine still stands each bone on
// its own outline: the L1 bone over L1's marks, never a neighbour's.
//
// The film is the committed made-up front film, read by the trained finder in
// the release engine as Find reads it. Sets of its found levels go through the
// app's fit input and the release fit; each fitted bone is shaped as the 3D view
// shapes it and its body corners are taken to the film through the camera the
// fit reports. With one level the fit refuses, which is why Find waits for two.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { fitInputFor } from '../../app/src/bench/three/fit-input';
import { createCaseStore } from '../../app/src/case-state/case-store';
import { emptyCase } from '../../app/src/case-state/empty-case';
import { createFilmPixels } from '../../app/src/case-state/film-pixels';
import { FilmDesk, fit_from_films, initSync } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { filmRecordFrom } from '../../app/src/open-and-save/film-record';
import type { Film, FilmFacts, FilmMarks, FitFromFilms, Level, SpineFit, Step, TrainedPassResult } from '../../app/src/shapes';
import { bonesFromDisk } from '../../app/src/spine-3d/bones/bones-from-disk';
import { boneLandmarks, moveLandmarks } from '../../app/src/spine-3d/placing/body-landmarks';
import { type Px, toFilm } from '../../app/src/spine-3d/placing/film-camera';
import { fittedFilmCamera } from '../../app/src/spine-3d/placing/fitted-film-camera';
import { placingOf } from '../../app/src/spine-3d/placing/place-vertebra';
import { rimEnds, rimThrough } from '../../app/src/spine-3d/placing/rim-on-film';
import { archJoinOf, shaperFor } from '../../app/src/spine-3d/placing/shape-bone';
import { createFitSources } from '../../app/src/view-state/fit-sources';
import { madeUpFront } from './made-up-front';

const ENGINE = fileURLToPath(new URL('../../app/src/engine-room/wasm-build/wasm_bridge_bg.wasm', import.meta.url));
const bones = bonesFromDisk();
let film: Film;
let found: FilmMarks;

beforeAll(async () => {
  initSync({ module: await WebAssembly.compile(readFileSync(ENGINE)) });
  const desk = FilmDesk.open(madeUpFront(), 'front');
  try {
    film = filmRecordFrom(desk.facts() as FilmFacts);
    const pass = desk.start_trained_pass(film, 1);
    for (;;) if ((pass.advance(200) as Step).done) break;
    found = (pass.finish() as TrainedPassResult).marks;
  } finally { desk.free(); }
}, 120_000);

/** The request Find's marks make when only these levels are kept. */
function requestWith(levels: readonly Level[]): FitFromFilms {
  const store = createCaseStore({ ...emptyCase(), films: [film] });
  const sources = createFitSources(); const pixels = createFilmPixels();
  const report = store.getState().apply([{ kind: 'replace-marks', marks: { ...found, vertebrae: found.vertebrae.filter((v) => levels.includes(v.level)) }, basedOn: store.getState().version, cause: { by: 'auto-pass' } }]);
  expect(report.outcomes.every((outcome) => outcome.kept)).toBe(true);
  sources.getState().kept('front', 'trained', pixels.getState().openings.front);
  return fitInputFor(store.getState().case, sources.getState().films, pixels.getState().openings);
}

function marked(level: Level): Px[] {
  const v = found.vertebrae.find((each) => each.level === level);
  if (v?.marks.kind !== 'front') throw new Error(`no front marks for ${level}`);
  const { supL, supR, infL, infR } = v.marks.points;
  return [supL.at, supR.at, infL.at, infR.at];
}
const middle = (points: readonly Px[]): Px => [points.reduce((sum, p) => sum + p[0], 0) / points.length, points.reduce((sum, p) => sum + p[1], 0) / points.length];
const apart = (a: Px, b: Px) => Math.hypot(a[0] - b[0], a[1] - b[1]);

describe('a spine built from some of the found vertebrae', () => {
  const sets: [string, (levels: readonly Level[]) => Level[]][] = [
    ['all of them', (levels) => [...levels]],
    ['the lowest two', (levels) => levels.slice(-2)],
    ['the top four', (levels) => levels.slice(0, 4)],
    ['every other one', (levels) => levels.filter((_, i) => i % 2 === 0)],
    ['two groups with a gap between', (levels) => [...levels.slice(4, 8), ...levels.slice(-4, -1)]],
  ];
  it.each(sets)('%s: each bone is on its own outline', (_, choose) => {
    const levels = choose(found.vertebrae.map((v) => v.level).filter((level) => bones.some((bone) => bone.level === level)));
    expect(levels.length).toBeGreaterThanOrEqual(2);
    const request = requestWith(levels);
    const fit = fit_from_films(request) as SpineFit;
    expect(fit.converged).toBe(true);
    // No level is invented or dropped.
    expect(fit.vertebrae.map((v) => v.level)).toEqual(levels);
    const camera = fittedFilmCamera(request.front.geometry, request.front.size, fit.frontFilm);
    if (camera === null) throw new Error('the front film has no camera');
    for (const { level, pose } of fit.vertebrae) {
      const bone = bones.find((each) => each.level === level);
      if (bone === undefined) throw new Error(`no bone for ${level}`);
      const shape = shaperFor(bone.body, pose, archJoinOf(bone).height), placing = placingOf(pose);
      const placed = moveLandmarks(boneLandmarks(bone.body), (point) => placing.toPatient(shape(point)));
      const top = rimEnds(rimThrough(placed.top.centre, placed.top.left, placed.top.back), camera, 'across');
      const bottom = rimEnds(rimThrough(placed.bottom.centre, placed.bottom.left, placed.bottom.back), camera, 'across');
      const drawn = [top[0], top[1], bottom[0], bottom[1]].map((point) => toFilm(camera, point));
      const own = marked(level), height = apart(middle(own.slice(0, 2)), middle(own.slice(2)));
      // Within the fit's residual. Measured on three real front films: 7% of the vertebra's height at worst.
      for (const [i, corner] of drawn.entries()) expect(apart(corner, own[i] ?? corner), level).toBeLessThan(0.15 * height);
      const nearest = [...found.vertebrae].sort((a, b) => apart(middle(drawn), middle(marked(a.level))) - apart(middle(drawn), middle(marked(b.level))))[0]?.level;
      expect(nearest, level).toBe(level);
    }
  });

  it('is refused with one vertebra, so Find waits for two', () => {
    const one = found.vertebrae.map((v) => v.level).find((level) => level === 'L1') ?? found.vertebrae[0]?.level;
    if (one === undefined) throw new Error('nothing was found');
    expect(() => { fit_from_films(requestWith([one])); }).toThrow();
  });
});
