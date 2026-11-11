// How the stand-in behaves at step 3, Find: how long the automatic pass
// takes to send the story's frames. In the normal pace it finishes well
// before the sweep animation does (11.8 seconds); in the slow pace, which
// `?slow` picks in development, it is still sending frames after the
// animation would have ended, so the player's waiting can be seen.

export interface FindScript {
  /** The whole pass, at the normal pace. */
  normalMs: number;
  /** The whole pass, at the slow pace. */
  slowMs: number;
  /** The share of the pass the chain pass takes; the film passes take the rest, side by side. */
  chainShare: number;
  /** How long placing one film's points takes. */
  placePointsMs: number;
}

const find: FindScript = {
  normalMs: 6000,
  slowMs: 24000,
  chainShare: 0.4,
  placePointsMs: 60,
};

export default find;
