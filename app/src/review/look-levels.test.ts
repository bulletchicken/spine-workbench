import { expect, it } from 'vitest';
import type { FilmKind, FilmMarks, Mark, VertebraOnFilm } from '@/shapes';
import { lookLevels } from './look-levels';

it('groups doubtful levels across films, in order, and includes clear partners', () => {
  const corner: Mark = { at: [10, 10], source: 'auto', pinned: false, inferred: false };
  const films: FilmMarks[] = (['front', 'side', 'bend-left', 'bend-right'] as FilmKind[]).map((film, i) => ({
    film, namedFor: 'patient-left', landmarks: { c7: null, sacrumTop: null, hips: [] },
    naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
    vertebrae: (['T1', 'T2'] as const).map((level, j): VertebraOnFilm => ({
      id: i * 2 + j, level, outline: null,
      confidence: { score: 0.9, band: 'high', reasons: [], calibrated: false, endplateSpreadPx: null, margin: null },
      marks: { kind: 'side', points: { supA: { ...corner }, supP: { ...corner }, infA: { ...corner }, infP: { ...corner } } },
    })),
  }));
  for (const film of films) for (const v of film.vertebrae) v.confidence.band = 'high';
  const front = films[0];
  const side = films[1];
  if (front === undefined || side === undefined) throw new Error('missing films');
  const first = front.vertebrae[0];
  const second = side.vertebrae[1];
  if (first === undefined || second === undefined || second.marks.kind !== 'side') throw new Error('missing outlines');
  first.confidence.band = 'low';
  second.marks.points.supA.inferred = true;
  const levels = lookLevels([...films].reverse());
  expect(levels.map((entry) => [entry.level, entry.why])).toEqual([[first.level, 'less-sure'], [second.level, 'filled-in']]);
  expect(levels[0]?.on).toEqual(Object.fromEntries(films.map((film) => [film.film, film.vertebrae[0]?.id])));
  first.level = 'C7';
  expect(lookLevels(films).map((entry) => entry.level)).not.toContain('C7');
});
