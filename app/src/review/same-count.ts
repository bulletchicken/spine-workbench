// Whether two films count alike, so that a level name on one is the same
// bone on the other. They must agree on the whole chain: as many vertebrae,
// the same name on each in order, the same count, and the same names counted
// from the first rib and from the sacrum. A split, a merge or a missed
// vertebra on one film shows in at least one of these, even where the names
// from T1 to L5 happen to match, and then the films are not linked at all.
// Two films whose mistakes are exactly alike still count alike; the case
// alone cannot tell them apart.

import type { FilmMarks, Level } from '@/shapes';

function sameNames(a: readonly Level[] | null, b: readonly Level[] | null): boolean {
  if (a === null || b === null) return a === b;
  return a.length === b.length && a.every((level, index) => level === b[index]);
}

/** True when the two films agree on every vertebra's name, the count and both ends' counting. */
export function countAlike(a: FilmMarks, b: FilmMarks): boolean {
  return (
    a.naming.template === b.naming.template &&
    sameNames(
      a.vertebrae.map((vertebra) => vertebra.level),
      b.vertebrae.map((vertebra) => vertebra.level),
    ) &&
    sameNames(a.naming.fromTop, b.naming.fromTop) &&
    sameNames(a.naming.fromBottom, b.naming.fromBottom)
  );
}

/** True when the two films have a different number of vertebrae, which no edit on a bending film can change. */
export function levelCountsDiffer(a: FilmMarks, b: FilmMarks): boolean {
  return a.vertebrae.length !== b.vertebrae.length;
}
