import type { QuarterTurns } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { fit, MAX_ZOOM_OVER_FIT, panBy, travelTo, zoomAt, type CameraFrame } from '../camera/camera';
import { filmSpace, type FilmPlacement } from '../film-space';
import { filmUniforms } from './film-shader';

/** What GLSL's `mat3 * vec3` gives for a matrix handed over column by column: row r, column c is m[3c + r]. */
function glslTimes(m: readonly number[], v: readonly [number, number, number]): number[] {
  return [0, 1, 2].map((r) => [0, 1, 2].reduce((sum, c) => sum + (m[3 * c + r] ?? Number.NaN) * (v[c] ?? Number.NaN), 0));
}

/** The film point the shader asks for at a screen point. */
function shaderToFilm(u: ReturnType<typeof filmUniforms>, [x, y]: readonly [number, number]): number[] {
  return glslTimes(u.toFilm, [x, y, 1]);
}

function near(actual: readonly number[], expected: readonly number[], tolerance = 1e-9) {
  expect(actual).toHaveLength(expected.length);
  actual.forEach((value, i) => {
    expect(Math.abs(value - (expected[i] ?? Number.NaN))).toBeLessThanOrEqual(tolerance * Math.max(1, Math.abs(expected[i] ?? 0)));
  });
}

describe('the film shader’s numbers', () => {
  const scene = {
    placement: { size: { width: 100, height: 200 }, view: { quarterTurns: 0 as const, mirrored: false }, zoom: 2, pan: [10, 20] as const },
    look: { centre: 0.4, width: 0.5, flipped: true },
    canvas: { deviceWidth: 600, deviceHeight: 1000, cssWidth: 300, cssHeight: 500 },
    corner: 3,
    texels: { width: 100, height: 200 },
    step: 1,
  };

  it('maps the screen back to film pixels with the centre of the top-left pixel at (0, 0)', () => {
    const u = filmUniforms(scene);
    // Screen (11, 21) is 10 + 2 · 0.5 across and 20 + 2 · 0.5 down: film (0, 0).
    expect(shaderToFilm(u, [11, 21])).toEqual([0, 0, 1]);
    expect(shaderToFilm(u, [10, 20])).toEqual([-0.5, -0.5, 1]);
    expect(shaderToFilm(u, [210, 420])).toEqual([99.5, 199.5, 1]);
  });

  it('maps the screen back to film pixels on a film turned a quarter, worked by hand', () => {
    // Turned clockwise, the 100 × 200 film shows 200 wide and 100 tall. Film pixel (x, y) sits upright
    // at (199 − y, x), so on screen at (10 + 2 · (199 − y + 0.5), 20 + 2 · (x + 0.5)).
    const u = filmUniforms({ ...scene, placement: { ...scene.placement, view: { quarterTurns: 1, mirrored: false } } });
    near(shaderToFilm(u, [409, 21]), [0, 0, 1]);
    near(shaderToFilm(u, [11, 21]), [0, 199, 1]);
    near(shaderToFilm(u, [409, 219]), [99, 0, 1]);
  });

  it('maps the screen back to film pixels on a mirrored film, worked by hand', () => {
    // Mirrored, film pixel (x, y) shows at (99 − x, y): on screen at (10 + 2 · (99 − x + 0.5), 20 + 2 · (y + 0.5)).
    const u = filmUniforms({ ...scene, placement: { ...scene.placement, view: { quarterTurns: 0, mirrored: true } } });
    near(shaderToFilm(u, [11, 21]), [99, 0, 1]);
    near(shaderToFilm(u, [209, 21]), [0, 0, 1]);
    near(shaderToFilm(u, [209, 419]), [0, 199, 1]);
  });

  it('undoes the film-space matrix exactly, for every placement a camera makes, under every turn and mirror', () => {
    const frames = fc.record({
      size: fc.record({ width: fc.integer({ min: 20, max: 6000 }), height: fc.integer({ min: 20, max: 9000 }) }),
      view: fc.record({ quarterTurns: fc.constantFrom<QuarterTurns>(0, 1, 2, 3), mirrored: fc.boolean() }),
      viewport: fc.record({ width: fc.integer({ min: 120, max: 1600 }), height: fc.integer({ min: 120, max: 1200 }) }),
      margins: fc.constant({ top: 12, right: 8, bottom: 12, left: 34 }),
    }) satisfies fc.Arbitrary<CameraFrame>;
    const fraction = fc.double({ min: 0, max: 1, noNaN: true });
    const placed = fc
      .tuple(frames, fc.constantFrom('fit', 'zoomAt', 'panBy', 'travelTo'), fc.double({ min: 1, max: MAX_ZOOM_OVER_FIT, noNaN: true }), fraction, fraction)
      .map(([frame, how, over, u, v]): FilmPlacement => {
        const at: [number, number] = [u * frame.viewport.width, v * frame.viewport.height];
        if (how === 'fit') return fit(frame);
        if (how === 'zoomAt') return zoomAt(frame, fit(frame), over, at);
        if (how === 'panBy') return panBy(frame, zoomAt(frame, fit(frame), over, at), [at[0] - 300, at[1] - 300]);
        return travelTo(frame, [u * frame.size.width - 0.5, v * frame.size.height - 0.5], over);
      });
    fc.assert(
      fc.property(placed, fraction, fraction, (placement, u, v) => {
        const uniforms = filmUniforms({ ...scene, placement, texels: placement.size });
        const film: [number, number] = [u * placement.size.width - 0.5, v * placement.size.height - 0.5];
        const screen = filmSpace(placement).toScreen(film);
        near(shaderToFilm(uniforms, screen), [...film, 1], 1e-9);
      }),
    );
  });

  it('knows how many texels a device pixel spans, and where the film’s edges are', () => {
    const u = filmUniforms(scene);
    // Two device pixels per CSS pixel and two CSS pixels per film pixel: a quarter texel each.
    expect(u.cssPerDevice).toEqual([0.5, 0.5]);
    expect(u.texelsPerDevice).toBe(0.25);
    expect(u.edges).toEqual([10, 20, 210, 420]);
    expect(u.deviceHeight).toBe(1000);
  });

  it('turns the look into the values shown as black and white', () => {
    const u = filmUniforms(scene);
    expect(u.window[0]).toBeCloseTo(0.15, 12);
    expect(u.window[1]).toBeCloseTo(0.65, 12);
    expect(u.flipped).toBe(true);
  });

  it('counts texels, not film pixels, on a film shown from a copy half its size', () => {
    // Half a CSS pixel per device pixel, two CSS pixels per film pixel, two film pixels per texel: an eighth of a texel.
    const u = filmUniforms({ ...scene, texels: { width: 50, height: 100 }, step: 2 });
    expect(u.texelsPerDevice).toBe(0.125);
    expect(u.step).toBe(2);
  });

  it('measures the edges on the turned film', () => {
    const turned = filmUniforms({ ...scene, placement: { ...scene.placement, view: { quarterTurns: 1, mirrored: false } } });
    expect(turned.edges).toEqual([10, 20, 410, 220]);
  });
});
