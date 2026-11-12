// The parallel camera in TypeScript against the corner fixtures that were
// worked out by hand for the engine's (proving-ground/known-answers/corners),
// and its way back from a pixel to the patient.

import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import type { ImageRight, QuarterTurns, VertebraPose } from '@/shapes';
import { fittedLandmarks } from './body-landmarks';
import { type FilmCamera, type Px, filmCamera, filmToUpright, fromFilm, screenRight, toFilm, towardReader, uprightToFilm } from './film-camera';
import { rimEnds, rimThrough } from './rim-on-film';
import { cross, dot } from './vectors';

interface CornerCase {
  what: string;
  vertebra: Omit<VertebraPose, 'level'>;
  film: { kind: string; imageRight: ImageRight; quarterTurns: QuarterTurns; size: [number, number]; mmPerPx: number; mmPerPxVertical?: number; origin: [number, number] };
  expect: Record<string, Px | [Px, Px]>;
}

const FOLDER = fileURLToPath(new URL('../../../../proving-ground/known-answers/corners/', import.meta.url));
const cases = readdirSync(FOLDER)
  .filter((name) => name.endsWith('.json'))
  .flatMap((name) => (JSON.parse(readFileSync(`${FOLDER}${name}`, 'utf8')) as { cases: CornerCase[] }).cases.map((each) => ({ ...each, file: name })));

const cameraOf = (film: CornerCase['film']): FilmCamera => {
  const camera = filmCamera(
    { mmPerPx: film.mmPerPx, mmPerPxVertical: film.mmPerPxVertical ?? null, quarterTurns: film.quarterTurns, imageRight: film.imageRight },
    film.size,
    film.origin,
  );
  if (camera === null) throw new Error('no camera');
  return camera;
};

describe('the parallel camera', () => {
  it('lands every hand-worked corner where the fixtures say', () => {
    expect(cases.length).toBeGreaterThan(15);
    let checked = 0;
    for (const { what, file, vertebra, film, expect: expected } of cases) {
      const camera = cameraOf(film);
      const placed = fittedLandmarks({ ...vertebra, level: 'T7' });
      const top = rimThrough(placed.top.centre, placed.top.left, placed.top.back);
      const bottom = rimThrough(placed.bottom.centre, placed.bottom.left, placed.bottom.back);
      const axis = film.kind === 'side' ? 'front-to-back' : 'across';
      const [supFirst, supSecond] = rimEnds(top, camera, axis).map((point) => toFilm(camera, point));
      const [infFirst, infSecond] = rimEnds(bottom, camera, axis).map((point) => toFilm(camera, point));
      const got: Record<string, Px | undefined> =
        film.kind === 'side'
          ? { supA: supFirst, supP: supSecond, infA: infFirst, infP: infSecond }
          : { supL: supFirst, supR: supSecond, infL: infFirst, infR: infSecond };
      const wanted: Record<string, Px> = {};
      for (const [name, value] of Object.entries(expected)) {
        if (name === 'sup' || name === 'inf') {
          const [left, right] = value as [Px, Px];
          wanted[`${name}L`] = left;
          wanted[`${name}R`] = right;
        } else if (!name.startsWith('ped')) wanted[name] = value as Px;
      }
      for (const [name, [x, y]] of Object.entries(wanted)) {
        const at = got[name];
        expect(at, `${file}, ${what}: ${name}`).toBeDefined();
        expect(Math.hypot((at?.[0] ?? NaN) - x, (at?.[1] ?? NaN) - y), `${file}, ${what}: ${name}`).toBeLessThan(1e-9);
        checked += 1;
      }
    }
    expect(checked).toBeGreaterThan(60);
  });

  it('turns a pixel upright and back, for every quarter turn', () => {
    for (const quarterTurns of [0, 1, 2, 3] as const) {
      const camera: FilmCamera = { imageRight: 'patient-left', quarterTurns, size: [400, 900], uprightMmPerPx: [0.3, 0.3], origin: [0, 0] };
      const back = uprightToFilm(camera, filmToUpright(camera, [37, 512]));
      expect(back).toEqual([37, 512]);
    }
    // One clockwise quarter turn puts the stored film's bottom-left pixel at the upright top-left.
    const turned: FilmCamera = { imageRight: 'patient-left', quarterTurns: 1, size: [400, 900], uprightMmPerPx: [0.3, 0.3], origin: [0, 0] };
    expect(filmToUpright(turned, [0, 899])).toEqual([0, 0]);
  });

  it('finds the patient point behind a pixel: it lands back on that pixel at any distance along the beam', () => {
    for (const imageRight of ['patient-left', 'patient-right', 'anterior', 'posterior'] as const) {
      for (const quarterTurns of [0, 1, 2, 3] as const) {
        const camera = filmCamera({ mmPerPx: 0.4, mmPerPxVertical: 0.5, quarterTurns, imageRight }, quarterTurns % 2 === 0 ? [2000, 4000] : [4000, 2000], [1000, 3000]);
        if (camera === null) throw new Error('no camera');
        for (const out of [-150, 0, 80]) {
          const point = fromFilm(camera, [321.5, 1234.25], out);
          const [x, y] = toFilm(camera, point);
          expect(Math.hypot(x - 321.5, y - 1234.25)).toBeLessThan(1e-9);
          expect(dot(point, towardReader(imageRight))).toBeCloseTo(out, 9);
        }
      }
    }
  });

  it('is read from in front of a film named for the patient\'s left, and from the patient\'s right of one named for anterior', () => {
    const reader = (imageRight: ImageRight) => towardReader(imageRight).map((n) => n + 0);
    expect(reader('patient-left')).toEqual([1, 0, 0]);
    expect(reader('patient-right')).toEqual([-1, 0, 0]);
    expect(reader('anterior')).toEqual([0, -1, 0]);
    expect(reader('posterior')).toEqual([0, 1, 0]);
    // Screen right, up and toward the reader are a right-handed set on every film: none is mirrored.
    for (const imageRight of ['patient-left', 'patient-right', 'anterior', 'posterior'] as const) {
      expect(dot(cross(screenRight(imageRight), [0, 0, 1]), towardReader(imageRight))).toBe(1);
    }
  });

  it('has no camera without a facing, and takes the fit\'s scale for a film that carries none', () => {
    expect(filmCamera({ mmPerPx: 0.3, mmPerPxVertical: null, quarterTurns: 0, imageRight: null }, [100, 100], [0, 0])).toBeNull();
    expect(filmCamera({ mmPerPx: null, mmPerPxVertical: null, quarterTurns: 0, imageRight: 'patient-left' }, [100, 100], [0, 0])).toBeNull();
    expect(filmCamera({ mmPerPx: 0, mmPerPxVertical: null, quarterTurns: 0, imageRight: 'patient-left' }, [100, 100], [0, 0])).toBeNull();
    const drawn = filmCamera({ mmPerPx: null, mmPerPxVertical: null, quarterTurns: 0, imageRight: 'patient-left' }, [100, 100], [50, 80], 0.42);
    expect(drawn?.uprightMmPerPx).toEqual([0.42, 0.42]);
    // A scale the film does carry is never replaced by the fit's.
    const carried = filmCamera({ mmPerPx: 0.3, mmPerPxVertical: 0.36, quarterTurns: 1, imageRight: 'anterior' }, [100, 100], [50, 80], 0.42);
    expect(carried?.uprightMmPerPx).toEqual([0.36, 0.3]);
  });
});
