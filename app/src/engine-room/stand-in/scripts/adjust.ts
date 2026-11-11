// How the stand-in behaves at step 4, Adjust: how long the tools and the
// checks take, and the plain geometry its tools follow. Every length is a
// share of the vertebra's height (the distance between the middles of its
// two endplates) unless it says pixels.

export interface AdjustScript {
  /** How long a hover preview takes. */
  previewMs: number;
  /** How long a wand click takes. */
  wandMs: number;
  /** How long a pin takes. */
  pinMs: number;
  /** How long the case checks take. */
  checkMs: number;
  /** How long comparing with an answer key takes. */
  compareMs: number;
  /** A boundary click splits a vertebra only this far or more from both of its endplates. */
  splitMargin: number;
  /** A boundary click this close to a gap between two vertebrae, or inside it, merges them. */
  mergeReach: number;
  /** A region click this close outside an outline still reaches it. */
  regionReach: number;
  /** A vertebra shorter than this, in film pixels, is too small for the region wand. */
  smallestRegionPx: number;
  /** A split leaves a disc between the two pieces this tall. */
  discShare: number;
  /** When a corner is pinned, the facing corner across the disc moves this share of the way with it. */
  neighbourGive: number;
  /** Where a new piece's pedicles go, across from its left-side edge and down from its upper endplate, as shares of its width and height. */
  pedicleSpot: { across: number; down: number };
}

const adjust: AdjustScript = {
  previewMs: 12,
  wandMs: 60,
  pinMs: 40,
  checkMs: 30,
  compareMs: 40,
  splitMargin: 0.25,
  mergeReach: 0.12,
  regionReach: 0.6,
  smallestRegionPx: 16,
  discShare: 0.1,
  neighbourGive: 0.25,
  pedicleSpot: { across: 0.11, down: 0.32 },
};

export default adjust;
