// Both fit entries through the release engine, with the made-up case's
// front corners alone and with a front and side pair.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { fit_from_films, fit_spine, initSync } from '../../app/src/engine-room/wasm-build/wasm_bridge.js';
import { FRONT_ONLY_CASE, MADE_UP_CASE, type MadeUpCase } from '../../app/src/spine-3d/made-up-case/made-up-case';
import type { FilmMarks, FilmToFit, FitFromFilms, FitInput, Mark, SpineFit } from '../../app/src/shapes';

const ENGINE = fileURLToPath(new URL('../../app/src/engine-room/wasm-build/wasm_bridge_bg.wasm', import.meta.url));
const mark = (at: readonly [number, number]): Mark => ({ at: [...at], source: 'hand', pinned: false, inferred: false });
const unseen = { seen: 'not-visible', source: 'hand' } as const;

/** Corners from the made-up films, without a pedicle reading. */
function markedFilm(fixture: MadeUpCase, side: boolean): FilmToFit {
  const film = side ? fixture.side : fixture.front;
  if (film === null) throw new Error('no side film');
  const marks: FilmMarks = {
    film: side ? 'side' : 'front',
    namedFor: side ? 'anterior' : 'patient-left',
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'user', alternatives: [], fromTop: film.vertebrae.map(({ level }) => level), fromBottom: film.vertebrae.map(({ level }) => level), flags: [], l5ConfirmedOn: null },
    vertebrae: film.vertebrae.map(({ level, marked }, index) => ({
      id: index + 1,
      level,
      confidence: { score: 1, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
      outline: null,
      marks: side
        ? { kind: 'side', points: Object.fromEntries(Object.entries(marked).map(([name, at]) => [name, mark(at)])) as Extract<FilmMarks['vertebrae'][number]['marks'], { kind: 'side' }>['points'] }
        : { kind: 'front', points: { ...Object.fromEntries(Object.entries(marked).map(([name, at]) => [name, mark(at)])), pedL: unseen, pedR: unseen } as Extract<FilmMarks['vertebrae'][number]['marks'], { kind: 'front' }>['points'] },
    })),
  };
  return { marks, geometry: film.geometry, size: film.size, markSpread: { kind: 'millimetres', sdMm: 1, sharedSdMm: 0 }, facingAssumed: false };
}

beforeAll(async () => {
  const module = await WebAssembly.compile(readFileSync(ENGINE));
  initSync({ module });
});

describe('films fitted as one spine', () => {
  it('fits front corners with built-in typical values, carrying lengths, film placement and the tilt read from the corners', () => {
    const input: FitFromFilms = { front: markedFilm(FRONT_ONLY_CASE, false), side: null, typical: 'built-in', twistReadings: [] };
    const fit = fit_from_films(input) as SpineFit;
    expect(fit.converged).toBe(true);
    expect(fit.lengths).toBe('typical-size');
    expect(fit.sideFilm).toBeNull();
    expect(fit.frontFilm.mmPerPx.every((n) => Number.isFinite(n) && n > 0)).toBe(true);
    expect(fit.frontFilm.originPx.every(Number.isFinite)).toBe(true);
    expect(fit.typicalCurveDeg).toBeGreaterThan(0);
    expect(fit.vertebrae).toHaveLength(17);
    for (const [i, vertebra] of fit.vertebrae.entries()) {
      expect(vertebra.inferred).toEqual({ depth: true, width: false, twist: true });
      expect(vertebra.cornerTiltRad).not.toBeNull();
      const saved = FRONT_ONLY_CASE.fit.vertebrae[i];
      if (saved === undefined) throw new Error('a fitted level is missing from the fixture');
      for (const key of ['centre', 'rotation'] as const) {
        vertebra.pose[key].forEach((n, axis) => {
          expect(n).toBeCloseTo(saved.pose[key][axis] ?? NaN, 8);
        });
      }
    }
    fit.frontFilm.originPx.forEach((n, axis) => {
      expect(n).toBeCloseTo(FRONT_ONLY_CASE.fit.frontFilm.originPx[axis] ?? NaN, 8);
    });
    expect(fit.frontFilm.mmPerPx).toEqual(FRONT_ONLY_CASE.fit.frontFilm.mmPerPx);
  });

  it('gives the same poses as fit_spine on a front and side pair without typical values', () => {
    const front = markedFilm(MADE_UP_CASE, false);
    const side = markedFilm(MADE_UP_CASE, true);
    const input: FitInput = { front: front.marks, side: side.marks, frontGeometry: front.geometry, sideGeometry: side.geometry, frontSize: front.size, sideSize: side.size };
    const original = fit_spine(input) as SpineFit;
    const fit = fit_from_films({ front, side, typical: 'none', twistReadings: [] } satisfies FitFromFilms) as SpineFit;
    expect(fit.lengths).toBe('measured');
    expect(fit.typicalCurveDeg).toBeNull();
    expect(fit.vertebrae.map(({ pose }) => pose)).toStrictEqual(original.vertebrae.map(({ pose }) => pose));
    expect(fit.frontFilm).toStrictEqual(original.frontFilm);
    expect(fit.sideFilm).toStrictEqual(original.sideFilm);
  });

  it('echoes Front and Side independently and refuses null input facing', () => {
    for (const frontAssumed of [false, true]) for (const sideAssumed of [false, true]) {
      const front = { ...markedFilm(MADE_UP_CASE, false), facingAssumed: frontAssumed };
      const side = { ...markedFilm(MADE_UP_CASE, true), facingAssumed: sideAssumed };
      const fit = fit_from_films({ front, side, typical: 'none', twistReadings: [] } satisfies FitFromFilms) as SpineFit;
      expect(fit.frontFilm.facingAssumed).toBe(frontAssumed);
      expect(fit.sideFilm?.facingAssumed).toBe(sideAssumed);
    }
    expect(() => { fit_from_films({ front: { ...markedFilm(FRONT_ONLY_CASE, false), facingAssumed: null }, side: null, typical: 'built-in', twistReadings: [] }); }).toThrow();
  });

  it.each(['front', 'side'] as const)('refuses malformed %s wire input without required facing metadata or with null', (film) => {
    const input = { front: markedFilm(MADE_UP_CASE, false), side: markedFilm(MADE_UP_CASE, true), typical: 'none', twistReadings: [] } satisfies FitFromFilms;
    const incomplete: Partial<FilmToFit> = { ...input[film] };
    delete incomplete.facingAssumed;
    let failure: unknown;
    try { fit_from_films({ ...input, [film]: incomplete }); }
    catch (error) { failure = error; }
    expect(failure).toMatchObject({ code: 'engine-out-of-sync', detail: `missing field ${film}.facingAssumed` });
    expect(() => { fit_from_films({ ...input, [film]: { ...input[film], facingAssumed: null } }); }).toThrow();
  });

  it('reports a supplied unusable calibrated Side as unused without changing the Front-only answer', () => {
    const front = { ...markedFilm(FRONT_ONLY_CASE, false), facingAssumed: true };
    const side = { ...markedFilm(MADE_UP_CASE, true), facingAssumed: false };
    side.marks.vertebrae = side.marks.vertebrae.slice(0, 1);
    const input = { front, side: null, typical: 'built-in', twistReadings: [] } satisfies FitFromFilms;
    const alone = fit_from_films(input) as SpineFit;
    const fallback = fit_from_films({ ...input, side }) as SpineFit;
    expect(fallback).toEqual(alone);
    expect(fallback.sideFilm).toBeNull(); expect(fallback.lengths).toBe('typical-size');
    expect(fallback.frontFilm.facingAssumed).toBe(true);
  });
});
