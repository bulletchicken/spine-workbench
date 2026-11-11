// The jobs waiting for one worker, in the order they were asked for.
//
// A job the job table marks replaceable (hover previews, live checks)
// keeps at most one waiting: the older one leaves the queue and is handed
// back, to be rejected with `pass-cancelled`, and the newer one joins at
// the back like any other. A job already running is never replaced.

import { JOBS, type JobName } from '../job-table';

export class JobQueue<Entry extends { readonly job: JobName }> {
  #waiting: Entry[] = [];

  /** Adds `entry` at the back. Returns the waiting job it replaced, if any. */
  add(entry: Entry): Entry | null {
    let replaced: Entry | null = null;
    if (JOBS[entry.job].replaceable) {
      const older = this.#waiting.findIndex((waiting) => waiting.job === entry.job);
      if (older >= 0) replaced = this.#waiting.splice(older, 1)[0] ?? null;
    }
    this.#waiting.push(entry);
    return replaced;
  }

  /** Puts `entry` before every waiting job: the film a respawned worker must open again. */
  addFirst(entry: Entry): void {
    this.#waiting.unshift(entry);
  }

  /** The next job to run, taken off the queue. */
  take(): Entry | undefined {
    return this.#waiting.shift();
  }

  /** Takes `entry` off the queue. Returns whether it was waiting. */
  remove(entry: Entry): boolean {
    const at = this.#waiting.indexOf(entry);
    if (at < 0) return false;
    this.#waiting.splice(at, 1);
    return true;
  }

  /** Every waiting job, taken off the queue. */
  clear(): Entry[] {
    const all = this.#waiting;
    this.#waiting = [];
    return all;
  }

  get length(): number {
    return this.#waiting.length;
  }

  get waiting(): readonly Entry[] {
    return this.#waiting;
  }
}
