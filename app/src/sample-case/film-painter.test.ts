import { describe, expect, it } from 'vitest';
import { boneOf, FILM_HEIGHT, FILM_WIDTH, toFilmPixels, truthOf, VIEWS } from './anatomy';
import { paintedFilm, seededRandom } from './film-painter';

/** The mean grey over a small square around a point of the upright film. */
function meanAround(grey: Uint8Array, x: number, y: number, half = 3): number {
  let sum = 0;
  let count = 0;
  for (let j = Math.round(y) - half; j <= Math.round(y) + half; j++) {
    for (let i = Math.round(x) - half; i <= Math.round(x) + half; i++) {
      sum += grey[j * FILM_WIDTH + i] ?? 0;
      count++;
    }
  }
  return sum / count;
}

describe('the anatomy', () => {
  it('stacks the bones as the mock does', () => {
    // Worked by hand from the mock's formulas: C7 starts at row 214 and is
    // 31 high with a 7-pixel disc; L5 is 61 high; S1 is 56 high.
    const c7 = boneOf('front', 0);
    expect(c7.yt).toBe(214);
    expect(c7.yb).toBe(245);
    expect(boneOf('front', 1).yt).toBe(252);
    expect(boneOf('front', 17).h).toBeCloseTo(61, 9);
    expect(boneOf('front', 18).h).toBe(56);
    // Widths: 58 at the top of the front film, 0.94 of that for C7, 108 at L5.
    expect(c7.w).toBeCloseTo(58 * 0.94, 9);
    expect(boneOf('front', 17).w).toBeCloseTo(108, 9);
    expect(boneOf('side', 17).w).toBeCloseTo(70, 9);
  });

  it('bends the front film where the mock does', () => {
    // 362 − 64 + 40·exp(−(455/118)²/2) at row 640.
    expect(VIEWS.front.X(640)).toBeCloseTo(298 + 40 * Math.exp(-((455 / 118) ** 2) / 2), 9);
    // The bending films lean 150 pixels at the top and not at all below row 1290.
    expect(VIEWS['bend-left'].lean(100)).toBe(150);
    expect(VIEWS['bend-right'].lean(1300)).toBeCloseTo(0, 9);
  });

  it('hides exactly one pedicle: T1 on the image right of the front-view films', () => {
    for (const kind of ['front', 'bend-left', 'bend-right'] as const) {
      const hidden = truthOf(kind).bones.flatMap((bone) => (bone.ped ?? []).filter((p) => !p.seen).map(() => bone.name));
      expect(hidden).toEqual(['T1']);
      expect(boneOf(kind, 1).ped?.[1].seen).toBe(false);
    }
    expect(truthOf('side').bones.every((bone) => bone.ped === null)).toBe(true);
  });

  it('measures the centre line by its length', () => {
    const { byS, len, sOfY } = truthOf('front');
    expect(byS).toHaveLength(len + 1);
    const step = byS.slice(1, 200).map((s, k) => Math.hypot(s.x - (byS[k]?.x ?? 0), s.y - (byS[k]?.y ?? 0)));
    expect(Math.max(...step)).toBeLessThan(1.6);
    expect(byS[sOfY(700)]?.y).toBeGreaterThanOrEqual(700);
  });

  it('puts the centre of the upright top-left pixel where the film stores it', () => {
    // Upright pixel (0, 0) covers the square (0, 0) to (1, 1), centre (0.5, 0.5).
    // Stored as it is, that is film pixel (0, 0); bending left is stored
    // turned over, so it is the stored film's last pixel, (719, 1719).
    expect(toFilmPixels('front', { x: 0.5, y: 0.5 })).toEqual([0, 0]);
    expect(toFilmPixels('bend-left', { x: 0.5, y: 0.5 })).toEqual([719, 1719]);
    expect(toFilmPixels('bend-left', { x: 719.5, y: 1719.5 })).toEqual([0, 0]);
  });
});

describe('the painted films', () => {
  it('draws the same random numbers on every run', () => {
    const a = seededRandom(11);
    const b = seededRandom(11);
    const first = Array.from({ length: 5 }, a);
    expect(Array.from({ length: 5 }, b)).toEqual(first);
    expect(first.every((value) => value >= 0 && value < 1)).toBe(true);
    expect(new Set(first).size).toBe(5);
  });

  it('paints bone brighter than lung, and the lead marker and the ball brightest of all', () => {
    const front = paintedFilm('front').clean;
    expect(front).toHaveLength(FILM_WIDTH * FILM_HEIGHT);
    const t6 = boneOf('front', 6).c;
    const leftLung = { x: 360 - 124, y: 560 };
    expect(meanAround(front, t6.x, t6.y)).toBeGreaterThan(meanAround(front, leftLung.x, leftLung.y) + 20);
    // The lead letter's stem, left of the ring's centre at (598, 150).
    expect(meanAround(front, 590, 150, 1)).toBeGreaterThan(200);
    const ball = paintedFilm('bend-right').clean;
    expect(meanAround(ball, 176, 1150, 4)).toBeGreaterThan(230);
    expect(meanAround(paintedFilm('front').clean, 176, 1150, 4)).toBeLessThan(200);
  });

  it('adds grain to the film but not to the clean picture', () => {
    const { clean, film } = paintedFilm('side');
    let differing = 0;
    for (let k = 0; k < 10000; k++) if (clean[k] !== film[k]) differing++;
    expect(differing).toBeGreaterThan(5000);
  });
});
