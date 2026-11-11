import { describe, expect, it } from 'vitest';
import type { FilmKind, FilmMarks, Level, Template, VertebraOnFilm } from '@/shapes';
import { sameLevelOn } from './same-level';

// Worked by hand: films with vertebrae at the levels listed, ids counting up
// from the base given. A link is the same level name on the other film, and
// only when the two films count alike: the same count, as many vertebrae with
// the same names in the same order, and the same names counted from the first
// rib and from the sacrum (both unknown counts as the same).

function vertebra(id: number, level: Level): VertebraOnFilm {
  const point: [number, number] = [0, 0];
  const mark = { at: point, source: 'auto' as const, pinned: false, inferred: false };
  return {
    id,
    level,
    confidence: { score: 0.95, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
    outline: null,
    marks: { kind: 'bending', lines: { sup: [mark, mark], inf: [mark, mark] } },
  };
}

function film(kind: FilmKind, template: Template, levels: Level[], idBase: number, fromTop: Level[] | null = null, fromBottom: Level[] | null = null): FilmMarks {
  return {
    film: kind,
    vertebrae: levels.map((level, index) => vertebra(idBase + index, level)),
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template, chosenBy: 'engine', alternatives: [], fromTop, fromBottom, flags: [], l5ConfirmedOn: null },
    namedFor: 'patient-left',
  };
}

const T1_TO_L5: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];
const C7_TO_L5: Level[] = ['C7', ...T1_TO_L5];
const T1_TO_L6: Level[] = [...T1_TO_L5, 'L6'];

describe('sameLevelOn', () => {
  it('links the same level name, with each film keeping its own ids', () => {
    const front = film('front', 'standard', T1_TO_L5, 1);
    const side = film('side', 'standard', T1_TO_L5, 101);
    expect(sameLevelOn(front, side, 5)).toBe(105);
    expect(sameLevelOn(side, front, 105)).toBe(5);
  });

  it('links nothing when the other film counts a different number of levels', () => {
    const front = film('front', 'standard', T1_TO_L5, 1);
    const side = film('side', 'extra-thoracic', [...T1_TO_L5.slice(0, 12), 'T13', ...T1_TO_L5.slice(12)], 101);
    for (const vertebra of front.vertebrae) {
      expect(sameLevelOn(front, side, vertebra.id)).toBeNull();
    }
  });

  it('links nothing when the other film has a vertebra fewer, under the same count', () => {
    const front = film('front', 'standard', T1_TO_L5, 1);
    const side = film('side', 'standard', T1_TO_L5.filter((level) => level !== 'T12'), 101);
    for (const vertebra of front.vertebrae) {
      expect(sameLevelOn(front, side, vertebra.id)).toBeNull();
    }
  });

  // The sample case at the pass. Front holds the C7 body, T11 and T12 taken
  // as one outline, and an outline on the sacrum: 18 vertebrae named C7 to L5
  // from the rib, T1 to L6 from the sacrum. A bending film found right holds
  // C7 to L5, 18 vertebrae, both counts C7 to L5. The names are the same, but
  // Front's id 17 (named L4) is the real L5 while the bending film's L4 is the
  // real L4: a link by name would be one bone off from T12 down. The counts
  // from the sacrum differ, so nothing is linked.
  it('links nothing when the names match but the counts from the sacrum differ', () => {
    const front = film('front', 'standard', C7_TO_L5, 1, C7_TO_L5, T1_TO_L6);
    const bending = film('bend-right', 'standard', C7_TO_L5, 55, C7_TO_L5, C7_TO_L5);
    for (const vertebra of front.vertebrae) {
      expect(sameLevelOn(front, bending, vertebra.id)).toBeNull();
    }
  });

  // The same Front once L5 is set on id 17: every name one further down, C7
  // body T1 and id 17 L5. The bending film found right is left as found.
  // Front's T4 (id 4, the real T3) would link the bending film's T4 (id 59,
  // the real T4). The names now differ, so nothing is linked.
  it('links nothing after L5 is set on a film whose chain differs', () => {
    const front = film('front', 'standard', T1_TO_L6, 1, C7_TO_L5, T1_TO_L6);
    const bending = film('bend-right', 'standard', C7_TO_L5, 55, C7_TO_L5, C7_TO_L5);
    for (const vertebra of front.vertebrae) {
      expect(sameLevelOn(front, bending, vertebra.id)).toBeNull();
    }
  });

  it('links nothing when one film could count from the first rib and the other could not', () => {
    const front = film('front', 'standard', C7_TO_L5, 1, C7_TO_L5, C7_TO_L5);
    const side = film('side', 'standard', C7_TO_L5, 101, null, C7_TO_L5);
    expect(sameLevelOn(front, side, 5)).toBeNull();
  });

  it('follows every vertebra when the two films count alike from both ends', () => {
    const front = film('front', 'standard', C7_TO_L5, 1, C7_TO_L5, T1_TO_L6);
    const side = film('side', 'standard', C7_TO_L5, 101, C7_TO_L5, T1_TO_L6);
    for (const vertebra of front.vertebrae) {
      expect(sameLevelOn(front, side, vertebra.id)).toBe(vertebra.id + 100);
    }
  });

  it('follows C7 and a level below the count when the films count alike', () => {
    const front = film('front', 'standard', ['C7', ...T1_TO_L5, 'L6'], 1);
    const side = film('side', 'standard', ['C7', ...T1_TO_L5, 'L6'], 101);
    expect(sameLevelOn(front, side, 1)).toBe(101);
    expect(sameLevelOn(front, side, 19)).toBe(119);
  });

  it('gives null for a vertebra id the first film does not have', () => {
    const front = film('front', 'standard', T1_TO_L5, 1);
    const side = film('side', 'standard', T1_TO_L5, 101);
    expect(sameLevelOn(front, side, 999)).toBeNull();
  });
});
