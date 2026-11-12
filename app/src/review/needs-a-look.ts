// Which outlines a person should look at after a search. The one rule, in
// one place: an outline needs a look when the engine is less sure of it
// (its band is low) or when any of its corners was filled in rather than
// seen. Outlines below a band of low sat on a vertebra far less often than
// the rest, and a filled-in outline was never seen at all, so neither is
// ever passed off as found.

import type { Mark, VertebraOnFilm } from '@/shapes';

/** The corners of an outline: a body's four corners, or the four ends of a bending film's two lines. Pedicles are not asked. */
function cornersOf(vertebra: VertebraOnFilm): readonly Mark[] {
  const { marks } = vertebra;
  switch (marks.kind) {
    case 'front': {
      const { supL, supR, infL, infR } = marks.points;
      return [supL, supR, infR, infL];
    }
    case 'side': {
      const { supA, supP, infA, infP } = marks.points;
      return [supA, supP, infP, infA];
    }
    case 'bending':
      return [...marks.lines.sup, ...marks.lines.inf];
  }
}

/** Any corner of the outline was placed without being seen: filled in from its neighbours. */
export function filledIn(vertebra: VertebraOnFilm): boolean {
  return cornersOf(vertebra).some((corner) => corner.inferred);
}

/** The outline is drawn amber and named in the dock: its band is low, or it was filled in. */
export function needsALook(vertebra: VertebraOnFilm): boolean {
  return vertebra.confidence.band === 'low' || filledIn(vertebra);
}
