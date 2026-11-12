import type { Film, FilmFacts } from '@/shapes';

/** Reading establishes the stored film's facts, without confirming its geometry. */
export function filmRecordFrom(facts: FilmFacts): Film {
  return {
    kind: facts.kind,
    image: { path: `films/${facts.kind}.png`, width: facts.width, height: facts.height, bitsStored: facts.bitsStored },
    source: facts.source, keptTags: facts.keptTags, values: facts.values, flags: facts.flags,
    blanked: [],
    geometry: {
      mmPerPx: null, mmPerPxVertical: null, scale: null, quarterTurns: 0, imageRight: null,
      confirmed: { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false },
    },
  };
}
