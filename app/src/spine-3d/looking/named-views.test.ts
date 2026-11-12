import { describe, expect, it } from 'vitest';
import { centreOf, fittedScale, namedView } from './framing';
import { toCanvas } from './lens';
import { NAMED_VIEWS, azimuthAlong, perspectiveFor, shortWayRound } from './named-views';
import { viewAxes } from './view';
import { toScene } from '../placing/scene-axes';
import { screenRight } from '../placing/film-camera';
import { dot } from '../placing/vectors';

describe('the named views', () => {
  it('look along an axis with no perspective, except the three-quarter view', () => {
    expect(NAMED_VIEWS.front.perspective).toBe(0);
    expect(NAMED_VIEWS.side.perspective).toBe(0);
    expect(NAMED_VIEWS.top.perspective).toBe(0);
    expect(NAMED_VIEWS['three-quarter']).toMatchObject({ azimuth: 34, elevation: 14, perspective: 1 });
  });

  it('read a film from where its facing says: screen right is the direction the film is named for', () => {
    for (const imageRight of ['patient-left', 'patient-right', 'anterior', 'posterior'] as const) {
      const { right } = viewAxes(azimuthAlong(imageRight), 0);
      expect(dot(toScene(screenRight(imageRight)), right)).toBeCloseTo(1, 15);
    }
  });

  it('give a view turned by hand no perspective along an axis and the whole lens well off one', () => {
    for (const [azimuth, elevation] of [[0, 0], [180, 0], [90, 0], [-90, 0], [0, 90], [57, 90]] as const) {
      expect(perspectiveFor(azimuth, elevation)).toBeCloseTo(0, 12);
    }
    expect(perspectiveFor(34, 14)).toBe(1);
    expect(perspectiveFor(0, -30)).toBe(1);
    // In between it eases, with no jump.
    const steps = [0, 4, 8, 12, 16, 20, 24].map((off) => perspectiveFor(off, 0));
    for (let i = 1; i < steps.length; i += 1) expect(steps[i]).toBeGreaterThan(steps[i - 1] ?? 0);
    expect(steps.at(-1)).toBe(1);
    expect(perspectiveFor(1, 0)).toBeLessThan(0.01);
  });

  it('turn the short way round', () => {
    expect(shortWayRound(350, 0)).toBe(360);
    expect(shortWayRound(10, -90)).toBe(-90);
    expect(shortWayRound(-170, 170)).toBe(-190);
  });
});

describe('framing', () => {
  const bounds = { low: [-40, 0, -90] as const, high: [60, 460, 40] as const };

  it('fits the spine to the canvas by its height, or by its footprint in a narrow canvas', () => {
    expect(fittedScale(bounds, { width: 900, height: 700 })).toBeCloseTo(700 / (1.24 * 460), 12);
    expect(fittedScale(bounds, { width: 120, height: 700 })).toBeCloseTo(120 / (1.4 * 130), 12);
  });

  it('aims a named view at the middle of the spine, in the middle of the canvas', () => {
    const canvas = { width: 900, height: 700 };
    const view = namedView(bounds, canvas, 'three-quarter');
    expect(view.aim).toEqual(centreOf(bounds));
    expect(view.aim).toEqual([10, 230, -25]);
    const drawn = toCanvas(view, canvas, view.aim);
    expect([drawn.x, drawn.y]).toEqual([450, 350]);
    expect(view.pxPerMm[0]).toBeCloseTo(1.02 * fittedScale(bounds, canvas), 12);
  });

  it('keeps the whole spine inside the canvas in every named view', () => {
    const canvas = { width: 640, height: 720 };
    for (const name of ['front', 'side', 'three-quarter'] as const) {
      const view = namedView(bounds, canvas, name);
      for (const x of [bounds.low[0], bounds.high[0]]) {
        for (const y of [bounds.low[1], bounds.high[1]]) {
          for (const z of [bounds.low[2], bounds.high[2]]) {
            const drawn = toCanvas(view, canvas, [x, y, z]);
            expect(drawn.x, name).toBeGreaterThan(0);
            expect(drawn.x, name).toBeLessThan(canvas.width);
            expect(drawn.y, name).toBeGreaterThan(0);
            expect(drawn.y, name).toBeLessThan(canvas.height);
          }
        }
      }
    }
  });
});
