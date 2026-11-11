// How the stand-in behaves when the case is saved: how long each film's
// PNG and the case file take, so the sheet's saving state can be seen.

export interface SaveScript {
  /** How long one film's PNG takes. */
  encodeMs: number;
  /** How long the case file takes, once every film's PNG is ready. */
  packMs: number;
}

const save: SaveScript = {
  encodeMs: 250,
  packMs: 350,
};

export default save;
