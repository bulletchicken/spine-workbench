// How the stand-in behaves at step 1, Films: reading a film takes a
// moment, so the screen's reading state can be seen, and so does making
// up a spine.

export interface FilmsScript {
  /** How long reading a film takes. */
  openMs: number;
  /** How long the film's pixels for the GPU take. */
  displayPixelsMs: number;
  /** How long making up a spine takes. */
  makeUpMs: number;
}

const films: FilmsScript = {
  openMs: 700,
  displayPixelsMs: 120,
  makeUpMs: 900,
};

export default films;
