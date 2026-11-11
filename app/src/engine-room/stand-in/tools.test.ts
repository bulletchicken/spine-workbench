import type { FilmFacts, FilmKind, FilmMarks, Level, Mark, Px, VertebraOnFilm } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { EngineFailure } from '../engine-client';
import { boundaryWand } from './boundary-wand';
import { resolvePins } from './pins';
import { regionWand } from './region-wand';
import adjust from './scripts/adjust';
import type { ToolFilm } from './tool-film';
import { previewTool } from './tool-preview';

// Three upright rectangles, 60 px tall and 40 wide, with 20 px discs:
// T12 from y 40 to 100, L1 from 120 to 180, L2 from 200 to 260, x from
// 30 (the b-side) to 70 (the a-side). Every answer below is worked out
// by hand from these numbers and the default adjust script.

const mark = (at: Px, pinned = false): Mark => ({ at, source: 'auto', pinned, inferred: false });
const LEVELS: Level[] = ['T12', 'L1', 'L2'];

function rectangle(id: number, top: number, kind: FilmKind): VertebraOnFilm {
  const supA = mark([70, top]);
  const supB = mark([30, top]);
  const infB = mark([30, top + 60]);
  const infA = mark([70, top + 60]);
  const confidence = { score: 0.9, band: 'high' as const, reasons: ['odd-height' as const], calibrated: false, endplateSpreadPx: null, margin: null };
  const level = LEVELS[id - 10] as Level;
  if (kind === 'side') return { id, level, marks: { kind: 'side', points: { supA, supP: supB, infP: infB, infA } }, confidence, outline: null };
  if (kind !== 'front') return { id, level, marks: { kind: 'bending', lines: { sup: [supA, supB], inf: [infA, infB] } }, confidence, outline: null };
  return {
    id,
    level,
    marks: {
      kind: 'front',
      points: {
        supL: supA,
        supR: supB,
        infR: infB,
        infL: infA,
        pedL: { seen: 'visible', at: [62, top + 15], source: 'auto', pinned: false, inferred: false, ringScore: 0.8 },
        pedR: { seen: 'visible', at: [38, top + 15], source: 'auto', pinned: false, inferred: false, ringScore: 0.8 },
      },
    },
    confidence,
    outline: null,
  };
}

function filmMarks(kind: FilmKind = 'front', l5ConfirmedOn: number | null = null): FilmMarks {
  return {
    film: kind,
    vertebrae: [rectangle(10, 40, kind), rectangle(11, 120, kind), rectangle(12, 200, kind)],
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: [...LEVELS], fromBottom: null, flags: [], l5ConfirmedOn },
    namedFor: 'patient-left',
  };
}

const film = (kind: FilmKind = 'front'): ToolFilm => ({ kind, facts: { width: 100, height: 300 } as FilmFacts });
const corners = (vertebra: VertebraOnFilm | undefined) => {
  if (vertebra?.marks.kind !== 'front') throw new Error('not a front vertebra');
  const { supL, supR, infR, infL } = vertebra.marks.points;
  return [supL.at, supR.at, infR.at, infL.at];
};
const code = (call: () => unknown) => {
  try {
    call();
  } catch (thrown) {
    if (thrown instanceof EngineFailure) return `${thrown.error.code} ${String(thrown.error.detail)}`;
    throw thrown;
  }
  return 'answered';
};

describe('boundary wand preview', () => {
  const at = (point: Px, kind: FilmKind = 'front') => previewTool(film(kind), { caseVersion: 4, tool: 'boundary-wand', at: point, marks: filmMarks(kind), newVertebraId: 20 }, adjust);

  it('splits a vertebra across the click, renaming every vertebra below', () => {
    // Half way down L1: the line runs across at y = 120 + 0.5 · 60 = 150.
    expect(at([50, 150])).toEqual({
      action: 'split',
      caseVersion: 4,
      vertebra: 11,
      line: [
        [70, 150],
        [30, 150],
      ],
      levelsAfter: [
        { vertebra: 20, level: 'L2' },
        { vertebra: 12, level: 'L3' },
      ],
    });
  });

  it('merges the two vertebrae on either side of a gap, renaming those below', () => {
    expect(at([50, 110])).toEqual({ action: 'merge', caseVersion: 4, upper: 10, lower: 11, levelsAfter: [{ vertebra: 12, level: 'L1' }] });
  });

  it('merges from a click just outside the gap, within an eighth of the height', () => {
    // The gap between T12 and L1 is the box from y 100 to 120; 0.12 · 60 = 7.2 px of reach.
    expect(at([76, 110])).toMatchObject({ action: 'merge', upper: 10 });
    expect(at([78, 110])).toMatchObject({ action: 'none', reason: 'off-spine' });
  });

  it.each([
    [[50, 132] as Px, 'near-endplate', 'a fifth of the way down L1'],
    [[50, 168] as Px, 'near-endplate', 'four fifths of the way down L1'],
    [[50, 20] as Px, 'outside-chain', 'above the first vertebra'],
    [[50, 280] as Px, 'outside-chain', 'below the last'],
    [[10, 150] as Px, 'off-spine', 'beside L1'],
    [[150, 150] as Px, 'outside-film', 'off the film'],
  ])('answers none at %j: %s (%s)', (point, reason) => {
    expect(at(point)).toEqual({ action: 'none', caseVersion: 4, reason });
  });

  it('splits a quarter of the height or more from either endplate', () => {
    // L1 runs from y 120 to 180: a quarter of its 60 px is 15, so 135 and 165 are the last places a split is offered.
    expect(at([50, 136])).toMatchObject({ action: 'split', vertebra: 11 });
    expect(at([50, 164])).toMatchObject({ action: 'split', vertebra: 11 });
    expect(at([50, 134])).toMatchObject({ action: 'none', reason: 'near-endplate' });
    expect(at([50, 166])).toMatchObject({ action: 'none', reason: 'near-endplate' });
  });

  it('is not offered on a bending film', () => {
    expect(at([50, 150], 'bend-left')).toEqual({ action: 'none', caseVersion: 4, reason: 'not-supported-on-film' });
  });
});

describe('boundary wand', () => {
  it('splits: the upper piece keeps its id, the lower takes the new one, with a disc between', () => {
    const answer = boundaryWand(film(), { caseVersion: 4, marks: filmMarks(), at: [50, 150], newVertebraId: 20 }, adjust);
    expect(answer.action).toBe('split');
    expect(answer.caseVersion).toBe(4);
    expect(answer.newVertebra).toBe(20);
    const [t12, upper, lower, last] = answer.marks.vertebrae;
    expect(answer.marks.vertebrae.map((vertebra) => [vertebra.id, vertebra.level])).toEqual([
      [10, 'T12'],
      [11, 'L1'],
      [20, 'L2'],
      [12, 'L3'],
    ]);
    // A disc a tenth of 60 px tall, centred on y 150: the pieces end at 147 and start at 153.
    expect(corners(upper)).toEqual([
      [70, 120],
      [30, 120],
      [30, 147],
      [70, 147],
    ]);
    expect(corners(lower)).toEqual([
      [70, 153],
      [30, 153],
      [30, 180],
      [70, 180],
    ]);
    // L1's pedicles (y 135) stay on the upper piece; the lower piece's are inferred at
    // 0.11 and 0.89 across from its a-side and 0.32 of its 27 px down: (65.6, 161.64) and (34.4, 161.64).
    if (lower?.marks.kind !== 'front') throw new Error('not a front vertebra');
    expect(lower.marks.points.pedL).toMatchObject({ seen: 'visible', inferred: true, source: 'boundary-wand' });
    if (lower.marks.points.pedL.seen !== 'visible' || lower.marks.points.pedR.seen !== 'visible') throw new Error('pedicles not placed');
    expect(lower.marks.points.pedL.at[0]).toBeCloseTo(65.6, 9);
    expect(lower.marks.points.pedL.at[1]).toBeCloseTo(161.64, 9);
    expect(lower.marks.points.pedR.at[0]).toBeCloseTo(34.4, 9);
    expect(upper?.marks.kind === 'front' ? upper.marks.points.pedL : null).toMatchObject({ at: [62, 135], inferred: false });
    // Only the two pieces were aimed at; the split resolves their odd height.
    expect(upper?.confidence.reasons).toEqual(['edited-by-user']);
    expect(lower.confidence.reasons).toEqual(['edited-by-user']);
    expect(t12?.confidence.reasons).toEqual(['odd-height']);
    expect(last?.confidence.reasons).toEqual(['odd-height']);
    expect(answer.changed).toEqual([11, 20, 12]);
    expect(answer.marks.naming.fromTop).toEqual(['T12', 'L1', 'L2', 'L3']);
    expect(answer.marks.naming.fromBottom).toBeNull();
  });

  it('merges: the upper id stays, the lower is removed, and names below move up', () => {
    const answer = boundaryWand(film(), { caseVersion: 4, marks: filmMarks(), at: [50, 110], newVertebraId: 20 }, adjust);
    expect(answer).toMatchObject({ action: 'merge', removedVertebra: 11, newVertebra: null, changed: [10, 12], warnings: [] });
    expect(answer.marks.vertebrae.map((vertebra) => [vertebra.id, vertebra.level])).toEqual([
      [10, 'T12'],
      [12, 'L1'],
    ]);
    expect(corners(answer.marks.vertebrae[0])).toEqual([
      [70, 40],
      [30, 40],
      [30, 180],
      [70, 180],
    ]);
    expect(answer.marks.vertebrae[0]?.confidence.reasons).toEqual(['odd-height', 'edited-by-user']);
  });

  it('clears an L5 confirmation whose vertebra loses the name, and says so', () => {
    const marks = filmMarks('front', 12);
    const named: FilmMarks = { ...marks, vertebrae: marks.vertebrae.map((vertebra, index) => ({ ...vertebra, level: (['L3', 'L4', 'L5'] as Level[])[index] as Level })) };
    const answer = boundaryWand(film(), { caseVersion: 4, marks: named, at: [50, 150], newVertebraId: 20 }, adjust);
    expect(answer.marks.naming.l5ConfirmedOn).toBeNull();
    expect(answer.warnings).toEqual([{ code: 'naming-conflict' }]);
  });

  it('refuses where its preview says none', () => {
    const at = (point: Px) => code(() => boundaryWand(film(), { caseVersion: 4, marks: filmMarks(), at: point, newVertebraId: 20 }, adjust));
    expect(at([50, 132])).toBe('tool-refused near-endplate');
    expect(at([150, 150])).toBe('tool-click-outside-film outside-film');
  });

  it('works on a side film the same way', () => {
    const answer = boundaryWand(film('side'), { caseVersion: 1, marks: filmMarks('side'), at: [50, 150], newVertebraId: 20 }, adjust);
    expect(answer.marks.vertebrae.map((vertebra) => vertebra.id)).toEqual([10, 11, 20, 12]);
  });
});

describe('region wand', () => {
  const click = (at: Px, polarity: 'add' | 'subtract', vertebra = 11, kind: FilmKind = 'front') =>
    regionWand(film(kind), { caseVersion: 9, marks: filmMarks(kind), vertebra, clicks: [{ at, polarity }] }, adjust);

  it('moves the nearest wall out to an added click', () => {
    // 5 px right of L1's a-side wall (x 70): that wall moves to x 75.
    const answer = click([75, 150], 'add');
    expect(corners(answer.marks.vertebrae[1])).toEqual([
      [75, 120],
      [30, 120],
      [30, 180],
      [75, 180],
    ]);
    expect(answer.outline).toEqual([
      [75, 120],
      [30, 120],
      [30, 180],
      [75, 180],
    ]);
    expect(answer).toMatchObject({ caseVersion: 9, changed: [11] });
    expect(answer.marks.vertebrae[1]?.confidence.reasons).toEqual(['odd-height', 'edited-by-user']);
    expect(answer.marks.vertebrae[0]?.confidence.reasons).toEqual(['odd-height']);
  });

  it('moves the nearest wall in to a click that takes away', () => {
    // 4 px inside L1's upper endplate (y 120): it moves down to y 124.
    const answer = click([50, 124], 'subtract');
    expect(corners(answer.marks.vertebrae[1])).toEqual([
      [70, 124],
      [30, 124],
      [30, 180],
      [70, 180],
    ]);
  });

  it('leaves a pinned corner where it is', () => {
    const marks = filmMarks();
    const l1 = marks.vertebrae[1];
    if (l1?.marks.kind !== 'front') throw new Error('not a front vertebra');
    const pinned: FilmMarks = { ...marks, vertebrae: [marks.vertebrae[0] as VertebraOnFilm, { ...l1, marks: { kind: 'front', points: { ...l1.marks.points, supL: mark([70, 120], true) } } }, marks.vertebrae[2] as VertebraOnFilm] };
    const answer = regionWand(film(), { caseVersion: 9, marks: pinned, vertebra: 11, clicks: [{ at: [75, 150], polarity: 'add' }] }, adjust);
    expect(corners(answer.marks.vertebrae[1])).toEqual([
      [70, 120],
      [30, 120],
      [30, 180],
      [75, 180],
    ]);
  });

  it('previews add outside an outline, subtract inside, and none where it cannot reach', () => {
    const hover = (at: Px, kind: FilmKind = 'front') => previewTool(film(kind), { caseVersion: 2, tool: 'region-wand', at, marks: filmMarks(kind), newVertebraId: 20 }, adjust);
    expect(hover([80, 150])).toEqual({ action: 'add', caseVersion: 2, vertebra: 11 });
    expect(hover([50, 150])).toEqual({ action: 'subtract', caseVersion: 2, vertebra: 11 });
    // Half way across the disc between T12 and L1 the two are equally near.
    expect(hover([50, 110])).toEqual({ action: 'none', caseVersion: 2, reason: 'ambiguous' });
    // 0.6 · 60 = 36 px of reach: x 99 is 29 px from L1's a-side wall and over 36 from the others.
    expect(hover([99, 150])).toEqual({ action: 'add', caseVersion: 2, vertebra: 11 });
    expect(hover([50, 150], 'bend-right')).toEqual({ action: 'none', caseVersion: 2, reason: 'not-supported-on-film' });
  });

  it('refuses on a bending film and for a vertebra the film does not hold', () => {
    expect(code(() => click([75, 150], 'add', 11, 'bend-left'))).toBe('tool-refused not-supported-on-film');
    expect(code(() => click([75, 150], 'add', 99))).toBe('tool-unknown-vertebra 99');
  });
});

describe('pins', () => {
  it('holds a dragged corner where it is put, and the corner across the disc gives a quarter', () => {
    // L1's supL from (70, 120) to (80, 124): T12's infL, across the disc, moves (2.5, 1) to (72.5, 101).
    const answer = resolvePins(film(), { caseVersion: 3, marks: filmMarks(), action: { action: 'move', target: { on: 'vertebra', vertebra: 11, mark: 'supL' }, to: [80, 124] } }, adjust);
    const [t12, l1] = answer.marks.vertebrae;
    if (l1?.marks.kind !== 'front' || t12?.marks.kind !== 'front') throw new Error('not front vertebrae');
    expect(l1.marks.points.supL).toEqual({ at: [80, 124], source: 'pin', pinned: true, inferred: false });
    expect(t12.marks.points.infL).toEqual({ at: [72.5, 101], source: 'pin', pinned: false, inferred: false });
    expect(answer.changed).toEqual([11, 10]);
    expect(l1.confidence.reasons).toEqual(['odd-height', 'edited-by-user']);
    expect(t12.confidence.reasons).toEqual(['odd-height']);
  });

  it('unpins a point where it is', () => {
    const pinned = resolvePins(film(), { caseVersion: 3, marks: filmMarks(), action: { action: 'move', target: { on: 'vertebra', vertebra: 12, mark: 'infR' }, to: [28, 262] } }, adjust);
    const answer = resolvePins(film(), { caseVersion: 4, marks: pinned.marks, action: { action: 'unpin', target: { on: 'vertebra', vertebra: 12, mark: 'infR' } } }, adjust);
    const l2 = answer.marks.vertebrae[2];
    if (l2?.marks.kind !== 'front') throw new Error('not a front vertebra');
    expect(l2.marks.points.infR).toEqual({ at: [28, 262], source: 'pin', pinned: false, inferred: false });
  });

  it('moves the end of an endplate line on a bending film', () => {
    const answer = resolvePins(film('bend-left'), { caseVersion: 3, marks: filmMarks('bend-left'), action: { action: 'move', target: { on: 'endplate', vertebra: 12, line: 'inf', end: 'patient-right' }, to: [31, 258] } }, adjust);
    const l2 = answer.marks.vertebrae[2];
    if (l2?.marks.kind !== 'bending') throw new Error('not a bending vertebra');
    expect(l2.marks.lines.inf[1]).toEqual({ at: [31, 258], source: 'pin', pinned: true, inferred: false });
    expect(answer.changed).toEqual([12]);
  });

  it('refuses a point the film does not have', () => {
    expect(code(() => resolvePins(film(), { caseVersion: 3, marks: filmMarks(), action: { action: 'move', target: { on: 'vertebra', vertebra: 11, mark: 'supA' }, to: [1, 1] } }, adjust))).toMatch(/^tool-pin-invalid/);
    expect(code(() => resolvePins(film(), { caseVersion: 3, marks: filmMarks(), action: { action: 'move', target: { on: 'landmark', landmark: 'sacL' }, to: [1, 1] } }, adjust))).toMatch(/^tool-pin-invalid/);
    expect(code(() => resolvePins(film(), { caseVersion: 3, marks: filmMarks(), action: { action: 'move', target: { on: 'vertebra', vertebra: 11, mark: 'supL' }, to: [500, 1] } }, adjust))).toMatch(/^tool-click-outside-film/);
  });

  it('works C7 out again when a C7 corner moves', () => {
    const marks = filmMarks();
    const withC7: FilmMarks = { ...marks, vertebrae: marks.vertebrae.map((vertebra, index) => ({ ...vertebra, level: (['C7', 'T1', 'T2'] as Level[])[index] as Level })) };
    const answer = resolvePins(film(), { caseVersion: 3, marks: withC7, action: { action: 'move', target: { on: 'vertebra', vertebra: 10, mark: 'supR' }, to: [26, 36] } }, adjust);
    // The mean of (70, 40), (26, 36), (30, 100) and (70, 100).
    expect(answer.marks.landmarks.c7).toEqual({ at: [49, 69], source: 'auto', pinned: false, inferred: false });
  });
});
