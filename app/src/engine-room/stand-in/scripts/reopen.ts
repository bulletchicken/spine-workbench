// How the stand-in behaves when a saved case is reopened: how long
// reading and checking the case file takes, so each line of the strict
// reading can be seen to tick.

export interface ReopenScript {
  /** How long reading and checking a case file takes. */
  unpackMs: number;
}

const reopen: ReopenScript = {
  unpackMs: 400,
};

export default reopen;
