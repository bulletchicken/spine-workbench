import type { QuarterTurns } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { apply, filmSpace, uprightSize, type FilmPlacement } from '../film-space';
import { fit, fitZoom, isFit, keepInside, lookPoint, MAX_ZOOM_OVER_FIT, panBy, travelTo, zoomAt, zoomOverFit, type CameraFrame } from './camera';

const noMargins = { top: 0, right: 0, bottom: 0, left: 0 };

/** A 100 × 200 film in a 300 × 500 viewport, upright as stored. Fit zoom 2.5 by hand. */
const plain: CameraFrame = {
  size: { width: 100, height: 200 },
  view: { quarterTurns: 0, mirrored: false },
  viewport: { width: 300, height: 500 },
  margins: noMargins,
};

function near(actual: readonly number[], expected: readonly number[], tolerance = 1e-9) {
  expect(actual).toHaveLength(expected.length);
  actual.forEach((value, i) => {
    expect(Math.abs(value - (expected[i] ?? Number.NaN))).toBeLessThanOrEqual(tolerance * Math.max(1, Math.abs(value)));
  });
}

const frames = fc.record({
  size: fc.record({ width: fc.integer({ min: 50, max: 6000 }), height: fc.integer({ min: 50, max: 9000 }) }),
  view: fc.record({ quarterTurns: fc.constantFrom<QuarterTurns>(0, 1, 2, 3), mirrored: fc.boolean() }),
  viewport: fc.record({ width: fc.integer({ min: 120, max: 1600 }), height: fc.integer({ min: 120, max: 1200 }) }),
  margins: fc.record({
    top: fc.integer({ min: 0, max: 40 }),
    right: fc.integer({ min: 0, max: 40 }),
    bottom: fc.integer({ min: 0, max: 40 }),
    left: fc.integer({ min: 0, max: 40 }),
  }),
}) satisfies fc.Arbitrary<CameraFrame>;

/** One camera move: a wheel notch at a point, a drag, or a trip to a film point, each as fractions of the viewport or film. */
const moves = fc.oneof(
  fc.record({ kind: fc.constant('zoom' as const), factor: fc.double({ min: 0.3, max: 4, noNaN: true }), u: fc.double({ min: 0, max: 1, noNaN: true }), v: fc.double({ min: 0, max: 1, noNaN: true }) }),
  fc.record({ kind: fc.constant('pan' as const), by: fc.tuple(fc.double({ min: -3000, max: 3000, noNaN: true }), fc.double({ min: -3000, max: 3000, noNaN: true })) }),
  fc.record({ kind: fc.constant('travel' as const), over: fc.double({ min: 1, max: MAX_ZOOM_OVER_FIT, noNaN: true }), u: fc.double({ min: 0, max: 1, noNaN: true }), v: fc.double({ min: 0, max: 1, noNaN: true }) }),
);

/** A frame and the placements a camera reaches from its fit by a run of moves: every one of them. */
const cameraRuns = fc.tuple(frames, fc.array(moves, { maxLength: 6 })).map(([frame, run]) => {
  const reached: FilmPlacement[] = [fit(frame)];
  for (const move of run) {
    const from = reached[reached.length - 1] ?? fit(frame);
    const { width, height } = frame.viewport;
    if (move.kind === 'zoom') reached.push(zoomAt(frame, from, move.factor, [move.u * width, move.v * height]));
    else if (move.kind === 'pan') reached.push(panBy(frame, from, move.by));
    else reached.push(travelTo(frame, [move.u * frame.size.width - 0.5, move.v * frame.size.height - 0.5], move.over));
  }
  return { frame, reached };
});

describe('every placement a camera makes', () => {
  it('takes a screen point to the film and back to the same screen point, under every turn and mirror', () => {
    fc.assert(
      fc.property(cameraRuns, fc.double({ min: 0, max: 1, noNaN: true }), fc.double({ min: 0, max: 1, noNaN: true }), ({ frame, reached }, u, v) => {
        const screen: [number, number] = [u * frame.viewport.width, v * frame.viewport.height];
        for (const placement of reached) {
          const space = filmSpace(placement);
          // Through the matrix the layers' groups are set to, not toScreen, so this is what a mark drawn at that film point shows.
          near(apply(space.matrix, space.toFilm(screen)), screen, 1e-9);
        }
      }),
    );
  });
});

describe('fit', () => {
  it('shows the whole film as large as the viewport allows, centred', () => {
    expect(fitZoom(plain)).toBe(2.5);
    const placement = fit(plain);
    expect(placement.zoom).toBe(2.5);
    // 250 wide in 300: 25 each side. 500 tall in 500: flush.
    expect(placement.pan).toEqual([25, 0]);
    // The centre of the top-left pixel is half a film pixel in: 1.25 screen pixels.
    expect(filmSpace(placement).toScreen([0, 0])).toEqual([26.25, 1.25]);
  });

  it('keeps the margins clear and stays centred where it can', () => {
    const frame = { ...plain, margins: { top: 12, right: 8, bottom: 12, left: 34 } };
    // Room 258 × 476, so zoom = min(2.58, 2.38) = 2.38: the height decides.
    expect(fitZoom(frame)).toBeCloseTo(2.38, 12);
    const placement = fit(frame);
    // 238 wide: centred would be at 31, which is inside the 34 margin, so 34.
    near(placement.pan, [34, 12]);
  });

  it('fits the turned film, not the stored one', () => {
    const frame: CameraFrame = { ...plain, view: { quarterTurns: 1, mirrored: false }, viewport: { width: 500, height: 300 } };
    // Turned, the film is 200 × 100 in 500 × 300: zoom 2.5, 500 × 250, so 25 above and below.
    expect(fitZoom(frame)).toBe(2.5);
    expect(fit(frame).pan).toEqual([0, 25]);
    // A quarter turn clockwise puts the stored top-left pixel at the top right.
    near(filmSpace(fit(frame)).toScreen([0, 0]), [500 - 1.25, 25 + 1.25]);
  });

  it('places every corner of the film inside the margins', () => {
    fc.assert(
      fc.property(frames, (frame) => {
        fc.pre(frame.viewport.width > frame.margins.left + frame.margins.right + 10);
        fc.pre(frame.viewport.height > frame.margins.top + frame.margins.bottom + 10);
        const placement = fit(frame);
        const upright = uprightSize(frame.size, frame.view.quarterTurns);
        const [left, top] = placement.pan;
        const right = left + upright.width * placement.zoom;
        const bottom = top + upright.height * placement.zoom;
        const slack = 1e-6;
        expect(left).toBeGreaterThanOrEqual(frame.margins.left - slack);
        expect(top).toBeGreaterThanOrEqual(frame.margins.top - slack);
        expect(right).toBeLessThanOrEqual(frame.viewport.width - frame.margins.right + slack);
        expect(bottom).toBeLessThanOrEqual(frame.viewport.height - frame.margins.bottom + slack);
        // And it touches the margins along at least one axis.
        const fillsAcross = Math.abs(right - left - (frame.viewport.width - frame.margins.left - frame.margins.right)) < slack * 100;
        const fillsDown = Math.abs(bottom - top - (frame.viewport.height - frame.margins.top - frame.margins.bottom)) < slack * 100;
        expect(fillsAcross || fillsDown).toBe(true);
      }),
    );
  });
});

describe('zoom at the pointer', () => {
  it('keeps the film point under the pointer where it is (worked by hand)', () => {
    const placement = zoomAt(plain, fit(plain), 2, [150, 250]);
    // Under (150, 250) at fit is (50, 100) on the upright film from its outer
    // corner. At zoom 5 that point stays at (150, 250): pan = (150 − 250, 250 − 500).
    expect(placement.zoom).toBe(5);
    expect(placement.pan).toEqual([-100, -250]);
    near(filmSpace(placement).toFilm([150, 250]), [49.5, 99.5]);
  });

  it('never goes past six times the fit, nor below the fit', () => {
    // The fit is 2.5 screen pixels per film pixel; six times that is 15.
    expect(zoomAt(plain, fit(plain), 100, [150, 250]).zoom).toBe(15);
    const near1 = zoomAt(plain, zoomAt(plain, fit(plain), 3, [10, 10]), 1 / 50, [200, 300]);
    expect(near1).toEqual(fit(plain));
    expect(isFit(plain, near1)).toBe(true);
  });

  it('keeps the point under the pointer for any frame when zooming about the film centre', () => {
    fc.assert(
      fc.property(frames, fc.double({ min: 1, max: MAX_ZOOM_OVER_FIT, noNaN: true }), (frame, factor) => {
        const symmetric = { ...frame, margins: { top: 10, right: 10, bottom: 10, left: 10 } };
        const start = fit(symmetric);
        const centre: [number, number] = [(frame.size.width - 1) / 2, (frame.size.height - 1) / 2];
        const at = filmSpace(start).toScreen(centre);
        const after = zoomAt(symmetric, start, factor, at);
        near(filmSpace(after).toScreen(centre), at, 1e-6);
      }),
    );
  });

  it('keeps the box inside the margins covered once the film is larger than it', () => {
    fc.assert(
      fc.property(
        frames,
        fc.array(fc.tuple(fc.double({ min: 0.5, max: 3, noNaN: true }), fc.double({ min: 0, max: 1, noNaN: true }), fc.double({ min: 0, max: 1, noNaN: true })), {
          minLength: 1,
          maxLength: 8,
        }),
        fc.tuple(fc.double({ min: -5000, max: 5000, noNaN: true }), fc.double({ min: -5000, max: 5000, noNaN: true })),
        (frame, zooms, drag) => {
          fc.pre(frame.viewport.width > frame.margins.left + frame.margins.right + 10);
          fc.pre(frame.viewport.height > frame.margins.top + frame.margins.bottom + 10);
          let placement = fit(frame);
          for (const [factor, u, v] of zooms) {
            placement = zoomAt(frame, placement, factor, [u * frame.viewport.width, v * frame.viewport.height]);
          }
          placement = panBy(frame, placement, drag);
          const upright = uprightSize(frame.size, frame.view.quarterTurns);
          const width = upright.width * placement.zoom;
          const height = upright.height * placement.zoom;
          const { viewport, margins } = frame;
          const slack = 1e-6 * Math.max(width, height);
          if (width > viewport.width - margins.left - margins.right + slack) {
            expect(placement.pan[0]).toBeLessThanOrEqual(margins.left + slack);
            expect(placement.pan[0] + width).toBeGreaterThanOrEqual(viewport.width - margins.right - slack);
          }
          if (height > viewport.height - margins.top - margins.bottom + slack) {
            expect(placement.pan[1]).toBeLessThanOrEqual(margins.top + slack);
            expect(placement.pan[1] + height).toBeGreaterThanOrEqual(viewport.height - margins.bottom - slack);
          }
          expect(zoomOverFit(frame, placement)).toBeGreaterThanOrEqual(1 - 1e-9);
          expect(zoomOverFit(frame, placement)).toBeLessThanOrEqual(MAX_ZOOM_OVER_FIT + 1e-9);
        },
      ),
    );
  });
});

describe('pan', () => {
  it('moves a zoomed film and stops at its edge', () => {
    const zoomed = zoomAt(plain, fit(plain), 2, [150, 250]);
    expect(panBy(plain, zoomed, [30, -40]).pan).toEqual([-70, -290]);
    // 500 wide in 300: the pan runs from −200 to 0.
    expect(panBy(plain, zoomed, [1000, 0]).pan[0]).toBe(0);
    expect(panBy(plain, zoomed, [-1000, 0]).pan[0]).toBe(-200);
  });

  it('does nothing at fit', () => {
    expect(panBy(plain, fit(plain), [40, 40])).toEqual(fit(plain));
  });
});

describe('travel', () => {
  it('puts the point 47% of the way down the viewport, across the middle', () => {
    const placement = travelTo(plain, [49.5, 99.5], 2);
    expect(placement.zoom).toBe(5);
    // (49.5, 99.5) is 50, 100 from the outer corner: pan = (150 − 250, 235 − 500).
    expect(placement.pan).toEqual([-100, -265]);
    near(filmSpace(placement).toScreen([49.5, 99.5]), [150, 235]);
    near(lookPoint(plain, placement), [49.5, 99.5]);
  });

  it('lands on the same point whichever way the film is turned', () => {
    fc.assert(
      fc.property(frames, fc.double({ min: 0.2, max: 0.8, noNaN: true }), fc.double({ min: 0.3, max: 0.7, noNaN: true }), (frame, u, v) => {
        const point: [number, number] = [u * frame.size.width, v * frame.size.height];
        // At six times the fit the film is far larger than the viewport, so nothing clamps a point this far in.
        const placement = travelTo(frame, point, MAX_ZOOM_OVER_FIT);
        fc.pre(Math.min(...uprightDims(frame).map((d) => d * placement.zoom)) > 4 * Math.max(frame.viewport.width, frame.viewport.height));
        near(lookPoint(frame, placement), point, 1e-6);
      }),
    );
  });

  it('stops at the film’s edge rather than show the room', () => {
    const placement = travelTo(plain, [0, 0], 2);
    expect(placement.pan).toEqual([0, 0]);
  });
});

describe('keep inside', () => {
  it('is the fit at the fit zoom, whatever the pan', () => {
    expect(keepInside(plain, 2.5, [999, -999])).toEqual(fit(plain));
  });
});

function uprightDims(frame: CameraFrame): [number, number] {
  const upright = uprightSize(frame.size, frame.view.quarterTurns);
  return [upright.width, upright.height];
}
