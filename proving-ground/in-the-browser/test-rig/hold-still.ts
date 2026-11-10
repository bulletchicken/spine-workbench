// Waits until what a test watches stops changing, so a request or a
// violation started late (on a timer, after a worker's first answer) is
// counted before the test reads its records.

/** How long the reading must stay the same, in milliseconds. */
export const QUIET_FOR_MS = 1_000;

const POLL_EVERY_MS = 100;
const GIVE_UP_AFTER_MS = 15_000;

/**
 * Resolves once `read` has given the same answer for `QUIET_FOR_MS`;
 * throws if it is still changing after fifteen seconds. Anything that
 * starts later than the quiet window is not waited for.
 */
export async function holdStill(read: () => Promise<string>): Promise<void> {
  const start = Date.now();
  let last = await read();
  let since = Date.now();
  while (Date.now() - since < QUIET_FOR_MS) {
    if (Date.now() - start > GIVE_UP_AFTER_MS) {
      throw new Error(`still changing after ${String(GIVE_UP_AFTER_MS)} ms: ${last}`);
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_EVERY_MS));
    const now = await read();
    if (now !== last) {
      last = now;
      since = Date.now();
    }
  }
}
