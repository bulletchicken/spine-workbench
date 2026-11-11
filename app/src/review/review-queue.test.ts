import { describe, expect, it } from 'vitest';
import type { CheckFinding, ConfidenceBand, ConfidenceReason, FilmKind, FilmMarks, Level, Mark, MarkSource, Px, VertebraOnFilm } from '@/shapes';
import { howSure } from './how-sure';
import { reviewQueue } from './review-queue';
import { standingRows } from './standing-checks';
import { tally } from './tally';

// The design's story, written out film by film and level by level, so every
// expected value below can be worked out by hand from this file.
//
// The automatic pass finds 18 outlines on Front and on Side: the C7 body the
// engine's chain always holds, and the design's 17. Two of the 17 are one
// outline (T11 and T12 taken as one, twice as tall as its neighbours, low
// with `odd-height`), and the lowest sits on the top of the sacrum (medium,
// `level-uncertain`). T3 is short of its bone on Front only (low,
// `rib-overlap`; Side sees it clearly). T1 is medium on both films.
//
// Front at the pass (Side is the same with every id 18 higher, T1 medium for
// `weak-gap` and T3 high):
//
//   id  from the rib  from the sacrum  band
//    1  C7            T1               high     not counted (above T1)
//    2  T1            T2               medium   left pedicle not visible
//    3  T2            T3               high
//    4  T3            T4               low      short of its bone
//    5  T4 ... 11 T10                  high
//   12  T11           T12              low      the tall outline, odd height
//   13  T12           L1               high
//   14  L1 ... 17 L4                   high
//   18  L5            L6               medium   on the sacrum
//
// The pass names from the rib. Counted from the rib, id 18 is L5; counted
// up from the sacrum, id 17 is. So the L5 question is open, with A the
// upper (id 17, which Side links to by its name L4: id 35) and B the lower
// (id 18; on Side id 36).
//
// The case's check reports the missing vertebra at the outline that
// swallowed it, on each film, and that the names may be off by one.
//
// At the pass, by hand:
//   queue  which is L5 (A 17/35, B 18/36); missing (T11, target 12/30);
//          needs T3 (4/22, least sure on Front); look T1 (2/20).
//          A level's key is its kind and its Front id (needs:4, look:2),
//          so a skip stays on the vertebra when the names move.
//          T11 is not asked again as a level (the missing item is about
//          it), nor L5 (the question is about it).
//   tally  over T1 to L5, 17 levels: needs T3 and T11, look T1 and L5,
//          sure the other 13, none set by you. The design's 13, 2, 2, 0.
//   found  T1 to L5 is 17 levels, each has an outline, and the check names
//          T11 as missing: 16 of 17, bad.
//
// Then, as the design walks it:
//   L5 set     A is chosen on both films. Every name moves one down: the C7
//              body becomes T1, the tall outline T12, A L5, the outline on
//              the sacrum L6 (not counted, flagged non-standard).
//   split      the tall outline is split at its disc on both films; each
//              piece was aimed at (`edited-by-user`), the lower takes the
//              next id (73 on Front, 74 on Side). The names above the split
//              move one up again, so C7 comes back. On Front the split
//              needed room and moved L1's upper corners (boundary wand).
//   T3 fixed   the region wand adds the bone T3's outline missed, on Front.
//   all clear  "Looks right" on T1, Front and Side: its corners set by hand.

type Row = readonly [id: number, level: Level, band: ConfidenceBand, reasons?: readonly ConfidenceReason[]];

const FRONT_AT_THE_PASS: readonly Row[] = [
  [1, 'C7', 'high'],
  [2, 'T1', 'medium', ['pedicle-hidden']],
  [3, 'T2', 'high'],
  [4, 'T3', 'low', ['rib-overlap']],
  [5, 'T4', 'high'],
  [6, 'T5', 'high'],
  [7, 'T6', 'high'],
  [8, 'T7', 'high'],
  [9, 'T8', 'high'],
  [10, 'T9', 'high'],
  [11, 'T10', 'high'],
  [12, 'T11', 'low', ['odd-height']],
  [13, 'T12', 'high'],
  [14, 'L1', 'high'],
  [15, 'L2', 'high'],
  [16, 'L3', 'high'],
  [17, 'L4', 'high'],
  [18, 'L5', 'medium', ['level-uncertain']],
];

const SIDE_AT_THE_PASS: readonly Row[] = [
  [19, 'C7', 'high'],
  [20, 'T1', 'medium', ['weak-gap']],
  [21, 'T2', 'high'],
  [22, 'T3', 'high'],
  [23, 'T4', 'high'],
  [24, 'T5', 'high'],
  [25, 'T6', 'high'],
  [26, 'T7', 'high'],
  [27, 'T8', 'high'],
  [28, 'T9', 'high'],
  [29, 'T10', 'high'],
  [30, 'T11', 'low', ['odd-height']],
  [31, 'T12', 'high'],
  [32, 'L1', 'high'],
  [33, 'L2', 'high'],
  [34, 'L3', 'high'],
  [35, 'L4', 'high'],
  [36, 'L5', 'medium', ['level-uncertain']],
];

const FROM_THE_RIB: Level[] = ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];
const FROM_THE_SACRUM: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];

/** The names once L5 is set on id 17 (Side 35): each one level further down. */
const NAMES_AFTER_L5: Level[] = FROM_THE_SACRUM;

/** The names after the split, top to bottom, with the lower piece between T11 and L1. */
const NAMES_AFTER_SPLIT: Level[] = ['C7', 'T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];

const TALL = { front: 12, side: 30 } as const;
const A = { front: 17, side: 35 } as const;
const LOWER_PIECE = { front: 73, side: 74 } as const;

const auto = (at: Px): Mark => ({ at, source: 'auto', pinned: false, inferred: false });

/** An outline's marks, its upper endplate at `top` and its lower `height` below. */
function marksAt(kind: 'front' | 'side', top: number, height: number): VertebraOnFilm['marks'] {
  const bottom = top + height;
  if (kind === 'side') {
    return { kind: 'side', points: { supA: auto([60, top]), supP: auto([140, top]), infA: auto([60, bottom]), infP: auto([140, bottom]) } };
  }
  return {
    kind: 'front',
    points: {
      supL: auto([140, top]),
      supR: auto([60, top]),
      infL: auto([140, bottom]),
      infR: auto([60, bottom]),
      pedL: { seen: 'visible', at: [125, top + 8], source: 'auto', pinned: false, inferred: false, ringScore: 0.8 },
      pedR: { seen: 'visible', at: [75, top + 8], source: 'auto', pinned: false, inferred: false, ringScore: 0.8 },
    },
  };
}

const SCORE: Record<ConfidenceBand, number> = { high: 0.95, medium: 0.7, low: 0.4 };

/** The film the pass leaves: outlines 40 px apart, each 30 px tall but the tall one, 70. */
function atThePass(kind: 'front' | 'side'): FilmMarks {
  const rows = kind === 'front' ? FRONT_AT_THE_PASS : SIDE_AT_THE_PASS;
  const vertebrae = rows.map(([id, level, band, reasons = []], index): VertebraOnFilm => {
    const height = id === TALL[kind] ? 70 : 30;
    const vertebra: VertebraOnFilm = {
      id,
      level,
      marks: marksAt(kind, 100 + index * 40, height),
      confidence: { score: SCORE[band], band, reasons: [...reasons], calibrated: false, endplateSpreadPx: null, margin: null },
      outline: null,
    };
    if (vertebra.marks.kind === 'front' && reasons.includes('pedicle-hidden')) {
      vertebra.marks.points.pedL = { seen: 'not-visible', source: 'auto' };
    }
    return vertebra;
  });
  return {
    film: kind,
    vertebrae,
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: {
      template: 'standard',
      chosenBy: 'engine',
      alternatives: [],
      fromTop: [...FROM_THE_RIB],
      fromBottom: [...FROM_THE_SACRUM],
      flags: ['ends-disagree'],
      l5ConfirmedOn: null,
    },
    namedFor: kind === 'front' ? 'patient-left' : 'anterior',
  };
}

const renamed = (film: FilmMarks, names: readonly Level[]): VertebraOnFilm[] =>
  film.vertebrae.map((vertebra, index) => ({ ...vertebra, level: names[index] ?? vertebra.level }));

function l5Set(kind: 'front' | 'side'): FilmMarks {
  const film = atThePass(kind);
  return { ...film, vertebrae: renamed(film, NAMES_AFTER_L5), naming: { ...film.naming, flags: ['non-standard-count'], l5ConfirmedOn: A[kind] } };
}

/** Every corner of a vertebra's marks, with the given source when `which` picks it. */
function withSource(vertebra: VertebraOnFilm, source: MarkSource, which: (name: string) => boolean): VertebraOnFilm {
  const set = (name: string, mark: Mark): Mark => (which(name) ? { ...mark, source } : mark);
  const marks = vertebra.marks;
  if (marks.kind === 'front') {
    const p = marks.points;
    return {
      ...vertebra,
      marks: { kind: 'front', points: { ...p, supL: set('supL', p.supL), supR: set('supR', p.supR), infL: set('infL', p.infL), infR: set('infR', p.infR) } },
    };
  }
  if (marks.kind === 'side') {
    const p = marks.points;
    return { ...vertebra, marks: { kind: 'side', points: { supA: set('supA', p.supA), supP: set('supP', p.supP), infA: set('infA', p.infA), infP: set('infP', p.infP) } } };
  }
  return vertebra;
}

const isUpper = (name: string) => name.startsWith('sup');
const isLower = (name: string) => name.startsWith('inf');
const isLeft = (name: string) => name.endsWith('L');
const everyCorner = () => true;

function afterSplit(kind: 'front' | 'side'): FilmMarks {
  const film = l5Set(kind);
  const at = film.vertebrae.findIndex((vertebra) => vertebra.id === TALL[kind]);
  const tall = film.vertebrae[at];
  if (!tall) throw new Error('the story has a tall outline');
  const top = 100 + at * 40;
  const aimed = { score: tall.confidence.score, band: tall.confidence.band, reasons: ['edited-by-user' as const], calibrated: false, endplateSpreadPx: null, margin: null };
  const upper = withSource({ ...tall, marks: marksAt(kind, top, 30), confidence: aimed }, 'boundary-wand', isLower);
  const lower = withSource({ ...tall, id: LOWER_PIECE[kind], marks: marksAt(kind, top + 40, 30), confidence: aimed }, 'boundary-wand', isUpper);
  const below = film.vertebrae.slice(at + 1);
  const movedNeighbour = kind === 'front' ? below.map((vertebra, k) => (k === 0 ? withSource(vertebra, 'boundary-wand', isUpper) : vertebra)) : below;
  const vertebrae = [...film.vertebrae.slice(0, at), upper, lower, ...movedNeighbour].map((vertebra, index) => ({
    ...vertebra,
    level: NAMES_AFTER_SPLIT[index] ?? vertebra.level,
  }));
  const names = vertebrae.map((vertebra) => vertebra.level);
  return { ...film, vertebrae, naming: { ...film.naming, fromTop: names, fromBottom: [...names] } };
}

function t3Fixed(kind: 'front' | 'side'): FilmMarks {
  const film = afterSplit(kind);
  if (kind === 'side') return film;
  const vertebrae = film.vertebrae.map((vertebra) => {
    if (vertebra.level !== 'T3') return vertebra;
    const edited = { ...vertebra, confidence: { ...vertebra.confidence, reasons: [...vertebra.confidence.reasons, 'edited-by-user' as const] } };
    return withSource(edited, 'region-wand', isLeft);
  });
  return { ...film, vertebrae };
}

function allClear(kind: 'front' | 'side'): FilmMarks {
  const film = t3Fixed(kind);
  return { ...film, vertebrae: film.vertebrae.map((vertebra) => (vertebra.level === 'T1' ? withSource(vertebra, 'hand', everyCorner) : vertebra)) };
}

const finding = (kind: CheckFinding['kind'], film: FilmKind, vertebraId: number | null, level: Level | null): CheckFinding => ({
  kind,
  severity: kind === 'vertebra-missing' ? 'blocking' : 'warning',
  film,
  vertebraId,
  level,
});

const STAGES = {
  pass: {
    films: [atThePass('front'), atThePass('side')],
    findings: [
      finding('vertebra-missing', 'front', 12, 'T11'),
      finding('likely-off-by-one', 'front', 18, 'L5'),
      finding('l5-unconfirmed', 'front', null, null),
      finding('vertebra-missing', 'side', 30, 'T11'),
      finding('likely-off-by-one', 'side', 36, 'L5'),
      finding('l5-unconfirmed', 'side', null, null),
      finding('pedicle-not-visible', 'front', 2, 'T1'),
    ],
  },
  l5Set: {
    films: [l5Set('front'), l5Set('side')],
    findings: [
      finding('vertebra-missing', 'front', 12, 'T12'),
      finding('non-standard-count', 'front', 18, 'L6'),
      finding('vertebra-missing', 'side', 30, 'T12'),
      finding('non-standard-count', 'side', 36, 'L6'),
      finding('pedicle-not-visible', 'front', 2, 'T2'),
    ],
  },
  afterSplit: {
    films: [afterSplit('front'), afterSplit('side')],
    findings: [finding('non-standard-count', 'front', 18, 'L6'), finding('non-standard-count', 'side', 36, 'L6'), finding('pedicle-not-visible', 'front', 2, 'T1')],
  },
  t3Fixed: {
    films: [t3Fixed('front'), t3Fixed('side')],
    findings: [finding('non-standard-count', 'front', 18, 'L6'), finding('non-standard-count', 'side', 36, 'L6'), finding('pedicle-not-visible', 'front', 2, 'T1')],
  },
  allClear: {
    films: [allClear('front'), allClear('side')],
    findings: [finding('non-standard-count', 'front', 18, 'L6'), finding('non-standard-count', 'side', 36, 'L6'), finding('pedicle-not-visible', 'front', 2, 'T1')],
  },
};

const WHICH_IS_L5 = { key: 'which-is-l5', kind: 'which-is-l5', level: 'L5', film: 'front', a: { front: 17, side: 35 }, b: { front: 18, side: 36 } };
const MISSING_T11 = { key: 'missing', kind: 'missing', level: 'T11', film: 'front', ids: { front: 12, side: 30 } };
const NEEDS_T3 = { key: 'needs:4', kind: 'needs', level: 'T3', film: 'front', ids: { front: 4, side: 22 } };
const LOOK_T1 = { key: 'look:2', kind: 'look', level: 'T1', film: 'front', ids: { front: 2, side: 20 } };

function vertebraWithId(films: readonly FilmMarks[], id: number): [FilmMarks, VertebraOnFilm] {
  for (const film of films) {
    const vertebra = film.vertebrae.find((candidate) => candidate.id === id);
    if (vertebra) return [film, vertebra];
  }
  throw new Error(`no vertebra ${String(id)} in the story`);
}

const stateOf = (films: readonly FilmMarks[], id: number) => howSure(...vertebraWithId(films, id));
const rowOf = (films: readonly FilmMarks[], findings: readonly CheckFinding[], row: string) => standingRows(films, findings).find((r) => r.row === row);

describe('the design story: at the pass', () => {
  const { films, findings } = STAGES.pass;

  it('walks L5, the missing vertebra, the one that needs you, then the one worth a look', () => {
    expect(reviewQueue(films, findings)).toEqual([WHICH_IS_L5, MISSING_T11, NEEDS_T3, LOOK_T1]);
  });

  it('tallies 13 sure, 2 worth a look, 2 needing you and none set by you', () => {
    expect(tally(films)).toEqual({ sure: 13, look: 2, needs: 2, byyou: 0 });
  });

  it('reads 16 of 17 found, bad, and names that may be off by one', () => {
    expect(rowOf(films, findings, 'found')).toEqual({ row: 'found', state: 'bad', found: 16, of: 17, missing: 1 });
    expect(rowOf(films, findings, 'in-order')).toEqual({ row: 'in-order', state: 'ok', reason: null });
    expect(rowOf(films, findings, 'names')).toEqual({ row: 'names', state: 'warn', reason: 'off-by-one' });
    expect(standingRows(films, findings)).toHaveLength(3);
  });

  it('leaves out an item whose key was skipped, and keeps the rest in order', () => {
    expect(reviewQueue(films, findings, ['needs:4', 'which-is-l5'])).toEqual([MISSING_T11, LOOK_T1]);
  });
});

describe('the design story: after L5 is set on A', () => {
  const { films, findings } = STAGES.l5Set;

  it('drops the question; the missing outline is now T12, T3 is now T4 and T1 is T2', () => {
    expect(reviewQueue(films, findings)).toEqual([
      { key: 'missing', kind: 'missing', level: 'T12', film: 'front', ids: { front: 12, side: 30 } },
      { key: 'needs:4', kind: 'needs', level: 'T4', film: 'front', ids: { front: 4, side: 22 } },
      { key: 'look:2', kind: 'look', level: 'T2', film: 'front', ids: { front: 2, side: 20 } },
    ]);
  });

  it('reads 16 of 17 found, and level names set by you with the unusual count as a second line', () => {
    expect(rowOf(films, findings, 'found')).toEqual({ row: 'found', state: 'bad', found: 16, of: 17, missing: 1 });
    expect(rowOf(films, findings, 'names')).toEqual({ row: 'names', state: 'you', reason: 'non-standard-count' });
  });

  it('counts the outline on the sacrum, now L6, as not counted, and the chosen L5 as set by you', () => {
    expect(stateOf(films, 18)).toBe('sacrum');
    expect(stateOf(films, 17)).toBe('byyou');
    expect(tally(films)).toEqual({ sure: 13, look: 1, needs: 2, byyou: 1 });
  });
});

describe('the design story: after the split', () => {
  const { films, findings } = STAGES.afterSplit;

  it('asks about T3, then T1', () => {
    expect(reviewQueue(films, findings)).toEqual([NEEDS_T3, LOOK_T1]);
  });

  it('reads 17 of 17 found', () => {
    expect(rowOf(films, findings, 'found')).toEqual({ row: 'found', state: 'ok', found: 17, of: 17, missing: 0 });
  });

  it('the two pieces the wand was aimed at are set by you; L1, which it only moved, is still sure', () => {
    expect(stateOf(films, TALL.front)).toBe('byyou');
    expect(stateOf(films, LOWER_PIECE.front)).toBe('byyou');
    expect(stateOf(films, 13)).toBe('sure');
  });

  it('keeps all three so after the case is saved and opened again', () => {
    const reopened = JSON.parse(JSON.stringify(films)) as FilmMarks[];
    expect(stateOf(reopened, TALL.front)).toBe('byyou');
    expect(stateOf(reopened, LOWER_PIECE.front)).toBe('byyou');
    expect(stateOf(reopened, 13)).toBe('sure');
    expect(reviewQueue(reopened, findings)).toEqual([NEEDS_T3, LOOK_T1]);
  });
});

describe('the design story: to the end', () => {
  it('with T3 fixed, only T1 is left', () => {
    const { films, findings } = STAGES.t3Fixed;
    expect(reviewQueue(films, findings)).toEqual([LOOK_T1]);
    expect(stateOf(films, 4)).toBe('byyou');
  });

  it('with T1 confirmed, nothing is left: T1, T3, both pieces and L5 set by you, 12 sure', () => {
    const { films, findings } = STAGES.allClear;
    expect(reviewQueue(films, findings)).toEqual([]);
    expect(tally(films)).toEqual({ sure: 12, look: 0, needs: 0, byyou: 5 });
    expect(rowOf(films, findings, 'found')).toEqual({ row: 'found', state: 'ok', found: 17, of: 17, missing: 0 });
  });
});

describe('reviewQueue: the rules behind the story', () => {
  it('keeps a skip on the vertebra while its name moves: T1 at the pass, T2 once L5 is set, T1 after the split', () => {
    expect(reviewQueue(STAGES.pass.films, STAGES.pass.findings, ['look:2']).map((item) => item.key)).toEqual(['which-is-l5', 'missing', 'needs:4']);
    expect(reviewQueue(STAGES.l5Set.films, STAGES.l5Set.findings, ['look:2']).map((item) => item.key)).toEqual(['missing', 'needs:4']);
    expect(reviewQueue(STAGES.afterSplit.films, STAGES.afterSplit.findings, ['look:2'])).toEqual([NEEDS_T3]);
  });

  // T6 is sure on Front (id 7) and low on Side (id 25). The level needs you,
  // it is least sure on Side, and it comes after T3 (top to bottom). Its key
  // is still Front's id. Front and Side count alike, so both ids are there.
  it('names the film where a level is least sure, here Side', () => {
    const [front, side] = STAGES.pass.films;
    if (!front || !side) throw new Error('the story has two films');
    const lowT6 = { ...side, vertebrae: side.vertebrae.map((v) => (v.id === 25 ? { ...v, confidence: { ...v.confidence, score: 0.4, band: 'low' as const } } : v)) };
    expect(reviewQueue([front, lowT6], STAGES.pass.findings)).toEqual([
      WHICH_IS_L5,
      MISSING_T11,
      NEEDS_T3,
      { key: 'needs:7', kind: 'needs', level: 'T6', film: 'side', ids: { side: 25, front: 7 } },
      LOOK_T1,
    ]);
  });

  // A bending film found right, C7 to L5 (ids 101 to 118), with T6 (id 107)
  // low. Only Front and Side are tallied and walked, so nothing changes; its
  // chain differs from Front's (the counts from the sacrum differ), so no
  // item links to it either.
  it('leaves the bending films out of the tally and the queue', () => {
    const line = (y: number): Mark => auto([100, y]);
    const bending: FilmMarks = {
      film: 'bend-left',
      vertebrae: FROM_THE_RIB.map((level, index) => ({
        id: 101 + index,
        level,
        marks: { kind: 'bending', lines: { sup: [line(100 + index * 40), line(100 + index * 40)], inf: [line(130 + index * 40), line(130 + index * 40)] } },
        confidence: level === 'T6' ? { score: 0.4, band: 'low', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null } : { score: 0.95, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
        outline: null,
      })),
      landmarks: { c7: null, sacrumTop: null, hips: [] },
      naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: [...FROM_THE_RIB], fromBottom: [...FROM_THE_RIB], flags: [], l5ConfirmedOn: null },
      namedFor: 'patient-left',
    };
    const films = [...STAGES.pass.films, bending];
    expect(tally(films)).toEqual({ sure: 13, look: 2, needs: 2, byyou: 0 });
    expect(reviewQueue(films, STAGES.pass.findings)).toEqual([WHICH_IS_L5, MISSING_T11, NEEDS_T3, LOOK_T1]);
  });

  it('asks no L5 question when both counts agree', () => {
    const [front, side] = STAGES.pass.films;
    if (!front || !side) throw new Error('the story has two films');
    const agreed = { ...front, naming: { ...front.naming, fromBottom: [...FROM_THE_RIB] } };
    expect(reviewQueue([agreed, side], STAGES.pass.findings).map((item) => item.key)).toEqual(['missing', 'needs:4', 'look:2', 'look:18']);
  });

  it('gives the missing item no target when no outline is odd in height, and then asks about that level', () => {
    const plain = STAGES.l5Set.films.map((film) => ({
      ...film,
      vertebrae: film.vertebrae.map((v) => ({ ...v, confidence: { ...v.confidence, reasons: v.confidence.reasons.filter((r) => r !== 'odd-height') } })),
    }));
    const queue = reviewQueue(plain, STAGES.l5Set.findings);
    expect(queue[0]).toEqual({ key: 'missing', kind: 'missing', level: 'T12', film: 'front', ids: {} });
    expect(queue.map((item) => item.key)).toEqual(['missing', 'needs:4', 'needs:12', 'look:2']);
  });

  it('aims the missing item at the tallest of several odd-height outlines', () => {
    const [front, side] = STAGES.l5Set.films;
    if (!front || !side) throw new Error('the story has two films');
    // T5 (id 5) is marked odd too, at its own 30 px; the tall one is 70 px.
    const twoOdd = { ...front, vertebrae: front.vertebrae.map((v) => (v.id === 5 ? { ...v, confidence: { ...v.confidence, reasons: ['odd-height' as const] } } : v)) };
    const missing = reviewQueue([twoOdd, side], STAGES.l5Set.findings).find((item) => item.kind === 'missing');
    expect(missing).toMatchObject({ ids: { front: 12, side: 30 } });
  });

  it('asks nothing when the case has no findings and no level needs a person', () => {
    expect(reviewQueue(STAGES.allClear.films, [])).toEqual([]);
  });

  it('returns nothing for a case with no films', () => {
    expect(reviewQueue([], STAGES.pass.findings)).toEqual([]);
  });
});
