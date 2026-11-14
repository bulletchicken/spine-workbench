import { expect, it } from 'vitest';
import type { FilmMarks, Mark, VertebraOnFilm } from '@/shapes';
import { renamedSide, sideReviewKey } from './side-review';

it('renames from the result baseline, drops below L5 and preserves every retained observation', () => {
  const corner: Mark = { at: [10, 20], source: 'auto', pinned: false, inferred: false };
  const base: VertebraOnFilm = { id: 0, level: 'L1', outline: null,
    confidence: { score: 0, band: 'low', reasons: ['level-uncertain'], calibrated: false, endplateSpreadPx: null, margin: null },
    marks: { kind: 'side', points: { supA: corner, supP: { ...corner, at: [30, 20] }, infA: { ...corner, at: [10, 40] }, infP: { ...corner, at: [30, 40] } } },
  };
  const marks: FilmMarks = { film: 'side', namedFor: 'anterior', vertebrae: [], landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: ['estimated'], l5ConfirmedOn: null },
  };
  marks.vertebrae = (['L1', 'L2', 'L3', 'L4', 'L5'] as const).map((level, i) => ({ ...base, id: i + 20, level }));
  marks.naming.flags = ['estimated'];
  const high = renamedSide(marks, 'too-high');
  expect(high.vertebrae.map((v) => v.level)).toEqual(['L2', 'L3', 'L4', 'L5']);
  high.vertebrae.forEach((v, i) => { expect({ ...v, level: marks.vertebrae[i]?.level }).toEqual(marks.vertebrae[i]); });
  expect(high.naming.flags).toEqual(['estimated']);
  expect(renamedSide(marks, 'too-high')).toEqual(high);
  const low = renamedSide(marks, 'too-low');
  expect(low.vertebrae.map((v) => v.level)).toEqual(['T12', 'L1', 'L2', 'L3', 'L4']);
  low.vertebrae.forEach((v, i) => { expect({ ...v, level: marks.vertebrae[i]?.level }).toEqual(marks.vertebrae[i]); });
  expect(low.vertebrae[0]?.marks).toBe(marks.vertebrae[0]?.marks);
  expect(low.naming).toEqual(marks.naming);
  expect(marks.vertebrae.map((v) => v.level)).toEqual(['L1', 'L2', 'L3', 'L4', 'L5']);
  expect(sideReviewKey(high, 2)).not.toBe(sideReviewKey(high, 3));
});
