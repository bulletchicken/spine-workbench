// A film's C7 landmark, worked out from its vertebrae: the TypeScript
// mirror of `Landmarks::c7_of` in engine/shapes/src/landmarks.rs, which
// the reader holds every case to.
//
// The corners are added in quad order, one after another, exactly as the
// Rust does, so the mean comes out the same to the last bit.

import type { Mark, VertebraMarks, VertebraOnFilm } from '@/shapes';

/**
 * A vertebra's four corners (a bending film's line ends) in quad order,
 * round the outline: upper left (or anterior), upper right (or
 * posterior), lower right, lower left.
 */
export function cornersOf(marks: VertebraMarks): [Mark, Mark, Mark, Mark] {
  switch (marks.kind) {
    case 'front':
      return [marks.points.supL, marks.points.supR, marks.points.infR, marks.points.infL];
    case 'side':
      return [marks.points.supA, marks.points.supP, marks.points.infP, marks.points.infA];
    case 'bending':
      return [marks.lines.sup[0], marks.lines.sup[1], marks.lines.inf[1], marks.lines.inf[0]];
  }
}

/**
 * The mean of the C7 vertebra's four corners, source `auto`, never
 * pinned, inferred when any corner is; `null` when no vertebra is C7.
 */
export function c7Of(vertebrae: readonly VertebraOnFilm[]): Mark | null {
  const c7 = vertebrae.find((vertebra) => vertebra.level === 'C7');
  if (c7 === undefined) return null;
  const [a, b, c, d] = cornersOf(c7.marks);
  return {
    at: [(a.at[0] + b.at[0] + c.at[0] + d.at[0]) / 4, (a.at[1] + b.at[1] + c.at[1] + d.at[1]) / 4],
    source: 'auto',
    pinned: false,
    inferred: a.inferred || b.inferred || c.inferred || d.inferred,
  };
}
