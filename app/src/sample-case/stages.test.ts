// The sample case against the frozen rules a hand-made case can break,
// written here from the contract (docs/contracts/case-file-v1.md, the
// strict reader in engine/shapes/src/reading.rs and the header of
// app/src/case-state/case-edits.ts), not from the code that makes it.

import type { CaseFileV1, CheckKind, FilmKind, FilmMarks, Level, Mark, Px, VertebraMarks } from '@/shapes';
import { CONFIDENCE_HIGH_FROM, CONFIDENCE_MEDIUM_FROM, LEVELS, TEMPLATE_LEVELS, TEMPLATE_NAMES } from '@/shapes';
import { describe, expect, expectTypeOf, it } from 'vitest';
import { caseAt, findingsAt, STORY_STAGES, type StoryStage } from './stages';

const cases = new Map<StoryStage, CaseFileV1>(STORY_STAGES.map((stage) => [stage, caseAt(stage).case]));
const caseOf = (stage: StoryStage) => cases.get(stage) ?? caseAt(stage).case;
const marksOf = (stage: StoryStage, film: FilmKind) => caseOf(stage).marks.find((marks) => marks.film === film);

/** Every corner of a vertebra (a bending film's line ends), in quad order, by its definition in the contract. */
function cornersOf(marks: VertebraMarks): Mark[] {
  if (marks.kind === 'front') return [marks.points.supL, marks.points.supR, marks.points.infR, marks.points.infL];
  if (marks.kind === 'side') return [marks.points.supA, marks.points.supP, marks.points.infP, marks.points.infA];
  return [marks.lines.sup[0], marks.lines.sup[1], marks.lines.inf[1], marks.lines.inf[0]];
}

const rank = (level: Level) => LEVELS.indexOf(level);
const cornersOfVertebra = (vertebra: { marks: VertebraMarks } | undefined) => (vertebra === undefined ? [] : cornersOf(vertebra.marks));

describe('every stage of the sample case', () => {
  it('is a CaseFileV1', () => {
    for (const stage of STORY_STAGES) expectTypeOf(caseAt(stage).case).toEqualTypeOf<CaseFileV1>();
  });

  it.each(STORY_STAGES)('%s keeps vertebra ids unique across the case and below nextVertebraId', (stage) => {
    const c = caseOf(stage);
    const ids = c.marks.flatMap((marks) => marks.vertebrae.map((vertebra) => vertebra.id));
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) expect(id).toBeLessThan(c.nextVertebraId);
  });

  it.each(STORY_STAGES)('%s runs levels in order from the top, each a name its count can give', (stage) => {
    for (const marks of caseOf(stage).marks) {
      const levels = marks.vertebrae.map((vertebra) => vertebra.level);
      for (let k = 1; k < levels.length; k++) expect(rank(levels[k] ?? 'C1')).toBeGreaterThan(rank(levels[k - 1] ?? 'C1'));
      for (const level of levels) {
        expect(TEMPLATE_NAMES[marks.naming.template]).toContain(level);
        if (!TEMPLATE_LEVELS[marks.naming.template].includes(level)) expect(marks.naming.flags).toContain('non-standard-count');
      }
      for (const count of [marks.naming.fromTop, marks.naming.fromBottom]) {
        if (count === null) continue;
        expect(count).toHaveLength(marks.vertebrae.length);
        for (let k = 1; k < count.length; k++) expect(rank(count[k] ?? 'C1')).toBeGreaterThan(rank(count[k - 1] ?? 'C1'));
      }
      if (marks.naming.l5ConfirmedOn !== null) {
        expect(marks.vertebrae.find((vertebra) => vertebra.id === marks.naming.l5ConfirmedOn)?.level).toBe('L5');
      }
    }
  });

  it.each(STORY_STAGES)('%s puts landmarks.c7 at the mean of the C7 corners, or nowhere without a C7', (stage) => {
    for (const marks of caseOf(stage).marks) {
      const c7 = marks.vertebrae.find((vertebra) => vertebra.level === 'C7');
      const landmark = marks.landmarks.c7;
      if (c7 === undefined) {
        expect(landmark).toBeNull();
        continue;
      }
      const corners = cornersOf(c7.marks);
      const mean = [0, 1].map((axis) => corners.reduce((sum, mark) => sum + (mark.at[axis] ?? 0), 0) / 4);
      expect(landmark).not.toBeNull();
      expect(Math.hypot((landmark?.at[0] ?? 0) - (mean[0] ?? 0), (landmark?.at[1] ?? 0) - (mean[1] ?? 0))).toBeLessThanOrEqual(0.001);
      expect(landmark?.source).toBe('auto');
      expect(landmark?.pinned).toBe(false);
      expect(landmark?.inferred).toBe(corners.some((mark) => mark.inferred));
    }
  });

  it.each(STORY_STAGES)('%s names each film\'s marks for its imageRight', (stage) => {
    const c = caseOf(stage);
    for (const marks of c.marks) expect(marks.namedFor).toBe(c.films.find((film) => film.kind === marks.film)?.geometry.imageRight);
  });

  it.each(STORY_STAGES)('%s gives pedicles to the front film only, and each film the marks of its view', (stage) => {
    const kindOf: Record<FilmKind, VertebraMarks['kind']> = { front: 'front', side: 'side', 'bend-left': 'bending', 'bend-right': 'bending' };
    for (const marks of caseOf(stage).marks) {
      for (const vertebra of marks.vertebrae) {
        expect(vertebra.marks.kind).toBe(kindOf[marks.film]);
        expect('pedL' in (vertebra.marks.kind === 'bending' ? vertebra.marks.lines : vertebra.marks.points)).toBe(marks.film === 'front');
      }
    }
  });

  it.each(STORY_STAGES)('%s keeps the other rules of the strict reader', (stage) => {
    const c = caseOf(stage);
    expect(new Set(c.films.map((film) => film.kind)).size).toBe(c.films.length);
    expect(new Set(c.marks.map((marks) => marks.film)).size).toBe(c.marks.length);
    for (const film of c.films) {
      expect(film.image.path).toBe(`films/${film.kind}.png`);
      expect(film.keptTags !== null).toBe(film.source === 'dicom');
      const g = film.geometry;
      expect(g.scale === null).toBe(g.mmPerPx === null);
      if (g.mmPerPx !== null) expect(g.mmPerPx >= 0.01 && g.mmPerPx <= 2).toBe(true);
      if (g.scale?.source === 'file') expect(film.keptTags).not.toBeNull();
      const frontView = film.kind !== 'side';
      if (g.confirmed.leftRight) expect(frontView && g.imageRight !== null).toBe(true);
      if (g.confirmed.facing) expect(!frontView && g.imageRight !== null).toBe(true);
      if (g.confirmed.scale) expect(g.mmPerPx).not.toBeNull();
      if (g.imageRight !== null) expect(['patient-left', 'patient-right'].includes(g.imageRight)).toBe(frontView);
    }
    for (const marks of c.marks) {
      const film = c.films.find((f) => f.kind === marks.film);
      const width = film?.image.width ?? 0;
      const height = film?.image.height ?? 0;
      const near = ([x, y]: [number, number]) => x >= -0.5 - width && x <= width - 0.5 + width && y >= -0.5 - height && y <= height - 0.5 + height;
      for (const vertebra of marks.vertebrae) {
        for (const corner of cornersOf(vertebra.marks)) expect(near(corner.at)).toBe(true);
        const { score, band, reasons } = vertebra.confidence;
        const earned = score >= CONFIDENCE_HIGH_FROM ? 2 : score >= CONFIDENCE_MEDIUM_FROM ? 1 : 0;
        expect(['low', 'medium', 'high'].indexOf(band)).toBeLessThanOrEqual(earned);
        expect(new Set(reasons).size).toBe(reasons.length);
        if (vertebra.outline !== null) expect(vertebra.outline.length).toBeGreaterThanOrEqual(3);
      }
      const hips = marks.landmarks.hips.map((hip) => hip.side);
      if (marks.film === 'front') expect(hips.filter((side) => side === 'left').length).toBeLessThanOrEqual(1);
      if (marks.film === 'side') expect(hips.every((side) => side === 'unpaired')).toBe(true);
      if (marks.film === 'bend-left' || marks.film === 'bend-right') expect(hips).toEqual([]);
      expect(marks.landmarks.sacrumTop?.view).toBe(marks.film === 'side' ? 'side' : 'front');
    }
  });

  it('comes out afresh on every call', () => {
    const first = caseAt('pass-result').case;
    first.marks.length = 0;
    expect(caseAt('pass-result').case.marks).toHaveLength(4);
  });
});

describe('which side each corner is on', () => {
  /** A stored film's point where it shows once the film is turned upright by its quarter turns, clockwise (docs/conventions/pixel-space.md). */
  function upright([x, y]: Px, turns: number, width: number, height: number): Px {
    switch (turns % 4) {
      case 1:
        return [height - 1 - y, x];
      case 2:
        return [width - 1 - x, height - 1 - y];
      case 3:
        return [y, width - 1 - x];
      default:
        return [x, y];
    }
  }

  /** Which way c turns from a to b: positive, negative or zero when the three are in a line. */
  const turn = (a: Px, b: Px, c: Px) => Math.sign((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]));
  /** Whether segments ab and cd cross at a point inside both. */
  const cross = (a: Px, b: Px, c: Px, d: Px) => turn(a, b, c) * turn(a, b, d) < 0 && turn(c, d, a) * turn(c, d, b) < 0;
  const at = (points: Px[], k: number): Px => points[((k % points.length) + points.length) % points.length] ?? [0, 0];

  /** True when no two edges of the closed polygon that do not share a corner cross. */
  function simple(points: Px[]): boolean {
    const n = points.length;
    for (let i = 0; i < n; i++) {
      for (let j = i + 2; j < n; j++) {
        if (i === 0 && j === n - 1) continue;
        if (cross(at(points, i), at(points, i + 1), at(points, j), at(points, j + 1))) return false;
      }
    }
    return true;
  }

  /** Twice the polygon's signed area, by the shoelace formula. */
  const twiceArea = (points: Px[]) => points.reduce((sum, p, k) => sum + p[0] * at(points, k + 1)[1] - at(points, k + 1)[0] * p[1], 0);

  /** The corners the contract names patient left (or anterior), upper then lower, and their partners on the other side. */
  function pairs(marks: VertebraMarks): [Mark, Mark][] {
    if (marks.kind === 'front') return [[marks.points.supL, marks.points.supR], [marks.points.infL, marks.points.infR]];
    if (marks.kind === 'side') return [[marks.points.supA, marks.points.supP], [marks.points.infA, marks.points.infP]];
    return [[marks.lines.sup[0], marks.lines.sup[1]], [marks.lines.inf[0], marks.lines.inf[1]]];
  }

  it('turns a stored point upright as the convention does: the top-left pixel to each corner in turn', () => {
    expect(upright([0, 0], 1, 720, 1720)).toEqual([1719, 0]);
    expect(upright([0, 0], 2, 720, 1720)).toEqual([719, 1719]);
    expect(upright([0, 0], 3, 720, 1720)).toEqual([0, 719]);
  });

  it.each(['pass-result', 'l5-set', 'after-split', 't3-fixed', 'all-clear'] as const)(
    '%s: every outline is a simple quad wound as its neighbours, its left (or anterior) corners on the side imageRight names',
    (stage) => {
      const c = caseOf(stage);
      for (const marks of c.marks) {
        const film = c.films.find((f) => f.kind === marks.film);
        if (film === undefined) throw new Error(`no film for ${marks.film}`);
        const { quarterTurns, imageRight } = film.geometry;
        expect(imageRight).not.toBeNull();
        const onScreen = (point: Px) => upright(point, quarterTurns, film.image.width, film.image.height);
        // imageRight names the patient direction on screen right once the film is upright.
        const namedOnTheRight = imageRight === 'patient-left' || imageRight === 'anterior';
        const windings = new Set<number>();
        for (const vertebra of marks.vertebrae) {
          const quad = cornersOf(vertebra.marks).map((mark) => onScreen(mark.at));
          expect(simple(quad), `${marks.film} ${vertebra.level} crosses itself`).toBe(true);
          expect(Math.abs(twiceArea(quad))).toBeGreaterThan(0);
          windings.add(Math.sign(twiceArea(quad)));
          for (const [named, partner] of pairs(vertebra.marks)) {
            expect(onScreen(named.at)[0] > onScreen(partner.at)[0], `${marks.film} ${vertebra.level} has its sides swapped`).toBe(namedOnTheRight);
          }
          if (vertebra.outline !== null) expect(simple(vertebra.outline.map(onScreen)), `${marks.film} ${vertebra.level}'s outline crosses itself`).toBe(true);
        }
        expect(windings.size).toBe(1);
      }
    },
  );
});

describe('the level names, stage by stage', () => {
  const FRONT_AND_SIDE: Partial<Record<StoryStage, Level[]>> = {
    // The pass names the 18 bodies from the top: the C7 body, then the 17,
    // T11 and T12 taken as one outline (T11), and the sacrum's outline as L5.
    'pass-result': ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'],
    // confirm-l5 on the outline above the sacrum's: every name moves one
    // down, so the C7 body is T1 and the sacrum's outline L6.
    'l5-set': ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'],
    // The split: the lower piece keeps T12, everything above moves one up.
    'after-split': ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'],
    't3-fixed': ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'],
    'all-clear': ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'],
  };
  const BENDING: Level[] = ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];
  const levels = (marks: FilmMarks | undefined) => marks?.vertebrae.map((vertebra) => vertebra.level);

  it('has no marks before the pass', () => {
    expect(caseOf('four-films-up').marks).toEqual([]);
    expect(caseOf('films-all-set').marks).toEqual([]);
  });

  it.each(['pass-result', 'l5-set', 'after-split', 't3-fixed', 'all-clear'] as const)('%s', (stage) => {
    expect(levels(marksOf(stage, 'front'))).toEqual(FRONT_AND_SIDE[stage]);
    expect(levels(marksOf(stage, 'side'))).toEqual(FRONT_AND_SIDE[stage]);
    expect(levels(marksOf(stage, 'bend-left'))).toEqual(BENDING);
    expect(levels(marksOf(stage, 'bend-right'))).toEqual(BENDING);
  });

  it('counts the front film one apart from each end until the split', () => {
    const naming = marksOf('pass-result', 'front')?.naming;
    expect(naming?.fromTop?.at(-1)).toBe('L5');
    expect(naming?.fromBottom?.at(-2)).toBe('L5');
    expect(naming?.flags).toContain('ends-disagree');
    const after = marksOf('after-split', 'front')?.naming;
    expect(after?.fromTop).toEqual(after?.fromBottom);
    expect(after?.flags).toEqual(['non-standard-count']);
  });

  it('keeps ids through the story: the split\'s upper piece keeps its id, the lower takes the next', () => {
    const before = marksOf('l5-set', 'front')?.vertebrae ?? [];
    const after = marksOf('after-split', 'front')?.vertebrae ?? [];
    expect(after.map((vertebra) => vertebra.id)).toEqual([...before.slice(0, 12).map((vertebra) => vertebra.id), 73, ...before.slice(12).map((vertebra) => vertebra.id)]);
    expect(marksOf('after-split', 'side')?.vertebrae[12]?.id).toBe(74);
    expect(caseOf('after-split').nextVertebraId).toBe(75);
    // L5 stays confirmed on the same vertebra.
    expect(marksOf('after-split', 'front')?.naming.l5ConfirmedOn).toBe(marksOf('l5-set', 'front')?.naming.l5ConfirmedOn);
  });

  it('marks what a person did, and only that', () => {
    const editedBy = (stage: StoryStage, film: FilmKind) =>
      marksOf(stage, film)?.vertebrae.filter((vertebra) => vertebra.confidence.reasons.includes('edited-by-user')).map((vertebra) => vertebra.level);
    expect(editedBy('l5-set', 'front')).toEqual([]);
    expect(editedBy('after-split', 'front')).toEqual(['T11', 'T12']);
    expect(editedBy('after-split', 'side')).toEqual(['T11', 'T12']);
    expect(editedBy('t3-fixed', 'front')).toEqual(['T3', 'T11', 'T12']);
    expect(editedBy('t3-fixed', 'side')).toEqual(['T11', 'T12']);
    const t1 = (film: FilmKind) => marksOf('all-clear', film)?.vertebrae.find((vertebra) => vertebra.level === 'T1');
    for (const film of ['front', 'side'] as const) {
      const corners = cornersOfVertebra(t1(film));
      expect(corners).toHaveLength(4);
      expect(corners.map((corner) => corner.source)).toEqual(['hand', 'hand', 'hand', 'hand']);
      expect(corners.every((corner) => !corner.pinned)).toBe(true);
    }
    const t3 = marksOf('t3-fixed', 'front')?.vertebrae.find((vertebra) => vertebra.level === 'T3');
    expect(t3?.outline?.length).toBeGreaterThanOrEqual(3);
    expect(t3?.marks.kind === 'front' && [t3.marks.points.supL.source, t3.marks.points.infL.source]).toEqual(['region-wand', 'region-wand']);
  });
});

describe('the story\'s outlines on the pass result', () => {
  it('holds the design\'s four that need a look, and T1\'s hidden pedicle', () => {
    const front = marksOf('pass-result', 'front')?.vertebrae ?? [];
    const notSure = front.filter((vertebra) => vertebra.confidence.band !== 'high').map((vertebra) => [vertebra.level, vertebra.confidence.band, vertebra.confidence.reasons]);
    expect(notSure).toEqual([
      ['T1', 'medium', ['pedicle-hidden']],
      ['T3', 'low', ['rib-overlap']],
      ['T11', 'low', ['odd-height']],
      ['L5', 'medium', ['level-uncertain']],
    ]);
    const t1 = front[1];
    expect(t1?.marks.kind === 'front' && t1.marks.points.pedL).toEqual({ seen: 'not-visible', source: 'auto' });
    // The merged outline is about twice as tall as its neighbours.
    const height = (k: number) => {
      const corners = cornersOfVertebra(front[k]);
      return (corners[3]?.at[1] ?? 0) - (corners[0]?.at[1] ?? 0);
    };
    expect(height(11) / ((height(10) + height(12)) / 2)).toBeGreaterThan(1.8);
  });
});

describe('the findings a check gives, stage by stage', () => {
  /** The finding kinds on one film, in order. */
  const kindsOn = (stage: StoryStage, film: FilmKind): CheckKind[] =>
    findingsAt(stage)
      .filter((f) => f.film === film)
      .map((f) => f.kind);
  const facts = (...kinds: CheckKind[]) => [...kinds, 'polarity-unconfirmed', 'privacy-unchecked'] as CheckKind[];

  it('four-films-up: every fact of every film is open', () => {
    expect(kindsOn('four-films-up', 'front')).toEqual(facts('upright-unconfirmed', 'left-right-unconfirmed', 'scale-unconfirmed'));
    expect(kindsOn('four-films-up', 'side')).toEqual(facts('upright-unconfirmed', 'facing-unconfirmed', 'scale-unconfirmed'));
    expect(kindsOn('four-films-up', 'bend-left')).toEqual(facts('upright-unconfirmed', 'left-right-unconfirmed', 'scale-unconfirmed'));
    expect(kindsOn('four-films-up', 'bend-right')).toEqual(facts('upright-unconfirmed', 'left-right-unconfirmed', 'scale-unconfirmed'));
  });

  it('films-all-set: the front film\'s side and the measured size are confirmed', () => {
    expect(kindsOn('films-all-set', 'front')).toEqual(facts('upright-unconfirmed', 'scale-unconfirmed'));
    expect(kindsOn('films-all-set', 'bend-right')).toEqual(facts('upright-unconfirmed', 'left-right-unconfirmed'));
  });

  it('pass-result: the merged outline, the off-by-one, L5 open everywhere, T1\'s pedicle', () => {
    expect(kindsOn('pass-result', 'front')).toEqual(['vertebra-missing', 'likely-off-by-one', 'l5-unconfirmed', ...facts(), 'pedicle-not-visible']);
    expect(kindsOn('pass-result', 'side')).toEqual(['vertebra-missing', 'likely-off-by-one', 'l5-unconfirmed', ...facts()]);
    expect(kindsOn('pass-result', 'bend-left')).toEqual(['l5-unconfirmed', ...facts('left-right-unconfirmed')]);
    const missing = findingsAt('pass-result').find((f) => f.kind === 'vertebra-missing' && f.film === 'front');
    expect(missing).toEqual({ kind: 'vertebra-missing', severity: 'blocking', film: 'front', vertebraId: 12, level: 'T11' });
  });

  it('l5-set: the count runs past L5 to the sacrum, and one is still missing', () => {
    expect(kindsOn('l5-set', 'front')).toEqual(['vertebra-missing', 'non-standard-count', ...facts(), 'pedicle-not-visible']);
    expect(findingsAt('l5-set').find((f) => f.kind === 'non-standard-count' && f.film === 'side')?.level).toBe('L6');
    expect(findingsAt('l5-set').find((f) => f.kind === 'pedicle-not-visible')?.level).toBe('T2');
  });

  it('after-split, t3-fixed and all-clear: nothing about the spine but the outline on the sacrum', () => {
    for (const stage of ['after-split', 't3-fixed', 'all-clear'] as const) {
      expect(kindsOn(stage, 'front')).toEqual(['non-standard-count', ...facts(), 'pedicle-not-visible']);
      expect(kindsOn(stage, 'side')).toEqual(['non-standard-count', ...facts()]);
      expect(kindsOn(stage, 'bend-left')).toEqual(facts('left-right-unconfirmed'));
      expect(kindsOn(stage, 'bend-right')).toEqual(facts('left-right-unconfirmed'));
    }
    expect(findingsAt('all-clear').some((f) => f.severity === 'blocking')).toBe(false);
  });
});
