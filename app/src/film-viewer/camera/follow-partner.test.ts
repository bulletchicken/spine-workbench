import type { FilmKind, FilmMarks, Level, Mark, Px, VertebraMarks } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { filmSpace } from '../film-space';
import { fit, lookSpot, travelTo, zoomOverFit, type CameraFrame } from './camera';
import { followPartner, vertebraCentre } from './follow-partner';

// Made-up marks: seventeen vertebrae, T1 to L5, on films of different
// sizes, scales and turns. Each film's centres are written down here
// directly, so the expected answer never comes from the code under test.

const LEVELS: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];

const viewport = { width: 340, height: 740 };
const margins = { top: 12, right: 8, bottom: 12, left: 34 };

function frameFor(width: number, height: number, quarterTurns: 0 | 1 | 2 | 3 = 0): CameraFrame {
  return { size: { width, height }, view: { quarterTurns, mirrored: false }, viewport, margins };
}

const front = frameFor(3000, 7000);
const side = frameFor(2600, 6600);
/** Stored upside down, as the sample case's bending-left film arrives. */
const bendLeft = frameFor(2800, 6900, 2);

/** Upright centres on the front film: uneven heights and a curve, as a spine has. */
const frontCentres: Px[] = LEVELS.map((_, i) => [1500 + 220 * Math.sin(i / 3), 1300 + i * 260 + 4 * i * i]);
/** On the side film: another scale, another offset, its own sway. */
const sideCentres: Px[] = LEVELS.map((_, i) => [1250 + 140 * Math.cos(i / 4), 1150 + i * 250 + 3 * i * i]);
/** On the bending film, upright; stored turned half a turn. */
const bendUpright: Px[] = LEVELS.map((_, i) => [1400 + 300 * Math.sin(i / 2.5), 1250 + i * 255 + 4 * i * i]);
const bendStored: Px[] = bendUpright.map(([x, y]) => [2800 - 1 - x, 6900 - 1 - y]);

const mark = (at: Px): Mark => ({ at, source: 'auto', pinned: false, inferred: false });

function boxAround([x, y]: Px, kind: FilmKind): VertebraMarks {
  const [w, h] = [180, 120];
  if (kind === 'side') {
    return {
      kind: 'side',
      points: { supA: mark([x + w / 2, y - h / 2]), supP: mark([x - w / 2, y - h / 2]), infA: mark([x + w / 2, y + h / 2]), infP: mark([x - w / 2, y + h / 2]) },
    };
  }
  if (kind === 'front') {
    return {
      kind: 'front',
      points: {
        supL: mark([x + w / 2, y - h / 2 + 8]),
        supR: mark([x - w / 2, y - h / 2 - 8]),
        infL: mark([x + w / 2, y + h / 2 + 8]),
        infR: mark([x - w / 2, y + h / 2 - 8]),
        pedL: { seen: 'not-visible', source: 'auto' },
        pedR: { seen: 'not-visible', source: 'auto' },
      },
    };
  }
  return {
    kind: 'bending',
    lines: { sup: [mark([x - w / 2, y - h / 2]), mark([x + w / 2, y - h / 2])], inf: [mark([x - w / 2, y + h / 2]), mark([x + w / 2, y + h / 2])] },
  };
}

function marksOn(film: FilmKind, centres: Px[], levels: Level[] = LEVELS): FilmMarks {
  return {
    film,
    vertebrae: levels.map((level) => {
      const centre = centres[LEVELS.indexOf(level)];
      if (centre === undefined) throw new Error(`no centre for ${level}`);
      return {
        id: LEVELS.indexOf(level),
        level,
        marks: boxAround(centre, film),
        confidence: { score: 0.95, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
        outline: null,
      };
    }),
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
    namedFor: film === 'side' ? 'anterior' : 'patient-left',
  };
}

const frontMarks = marksOn('front', frontCentres);
const sideMarks = marksOn('side', sideCentres);
const bendMarks = marksOn('bend-left', bendStored);

function at(i: number, centres: Px[]): Px {
  const centre = centres[i];
  if (centre === undefined) throw new Error(`no centre ${String(i)}`);
  return centre;
}

describe('follow the partner', () => {
  it('finds the middle of a vertebra from its marks', () => {
    expect(vertebraCentre(frontMarks.vertebrae[4] ?? fail())).toEqual(at(4, frontCentres));
    expect(vertebraCentre(sideMarks.vertebrae[9] ?? fail())).toEqual(at(9, sideCentres));
    expect(vertebraCentre(bendMarks.vertebrae[0] ?? fail())).toEqual(at(0, bendStored));
  });

  it('puts every level at the same height on Side as on Front, within one screen pixel', () => {
    // Levels whose travel the film's edges do not stop, at four times the fit.
    for (let i = 1; i <= 14; i += 1) {
      const placement = travelTo(front, at(i, frontCentres), 4);
      const sourceY = filmSpace(placement).toScreen(at(i, frontCentres))[1];
      expect(sourceY).toBeCloseTo(lookSpot(viewport)[1], 6);
      const followed = followPartner({ frame: front, placement, marks: frontMarks }, { frame: side, marks: sideMarks });
      const partnerY = filmSpace(followed).toScreen(at(i, sideCentres))[1];
      expect(Math.abs(partnerY - sourceY)).toBeLessThanOrEqual(1);
      expect(zoomOverFit(side, followed)).toBeCloseTo(4, 9);
    }
  });

  it('does the same for a film shown turned upright', () => {
    for (const i of [2, 8, 14]) {
      const placement = travelTo(front, at(i, frontCentres), 4);
      const sourceY = filmSpace(placement).toScreen(at(i, frontCentres))[1];
      const followed = followPartner({ frame: front, placement, marks: frontMarks }, { frame: bendLeft, marks: bendMarks });
      const [x, y] = filmSpace(followed).toScreen(at(i, bendStored));
      expect(Math.abs(y - sourceY)).toBeLessThanOrEqual(1);
      // The partner's vertebra sits across the middle, where its own camera would travel.
      expect(Math.abs(x - viewport.width / 2)).toBeLessThanOrEqual(1);
    }
  });

  it('works both ways: Side leads and Front follows', () => {
    const placement = travelTo(side, at(10, sideCentres), 4);
    const sourceY = filmSpace(placement).toScreen(at(10, sideCentres))[1];
    const followed = followPartner({ frame: side, placement, marks: sideMarks }, { frame: front, marks: frontMarks });
    expect(Math.abs(filmSpace(followed).toScreen(at(10, frontCentres))[1] - sourceY)).toBeLessThanOrEqual(1);
  });

  it('between two levels, shows the same fraction of the way between them', () => {
    // Halfway between T5 and T6 on Front shows halfway between T5 and T6 on Side.
    const t5 = at(4, frontCentres);
    const t6 = at(5, frontCentres);
    const halfway: Px = [(t5[0] + t6[0]) / 2, (t5[1] + t6[1]) / 2];
    const placement = travelTo(front, halfway, 2);
    const followed = followPartner({ frame: front, placement, marks: frontMarks }, { frame: side, marks: sideMarks });
    const s5 = at(4, sideCentres);
    const s6 = at(5, sideCentres);
    const spot = lookSpot(viewport);
    const [x, y] = filmSpace(followed).toScreen([(s5[0] + s6[0]) / 2, (s5[1] + s6[1]) / 2]);
    expect(Math.abs(x - spot[0])).toBeLessThanOrEqual(1e-6);
    expect(Math.abs(y - spot[1])).toBeLessThanOrEqual(1e-6);
  });

  it('matches levels by name, so a vertebra missing on one film does not shift the rest', () => {
    const sideWithoutT3 = marksOn(
      'side',
      sideCentres,
      LEVELS.filter((level) => level !== 'T3'),
    );
    const placement = travelTo(front, at(9, frontCentres), 2);
    const sourceY = filmSpace(placement).toScreen(at(9, frontCentres))[1];
    const followed = followPartner({ frame: front, placement, marks: frontMarks }, { frame: side, marks: sideWithoutT3 });
    expect(Math.abs(filmSpace(followed).toScreen(at(9, sideCentres))[1] - sourceY)).toBeLessThanOrEqual(1);
  });

  it('past the first and last shared levels, goes on at the films’ relative scale', () => {
    // Only T6 and T7 on both. Front: T6 at y 2000, T7 at 2400. Side: T6 at
    // 1500, T7 at 1700, so Side shows half as many pixels per level. 400
    // above T6 on Front is 200 above T6 on Side, at 1300; 400 below T7 is
    // 200 below it, at 1900. Across, the end level's own x on Side.
    const pair: Level[] = ['T6', 'T7'];
    const frontPair = marksOn('front', LEVELS.map((_, i): Px => [1500, 2000 + (i - 5) * 400]), pair);
    const sidePair = marksOn('side', LEVELS.map((_, i): Px => [1250, 1500 + (i - 5) * 200]), pair);
    const spot = lookSpot(viewport);
    for (const [onFront, onSide] of [
      [[1500, 1600], [1250, 1300]],
      [[1500, 2800], [1250, 1900]],
    ] as [Px, Px][]) {
      const placement = travelTo(front, onFront, 4);
      const followed = followPartner({ frame: front, placement, marks: frontPair }, { frame: side, marks: sidePair });
      const [x, y] = filmSpace(followed).toScreen(onSide);
      expect(Math.abs(x - spot[0])).toBeLessThanOrEqual(1e-6);
      expect(Math.abs(y - spot[1])).toBeLessThanOrEqual(1e-6);
    }
  });

  it('with one shared level, goes on from it at the films’ upright heights', () => {
    // Only T6 on both: Front's at (1500, 2000), Side's at (1250, 1500).
    // Front is 7000 high and Side 6600, so 400 above T6 on Front is
    // 400 × 6600 / 7000 = 377.142857… above it on Side, and 400 below is as
    // far below.
    const one: Level[] = ['T6'];
    const frontOne = marksOn('front', LEVELS.map((): Px => [1500, 2000]), one);
    const sideOne = marksOn('side', LEVELS.map((): Px => [1250, 1500]), one);
    const spot = lookSpot(viewport);
    for (const [onFront, onSide] of [
      [[1500, 1600], [1250, 1500 - 2640000 / 7000]],
      [[1500, 2400], [1250, 1500 + 2640000 / 7000]],
    ] as [Px, Px][]) {
      const placement = travelTo(front, onFront, 4);
      const followed = followPartner({ frame: front, placement, marks: frontOne }, { frame: side, marks: sideOne });
      const [x, y] = filmSpace(followed).toScreen(onSide);
      expect(Math.abs(x - spot[0])).toBeLessThanOrEqual(1e-6);
      expect(Math.abs(y - spot[1])).toBeLessThanOrEqual(1e-6);
    }
  });

  it('reads marks listed in any order, bottom to top as well as top to bottom', () => {
    // The same halfway point between T5 and T6 as above, from lists written L5 first.
    const upwards = (marks: FilmMarks): FilmMarks => ({ ...marks, vertebrae: [...marks.vertebrae].reverse() });
    const t5 = at(4, frontCentres);
    const t6 = at(5, frontCentres);
    const placement = travelTo(front, [(t5[0] + t6[0]) / 2, (t5[1] + t6[1]) / 2], 2);
    const followed = followPartner({ frame: front, placement, marks: upwards(frontMarks) }, { frame: side, marks: upwards(sideMarks) });
    const s5 = at(4, sideCentres);
    const s6 = at(5, sideCentres);
    const spot = lookSpot(viewport);
    const [x, y] = filmSpace(followed).toScreen([(s5[0] + s6[0]) / 2, (s5[1] + s6[1]) / 2]);
    expect(Math.abs(x - spot[0])).toBeLessThanOrEqual(1e-6);
    expect(Math.abs(y - spot[1])).toBeLessThanOrEqual(1e-6);
  });

  it('with no marks, shows the same fraction of the film’s height', () => {
    // The middle of Front (1499.5, 3499.5) is the middle of Side (1299.5, 3299.5).
    const placement = travelTo(front, [1499.5, 3499.5], 2);
    const followed = followPartner({ frame: front, placement, marks: null }, { frame: side, marks: null });
    const spot = lookSpot(viewport);
    const [x, y] = filmSpace(followed).toScreen([1299.5, 3299.5]);
    expect(Math.abs(x - spot[0])).toBeLessThanOrEqual(1e-6);
    expect(Math.abs(y - spot[1])).toBeLessThanOrEqual(1e-6);
  });

  it('with no marks, measures the fraction on the partner as it is shown, upright', () => {
    // (749.5, 1749.5) on Front is 750 and 1750 from its outer corner: a quarter across and a quarter down.
    // On the bending film, upright, that is 700 and 1725 from the corner, the pixel centred at (699.5, 1724.5);
    // stored half a turn round, that pixel is (2799 − 699.5, 6899 − 1724.5).
    const placement = travelTo(front, [749.5, 1749.5], 3);
    const followed = followPartner({ frame: front, placement, marks: null }, { frame: bendLeft, marks: null });
    const spot = lookSpot(viewport);
    const [x, y] = filmSpace(followed).toScreen([2099.5, 5174.5]);
    expect(Math.abs(x - spot[0])).toBeLessThanOrEqual(1e-6);
    expect(Math.abs(y - spot[1])).toBeLessThanOrEqual(1e-6);
  });

  it('fits the partner when the source is at its fit', () => {
    expect(followPartner({ frame: front, placement: fit(front), marks: frontMarks }, { frame: side, marks: sideMarks })).toEqual(fit(side));
  });
});

function fail(): never {
  throw new Error('missing fixture');
}
