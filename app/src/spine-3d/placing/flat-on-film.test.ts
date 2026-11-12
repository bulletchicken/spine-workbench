import { describe, expect, it } from 'vitest';
import { filmCamera, toFilm } from './film-camera';
import { filmPlane } from './film-plane';
import { THINNEST, moved, pressedOnto } from './flat-on-film';
import { type Vec3, dot, length, sub } from './vectors';

const POINTS: Vec3[] = [
  [12, -30, 210],
  [-48, 22, 95],
  [3, 0, 0],
];

describe('a bone pressed onto its film', () => {
  for (const imageRight of ['patient-left', 'patient-right', 'anterior', 'posterior'] as const) {
    const camera = filmCamera({ mmPerPx: 0.3, mmPerPxVertical: null, quarterTurns: 0, imageRight }, [1200, 2600], [600, 2200]);
    if (camera === null) throw new Error('no camera');
    const plane = filmPlane(camera, -120);

    it(`stays on the same pixel of a film facing ${imageRight} however far it is pressed`, () => {
      for (const amount of [0, 0.25, 0.5, 1]) {
        const press = pressedOnto(plane, amount);
        for (const point of POINTS) {
          const [x, y] = toFilm(camera, moved(press, point));
          const [wasX, wasY] = toFilm(camera, point);
          expect(Math.hypot(x - wasX, y - wasY)).toBeLessThan(1e-9);
        }
      }
    });

    it(`lies on the sheet of a film facing ${imageRight} when pressed flat, and where it was when not pressed`, () => {
      for (const point of POINTS) {
        const fromSheet = dot(point, plane.towardReader) - plane.out;
        const flat = moved(pressedOnto(plane, 0), point);
        // What is kept of its distance keeps it on its own side of the sheet.
        expect(dot(flat, plane.towardReader) - plane.out).toBeCloseTo(THINNEST * fromSheet, 12);
        expect(length(sub(moved(pressedOnto(plane, 1), point), point))).toBe(0);
        const half = moved(pressedOnto(plane, 0.5), point);
        expect(dot(half, plane.towardReader) - plane.out).toBeCloseTo(0.5 * fromSheet, 12);
      }
    });
  }

  it('stands a film where its pixels say: its corners land on the film\'s own corners', () => {
    for (const quarterTurns of [0, 1, 2, 3] as const) {
      const size: [number, number] = quarterTurns % 2 === 0 ? [1200, 2600] : [2600, 1200];
      const camera = filmCamera({ mmPerPx: 0.3, mmPerPxVertical: 0.33, quarterTurns, imageRight: 'anterior' }, size, [600, 2200]);
      if (camera === null) throw new Error('no camera');
      const plane = filmPlane(camera, 75);
      const wanted = [[-0.5, -0.5], [size[0] - 0.5, -0.5], [size[0] - 0.5, size[1] - 0.5], [-0.5, size[1] - 0.5]] as const;
      plane.corners.forEach((corner, i) => {
        const [x, y] = toFilm(camera, corner);
        expect(Math.hypot(x - (wanted[i]?.[0] ?? NaN), y - (wanted[i]?.[1] ?? NaN))).toBeLessThan(1e-9);
        expect(dot(corner, plane.towardReader)).toBeCloseTo(75, 9);
      });
    }
  });
});
