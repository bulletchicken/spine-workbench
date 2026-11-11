import { describe, expect, it } from 'vitest';
import type { FilmMarks, Level, Template, VertebraOnFilm } from '@/shapes';
import { missingLevels } from './missing-levels';

// Worked by hand: the levels from T1 to L5 that a count gives, less the
// levels that have a vertebra on the film. Levels above T1, and levels below
// L5 (T13 and L6 included), are never reported.

function film(template: Template, levels: Level[]): FilmMarks {
  const mark = { at: [0, 0] as [number, number], source: 'auto' as const, pinned: false, inferred: false };
  const vertebrae: VertebraOnFilm[] = levels.map((level, index) => ({
    id: index + 1,
    level,
    confidence: { score: 0.95, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
    outline: null,
    marks: { kind: 'bending', lines: { sup: [mark, mark], inf: [mark, mark] } },
  }));
  return {
    film: 'front',
    vertebrae,
    landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template, chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
    namedFor: 'patient-left',
  };
}

const T1_TO_L5: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];

describe('missingLevels', () => {
  it('reports nothing when every level from T1 to L5 has a vertebra', () => {
    expect(missingLevels(film('standard', ['C7', ...T1_TO_L5]))).toEqual([]);
  });

  it('reports the one level with no vertebra, between T1 and L5', () => {
    const withoutT12 = T1_TO_L5.filter((level) => level !== 'T12');
    expect(missingLevels(film('standard', ['C7', ...withoutT12]))).toEqual(['T12']);
  });

  it('reports the levels in order, top to bottom', () => {
    const gaps = T1_TO_L5.filter((level) => !['T3', 'T9', 'L2'].includes(level));
    expect(missingLevels(film('standard', gaps))).toEqual(['T3', 'T9', 'L2']);
  });

  it('does not expect L5 under a count that lacks it', () => {
    const levels: Level[] = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4'];
    expect(missingLevels(film('missing-lumbar', levels))).toEqual([]);
  });

  it('reports L5 when the count has it and it is absent', () => {
    const levels = T1_TO_L5.filter((level) => level !== 'L5');
    expect(missingLevels(film('standard', levels))).toEqual(['L5']);
  });

  it('never reports L6, even when the count has it and it is absent', () => {
    expect(missingLevels(film('extra-lumbar', T1_TO_L5))).toEqual([]);
  });

  it('expects T13 under an extra thoracic count and reports it when absent', () => {
    const extra: Level[] = [...T1_TO_L5.slice(0, 12), 'T13', ...T1_TO_L5.slice(12)];
    expect(missingLevels(film('extra-thoracic', extra.filter((level) => level !== 'T13')))).toEqual(['T13']);
  });
});
