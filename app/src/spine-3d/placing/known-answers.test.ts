// The placing code against the engine's own forward model. The file read
// here was written by the engine: for twelve vertebrae, where the endplate
// centres and the body's corners are in the patient, and where each lands
// on four films. Every shipped bone is shaped to each of those vertebrae
// and stood in the patient, and its own landmarks must be where the engine
// says the body is.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import type { ImageRight, QuarterTurns, VertebraPose } from '@/shapes';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { boneLandmarks, moveLandmarks } from './body-landmarks';
import { type FilmCamera, type Px, filmCamera, toFilm } from './film-camera';
import { placingOf } from './place-vertebra';
import { rimEnds, rimThrough } from './rim-on-film';
import { archJoinOf, shaperFor } from './shape-bone';
import { type Vec3, length, sub } from './vectors';

const POINTS = ['topCentre', 'bottomCentre', 'supL', 'supR', 'infL', 'infR', 'supA', 'supP', 'infA', 'infP'] as const;
type PointName = (typeof POINTS)[number];

interface WrittenFilm {
  kind: 'front' | 'side';
  imageRight: ImageRight;
  quarterTurns: QuarterTurns;
  size: [number, number];
  mmPerPx: number;
  mmPerPxVertical: number | null;
  origin: [number, number];
}

interface KnownAnswers {
  toleranceMm: number;
  tolerancePx: number;
  films: Record<string, WrittenFilm>;
  cases: { what: string; vertebra: Omit<VertebraPose, 'level'>; points: Record<PointName, Vec3>; on: Record<string, Record<PointName, Px>> }[];
}

const answers = JSON.parse(
  readFileSync(fileURLToPath(new URL('../../../../proving-ground/known-answers/spine-3d/placed-bodies.json', import.meta.url)), 'utf8'),
) as KnownAnswers;

const cameras = Object.fromEntries(
  Object.entries(answers.films).map(([name, film]) => {
    const camera = filmCamera({ ...film, mmPerPx: film.mmPerPx }, film.size, film.origin);
    if (camera === null) throw new Error(`the ${name} film has no camera`);
    return [name, camera];
  }),
) as Record<string, FilmCamera>;

const bones = bonesFromDisk();

/** The ten points of a bone shaped to a vertebra and stood in the patient, found from the bone's own landmarks. */
function placedPoints(bone: (typeof bones)[number], vertebra: Omit<VertebraPose, 'level'>): Record<PointName, Vec3> {
  const shape = shaperFor(bone.body, vertebra, archJoinOf(bone).height);
  const placing = placingOf(vertebra);
  const placed = moveLandmarks(boneLandmarks(bone.body), (point) => placing.toPatient(shape(point)));
  const front = cameras.front;
  const side = cameras.side;
  if (!front || !side) throw new Error('the front and side films are missing');
  const top = rimThrough(placed.top.centre, placed.top.left, placed.top.back);
  const bottom = rimThrough(placed.bottom.centre, placed.bottom.left, placed.bottom.back);
  const [supL, supR] = rimEnds(top, front, 'across');
  const [infL, infR] = rimEnds(bottom, front, 'across');
  const [supA, supP] = rimEnds(top, side, 'front-to-back');
  const [infA, infP] = rimEnds(bottom, side, 'front-to-back');
  return { topCentre: placed.top.centre, bottomCentre: placed.bottom.centre, supL, supR, infL, infR, supA, supP, infA, infP };
}

describe('a bone shaped and placed, against the engine\'s forward model', () => {
  it('reads twelve vertebrae and four films, one of them mirrored and one turned', () => {
    expect(answers.cases.length).toBe(12);
    expect(Object.keys(answers.films)).toEqual(['front', 'side', 'mirroredFront', 'turnedSide']);
    expect(answers.films.mirroredFront?.imageRight).toBe('patient-right');
    expect(answers.films.turnedSide?.quarterTurns).toBe(1);
  });

  it('puts every bone\'s endplate centres and body corners where the engine puts the vertebra\'s', () => {
    let worst = 0;
    for (const { what, vertebra, points } of answers.cases) {
      for (const bone of bones) {
        const placed = placedPoints(bone, vertebra);
        for (const name of POINTS) {
          const off = length(sub(placed[name], points[name]));
          worst = Math.max(worst, off);
          expect(off, `${bone.level}, ${what}: ${name}`).toBeLessThan(answers.toleranceMm);
        }
      }
    }
    // Measured: the worst is rounding, far inside the 0.05 mm allowed.
    expect(worst).toBeLessThan(1e-9);
  });

  it('lands each of those points on the engine\'s pixel on every film', () => {
    let worst = 0;
    for (const { what, vertebra, on } of answers.cases) {
      for (const bone of bones) {
        const placed = placedPoints(bone, vertebra);
        for (const [film, camera] of Object.entries(cameras)) {
          for (const name of POINTS) {
            const [x, y] = toFilm(camera, placed[name]);
            const [wantX, wantY] = on[film]?.[name] ?? [Number.NaN, Number.NaN];
            const off = Math.hypot(x - wantX, y - wantY);
            worst = Math.max(worst, off);
            expect(off, `${bone.level}, ${what}: ${name} on ${film}`).toBeLessThan(answers.tolerancePx);
          }
        }
      }
    }
    expect(worst).toBeLessThan(1e-8);
  });
});
