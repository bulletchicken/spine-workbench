import type { FilmKind, FilmMarks, Level, Mark, Px } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { filmSpace, type FilmPlacement } from '../film-space';
import { fit, lookPoint, lookSpot, travelTo, type CameraFrame } from './camera';
import { makeCameras, type Curve } from './cameras';

const viewport = { width: 300, height: 500 };
const margins = { top: 0, right: 0, bottom: 0, left: 0 };
const front: CameraFrame = { size: { width: 1000, height: 2000 }, view: { quarterTurns: 0, mirrored: false }, viewport, margins };
const side: CameraFrame = { size: { width: 800, height: 1800 }, view: { quarterTurns: 0, mirrored: false }, viewport, margins };

const linear: Curve = (t) => t;
const easeInOut: Curve = (t) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);

function hung() {
  const cameras = makeCameras();
  cameras.hang('front', front);
  cameras.hang('side', side);
  return cameras;
}

function placementOf(cameras: ReturnType<typeof makeCameras>, film: FilmKind): FilmPlacement {
  const placement = cameras.placementOf(film);
  if (placement === null) throw new Error(`${film} is not hung`);
  return placement;
}

describe('cameras', () => {
  it('hangs a film at its fit', () => {
    const cameras = hung();
    expect(cameras.placementOf('front')).toEqual(fit(front));
    expect(cameras.zoomOverFitOf('front')).toBe(1);
    expect(cameras.placementOf('bend-left')).toBeNull();
  });

  it('lands a glide exactly on its target', () => {
    fc.assert(
      fc.property(fc.array(fc.double({ min: 1, max: 40, noNaN: true }), { minLength: 1, maxLength: 200 }), (frames) => {
        const cameras = hung();
        const target = travelTo(front, [500, 900], 3);
        cameras.travelTo('front', [500, 900], 3, true);
        let total = 0;
        for (const dt of frames) {
          cameras.step(dt, 700, easeInOut);
          total += dt;
        }
        // Then whatever time was left.
        if (total < 700) cameras.step(700 - total, 700, easeInOut);
        expect(cameras.placementOf('front')).toEqual(target);
        expect(cameras.isGliding('front')).toBe(false);
      }),
    );
  });

  it('is halfway at half the time on a straight curve', () => {
    const cameras = hung();
    const start = fit(front);
    const target = travelTo(front, [500, 900], 3);
    cameras.travelTo('front', [500, 900], 3, true);
    expect(cameras.step(350, 700, linear)).toBe(true);
    const halfway = placementOf(cameras, 'front');
    expect(halfway.zoom).toBeCloseTo((start.zoom + target.zoom) / 2, 12);
    expect(halfway.pan[0]).toBeCloseTo((start.pan[0] + target.pan[0]) / 2, 9);
    expect(halfway.pan[1]).toBeCloseTo((start.pan[1] + target.pan[1]) / 2, 9);
  });

  it('starts a second glide from wherever the first had got to, without a jump', () => {
    const cameras = hung();
    cameras.travelTo('front', [500, 400], 4, true);
    cameras.step(300, 700, easeInOut);
    const midway = placementOf(cameras, 'front');
    cameras.travelTo('front', [500, 1700], 2, true);
    expect(cameras.placementOf('front')).toEqual(midway);
    cameras.step(1, 700, easeInOut);
    const next = placementOf(cameras, 'front');
    // One millisecond into an eased glide moves a hair, not a jump.
    expect(Math.abs(next.zoom - midway.zoom)).toBeLessThan(1e-3 * midway.zoom);
    expect(Math.hypot(next.pan[0] - midway.pan[0], next.pan[1] - midway.pan[1])).toBeLessThan(0.5);
    cameras.step(699, 700, easeInOut);
    expect(cameras.placementOf('front')).toEqual(travelTo(front, [500, 1700], 2));
  });

  it('lands at once when asked to glide with no duration (reduced motion)', () => {
    const cameras = hung();
    cameras.travelTo('front', [500, 900], 3, true);
    expect(cameras.step(16, 0, easeInOut)).toBe(false);
    expect(cameras.placementOf('front')).toEqual(travelTo(front, [500, 900], 3));
  });

  it('lands pointer changes at once, and they stop a glide', () => {
    const cameras = hung();
    cameras.travelTo('front', [500, 900], 3, true);
    cameras.step(100, 700, linear);
    const before = placementOf(cameras, 'front');
    cameras.zoomAt('front', 1.5, [150, 250]);
    expect(cameras.isGliding('front')).toBe(false);
    expect(placementOf(cameras, 'front').zoom).toBeCloseTo(before.zoom * 1.5, 12);
    expect(cameras.step(16, 700, linear)).toBe(false);
  });

  it('tells listeners each move, and whether the film is still on its way', () => {
    const cameras = hung();
    const heard: [FilmKind, boolean][] = [];
    const stop = cameras.subscribe((film, moving) => heard.push([film, moving]));
    cameras.travelTo('front', [500, 900], 3, true);
    cameras.step(300, 700, linear);
    cameras.step(400, 700, linear);
    cameras.panBy('front', [10, 10]);
    stop();
    cameras.panBy('front', [10, 10]);
    expect(heard).toEqual([
      ['front', true],
      ['front', true],
      ['front', false],
      ['front', false],
    ]);
  });

  it('tells listeners a glide has started before any step, still at its start', () => {
    const cameras = hung();
    const heard: [FilmKind, boolean, FilmPlacement | null][] = [];
    cameras.subscribe((film, moving) => heard.push([film, moving, cameras.placementOf(film)]));
    cameras.fit('front', true);
    cameras.zoomBy('front', 2, true);
    expect(heard).toEqual([
      ['front', true, fit(front)],
      ['front', true, fit(front)],
    ]);
  });

  it('says a glide is under way when a listener starts one as another lands, and moves it on only from the next step', () => {
    const cameras = hung();
    const target = travelTo(side, [400, 900], 2);
    cameras.subscribe((film, moving) => {
      if (film === 'front' && !moving) cameras.travelTo('side', [400, 900], 2, true);
    });
    cameras.travelTo('front', [500, 900], 3, true);
    // Front lands in this step; Side's glide starts inside it, on a film later in the map.
    expect(cameras.step(700, 700, linear)).toBe(true);
    expect(cameras.isGliding('side')).toBe(true);
    expect(cameras.placementOf('side')).toEqual(fit(side));
    // Halfway in time on a straight curve is halfway in zoom.
    expect(cameras.step(350, 700, linear)).toBe(true);
    expect(placementOf(cameras, 'side').zoom).toBeCloseTo((fit(side).zoom + target.zoom) / 2, 12);
    expect(cameras.step(350, 700, linear)).toBe(false);
    expect(cameras.placementOf('side')).toEqual(target);
  });

  it('lets a glide a listener starts mid-step replace one that step had still to move on', () => {
    const cameras = hung();
    cameras.subscribe((film, moving) => {
      if (film === 'front' && !moving) cameras.travelTo('side', [200, 1500], 2, true);
    });
    cameras.travelTo('front', [500, 900], 3, true);
    cameras.travelTo('side', [400, 900], 2, true);
    expect(cameras.step(700, 700, linear)).toBe(true);
    // Side's first glide never moved: the new one starts where Side was, at its fit.
    expect(cameras.placementOf('side')).toEqual(fit(side));
    cameras.step(700, 700, linear);
    expect(cameras.placementOf('side')).toEqual(travelTo(side, [200, 1500], 2));
  });

  it('zooms with the keys from where a glide is headed, not from where it has got to (worked by hand)', () => {
    // Front's fit is 0.25 (300 / 1000 across, 500 / 2000 down). Travelling to
    // (500, 1000) at twice the fit: zoom 0.5, and that point, 500.5 and
    // 1000.5 from the outer corner, on the look spot (150, 235), so the pan
    // is (150 − 250.25, 235 − 500.25) = (−100.25, −265.25). Half again about
    // the middle (150, 250): zoom 0.75, pan (150 − 250.25 × 1.5, 250 − 515.25 × 1.5).
    const cameras = hung();
    cameras.travelTo('front', [500, 1000], 2, true);
    cameras.step(200, 700, linear);
    cameras.zoomBy('front', 1.5, true);
    cameras.step(700, 700, linear);
    const landed = placementOf(cameras, 'front');
    expect(landed.zoom).toBe(0.75);
    expect(landed.pan[0]).toBeCloseTo(-225.375, 9);
    expect(landed.pan[1]).toBeCloseTo(-522.875, 9);
  });

  it('keeps where it was looking when the viewport changes', () => {
    const cameras = hung();
    cameras.travelTo('front', [400, 1200], 2, false);
    const looking = lookPoint(front, placementOf(cameras, 'front'));
    const wider: CameraFrame = { ...front, viewport: { width: 420, height: 640 } };
    cameras.hang('front', wider);
    const now = placementOf(cameras, 'front');
    expect(cameras.zoomOverFitOf('front')).toBeCloseTo(2, 12);
    const [x, y] = lookPoint(wider, now);
    expect(x).toBeCloseTo(looking[0], 6);
    expect(y).toBeCloseTo(looking[1], 6);
  });

  it('refits a film that was at its fit when the viewport changes', () => {
    const cameras = hung();
    const smaller: CameraFrame = { ...front, viewport: { width: 200, height: 300 } };
    cameras.hang('front', smaller);
    expect(cameras.placementOf('front')).toEqual(fit(smaller));
  });

  describe('Front and its partner', () => {
    const levels: Level[] = ['T6', 'T7', 'T8', 'T9', 'T10'];
    const frontCentres: Px[] = [[500, 700], [510, 820], [525, 950], [530, 1090], [520, 1240]];
    const sideCentres: Px[] = [[400, 600], [405, 700], [415, 810], [420, 930], [415, 1060]];

    function linked() {
      const cameras = hung();
      cameras.link('side');
      cameras.setMarks('front', marks('front', frontCentres));
      cameras.setMarks('side', marks('side', sideCentres));
      return cameras;
    }

    function marks(film: FilmKind, centres: Px[]): FilmMarks {
      const mark = (at: Px): Mark => ({ at, source: 'auto', pinned: false, inferred: false });
      return {
        film,
        vertebrae: centres.map(([x, y], i) => ({
          id: i,
          level: levels[i] ?? 'T1',
          marks: {
            kind: 'side' as const,
            points: { supA: mark([x + 40, y - 30]), supP: mark([x - 40, y - 30]), infA: mark([x + 40, y + 30]), infP: mark([x - 40, y + 30]) },
          },
          confidence: { score: 1, band: 'high' as const, reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
          outline: null,
        })),
        landmarks: { c7: null, sacrumTop: null, hips: [] },
        naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
        namedFor: 'anterior',
      };
    }

    it('glides the partner to the same level as Front travels', () => {
      const cameras = linked();
      cameras.travelTo('front', frontCentres[2] ?? [0, 0], 3, true);
      expect(cameras.isGliding('side')).toBe(true);
      cameras.step(700, 700, easeInOut);
      // T8 sits on the look spot of both films.
      const spot = lookSpot(viewport);
      const onFront = filmSpace(placementOf(cameras, 'front')).toScreen(frontCentres[2] ?? [0, 0]);
      const onSide = filmSpace(placementOf(cameras, 'side')).toScreen(sideCentres[2] ?? [0, 0]);
      expect(onFront[1]).toBeCloseTo(spot[1], 6);
      expect(Math.abs(onSide[1] - onFront[1])).toBeLessThanOrEqual(1);
    });

    it('moves Front at once when its partner is dragged', () => {
      const cameras = linked();
      cameras.travelTo('side', sideCentres[1] ?? [0, 0], 3, false);
      cameras.panBy('side', [0, -40]);
      expect(cameras.isGliding('front')).toBe(false);
      const sideLook = lookPoint(side, placementOf(cameras, 'side'));
      const frontLook = lookPoint(front, placementOf(cameras, 'front'));
      // Side looks 40 / zoom film pixels below T7; Front looks the same fraction of the way to T8.
      const t = (sideLook[1] - (sideCentres[1]?.[1] ?? 0)) / ((sideCentres[2]?.[1] ?? 0) - (sideCentres[1]?.[1] ?? 0));
      expect(frontLook[1]).toBeCloseTo((frontCentres[1]?.[1] ?? 0) + t * ((frontCentres[2]?.[1] ?? 0) - (frontCentres[1]?.[1] ?? 0)), 6);
    });

    it('fits both together', () => {
      const cameras = linked();
      cameras.travelTo('front', [500, 900], 3, false);
      cameras.fit('front', false);
      expect(cameras.placementOf('front')).toEqual(fit(front));
      expect(cameras.placementOf('side')).toEqual(fit(side));
    });

    it('brings a newly linked film to the level Front shows, not its fit', () => {
      // The bending film is the same size as Front and has no marks, so the same level is the same placement.
      const cameras = hung();
      cameras.hang('bend-left', front);
      cameras.travelTo('front', [400, 1200], 3, false);
      cameras.link('bend-left');
      expect(placementOf(cameras, 'bend-left')).toEqual(placementOf(cameras, 'front'));
    });

    it('brings a film hung beside a zoomed Front to Front’s level', () => {
      const cameras = makeCameras();
      cameras.hang('front', front);
      cameras.link('bend-right');
      cameras.travelTo('front', [400, 1200], 3, false);
      cameras.hang('bend-right', front);
      expect(placementOf(cameras, 'bend-right')).toEqual(placementOf(cameras, 'front'));
    });

    it('glides a newly linked film to where a gliding Front is headed', () => {
      const cameras = hung();
      cameras.hang('bend-left', front);
      cameras.travelTo('front', [400, 1200], 3, true);
      cameras.step(200, 700, easeInOut);
      cameras.link('bend-left');
      expect(cameras.isGliding('bend-left')).toBe(true);
      cameras.step(700, 700, easeInOut);
      expect(placementOf(cameras, 'bend-left')).toEqual(travelTo(front, [400, 1200], 3));
    });

    it('leaves a film that is not linked alone', () => {
      const cameras = linked();
      cameras.hang('bend-right', side);
      cameras.travelTo('front', [500, 900], 3, false);
      expect(cameras.placementOf('bend-right')).toEqual(fit(side));
      cameras.link(null);
      const sideBefore = cameras.placementOf('side');
      cameras.travelTo('front', [500, 600], 2, false);
      expect(cameras.placementOf('side')).toEqual(sideBefore);
    });

    it('takes Front named as its own partner to mean no partner', () => {
      const cameras = linked();
      cameras.link('front');
      cameras.travelTo('front', [500, 1000], 3, false);
      const frontBefore = placementOf(cameras, 'front');
      const sideBefore = cameras.placementOf('side');
      const heard: FilmKind[] = [];
      cameras.subscribe((film) => heard.push(film));
      cameras.panBy('front', [10, 10]);
      // One move, heard once, landed where the drag put it, and nothing else moved.
      expect(heard).toEqual(['front']);
      expect(placementOf(cameras, 'front').pan).toEqual([frontBefore.pan[0] + 10, frontBefore.pan[1] + 10]);
      expect(cameras.placementOf('side')).toEqual(sideBefore);
    });
  });
});
