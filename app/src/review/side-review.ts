import { TEMPLATE_LEVELS, type FilmMarks } from '@/shapes';

export type SideNameChange = 'too-high' | 'too-low';

/** Only the names change. Boundary outlines are never moved to a neighbour. */
export function renamedSide(baseline: FilmMarks, change: SideNameChange): FilmMarks {
  const lumbar = TEMPLATE_LEVELS.standard.filter((level) => level.startsWith('L'));
  return { ...baseline, vertebrae: baseline.vertebrae.flatMap((v) => {
    const at = lumbar.indexOf(v.level);
    if (at < 0) return [];
    const level = change === 'too-high' ? lumbar[at + 1] : at === 0 ? 'T12' : lumbar[at - 1];
    return level === undefined ? [] : [{ ...v, level }];
  }) };
}

export function sideReviewKey(marks: FilmMarks, revision: number): string {
  return JSON.stringify([revision, marks.vertebrae.map((v) => v.id)]);
}
