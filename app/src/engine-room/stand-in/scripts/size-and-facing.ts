// How the stand-in behaves at step 2, Size & facing: how long sensing a
// film takes, and the ruler's rules. A ruler's scale is its length in mm
// over its length in pixels.

export interface SizeAndFacingScript {
  /** How long sensing a film's geometry takes. */
  senseMs: number;
  /** How long working out a scale takes. */
  scaleMs: number;
  /** A ruler (or a round object's diameter) shorter than this, in film pixels, is refused. */
  shortestRulerPx: number;
  /** A ruler shorter than this, in film pixels, is usable but weak. */
  weakRulerBelowPx: number;
  /** How far a click can be off, in film pixels, which sets the scale's uncertainty. */
  clickErrorPx: number;
}

const sizeAndFacing: SizeAndFacingScript = {
  senseMs: 500,
  scaleMs: 150,
  shortestRulerPx: 20,
  weakRulerBelowPx: 150,
  clickErrorPx: 1.5,
};

export default sizeAndFacing;
