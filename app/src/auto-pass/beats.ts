// The automatic pass as a film of fixed length: its six beats and every
// time the design fixes for them, in milliseconds from the moment the
// pass starts playing. The film runs at this pace whatever the engine's
// speed; the clock holds when the data of the next beat has not arrived.
//
// These are data, not CSS tokens: under reduced motion the clock shows
// the end of each beat instead of shortening them.

export type BeatId = 'evidence' | 'centre-line' | 'unroll' | 'search' | 'lock-in' | 'confidence';

export interface Beat {
  id: BeatId;
  start: number;
  end: number;
}

/** The six beats, in order. Each starts where the one before ends. */
export const BEATS: readonly Beat[] = [
  { id: 'evidence', start: 400, end: 2400 },
  { id: 'centre-line', start: 2400, end: 3700 },
  { id: 'unroll', start: 3700, end: 5400 },
  { id: 'search', start: 5400, end: 9000 },
  { id: 'lock-in', start: 9000, end: 10400 },
  { id: 'confidence', start: 10400, end: 11600 },
];

/** The end of the film: the bench settles into its result. */
export const PASS_END = 11800;

/** The times the design fixes inside the beats. */
export const PASS_TIMES = {
  /** Both films dim from the start until this time. */
  dimmedBy: 450,
  /** Each evidence map wipes down its film from one of these times. */
  wipeStarts: [400, 1060, 1720],
  wipeLasts: 640,
  /** The thread draws neck to pelvis from the centre-line beat's start. */
  threadLasts: 1100,
  /** The thread's white head has faded by this time. */
  threadHeadGoneBy: 3800,
  /** Front and Side glide apart and the lane opens between them. */
  filmsPart: 3350,
  /** The corridor around each spine outlines itself from the unroll's start. */
  corridorLasts: 280,
  /** The spine lifts off in slices, top first, over `slicesSpread`; each flies for `sliceFlies`. */
  slicesFrom: 3980,
  slicesSpread: 400,
  sliceFlies: 980,
  /** Two straight strips stand in the lane. */
  stripsStandBy: 5360,
  /** A barcode wipes in beside each strip. */
  barcodeFrom: 5100,
  barcodeUntil: 5420,
  /** One line sweeps down both strips at constant speed. */
  sweepFrom: 5400,
  sweepUntil: 9000,
  /** Outlines lock from the pelvis up, this far apart. */
  lockFrom: 9150,
  lockGap: 36,
  /** A wrong guess slides to its place. */
  movedSlides: 300,
  /** Names roll in from both ends at once, this far apart. */
  namesFrom: 9800,
  nameGap: 38,
  /** Colour arrives top to bottom, this far apart. */
  colourFrom: 10400,
  colourGap: 34,
  /** The films are back to full brightness. */
  brightAgainFrom: 10400,
  brightAgainBy: 11300,
} as const;

/** How bright Front and Side are while the pass plays. */
export const FILM_LIGHT_DURING = 0.58;
/** How bright the bending stack is while the pass plays. */
export const STACK_LIGHT_DURING = 0.38;

/** The beat playing at time t, or null before the first and after the last. */
export function beatAt(t: number): Beat | null {
  return BEATS.find((beat) => t >= beat.start && t < beat.end) ?? null;
}

/** How bright Front and Side are at time t: 1 is full. */
export function filmLightAt(t: number): number {
  return lightAt(t, FILM_LIGHT_DURING);
}

/** How bright the bending films are at time t: 1 is full. */
export function stackLightAt(t: number): number {
  return lightAt(t, STACK_LIGHT_DURING);
}

function lightAt(t: number, during: number): number {
  const dim = 1 - during;
  return 1 - dim * ramp(t, 0, PASS_TIMES.dimmedBy) + dim * ramp(t, PASS_TIMES.brightAgainFrom, PASS_TIMES.brightAgainBy);
}

/** 0 before `from`, 1 after `to`, straight in between. */
function ramp(t: number, from: number, to: number): number {
  return Math.min(1, Math.max(0, (t - from) / (to - from)));
}
