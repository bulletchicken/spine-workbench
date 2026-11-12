// Runs a stepped pass (`film-pass`, `chain-pass`) inside a worker, a
// slice at a time.
//
// The engine has no clock. A pass is a state machine whose
// `advance(workUnits)` does a bounded slice of work; this loop times
// each slice with the worker's clock and scales the work units so a
// slice takes about 8 ms, then pauses so a `cancel` message can land
// before the next one.
//
// What it sends, by the rules in docs/contracts/worker-messages.md:
// - progress after a slice, at most ten times a second;
// - every frame, except the two kinds that may be thinned
//   (`mayBeThinned`), which go out at most thirty times a second with the
//   newest kept. A thinned frame held back is sent or dropped before any
//   later frame is sent, so frames go out in the order they were made.
//
// The main thread counts a pass as stuck when no message about it has
// come for the stuck-pass limit (`STUCK_PASS_MS` in the crew), and
// replaces the worker. Progress after every
// slice that ends at least 100 ms after the last is what tells it the
// pass is alive.

import type { Stage, Step, WatchFrame } from '@/shapes';

/** A pass as the engine runs it: a slice at a time, then its answer. */
export interface Stepper<Output> {
  advance(workUnits: number): Step;
  finish(): Output;
  free(): void;
}

/** Where a pass's progress and frames go. */
export interface StepReport {
  progress(stage: Stage, fraction: number): void;
  frame(frame: WatchFrame): void;
}

/** The worker's clock, and how it lets other messages in between slices. */
export interface StepClock {
  now(): number;
  pause(): Promise<void>;
}

export type StepOutcome<Output> = { kind: 'done'; output: Output } | { kind: 'cancelled' };

/** How long a slice should take. */
export const SLICE_MS = 8;
/** The shortest gap between two progress messages. */
export const PROGRESS_GAP_MS = 100;
/** The shortest gap between two frames of the kinds that may be thinned. */
export const THINNED_GAP_MS = 1000 / 30;
/** The work units of the first slice. */
export const FIRST_WORK_UNITS = 1;
/** The most work units one slice is ever given. */
const MOST_UNITS = 2 ** 30;

/**
 * Whether a frame may be thinned: `joint-progress`, and `evidence` that is
 * not the finished map and does not show the wavefront. Every other frame
 * is always sent (the Rust twin is `WatchFrame::is_keyframe`).
 */
export function mayBeThinned(frame: WatchFrame): boolean {
  return frame.kind === 'joint-progress' || (frame.kind === 'evidence' && !frame.final && frame.map !== 'wavefront');
}

/**
 * The work units for the next slice, from how long `units` took: scaled
 * toward `SLICE_MS`, but never more than double or less than a quarter of
 * the last, so one odd slice cannot swing the next far.
 */
export function nextWorkUnits(units: number, elapsedMs: number): number {
  const lowest = Math.max(1, Math.floor(units / 4));
  const highest = Math.min(MOST_UNITS, units * 2);
  if (elapsedMs <= 0) return highest;
  return Math.min(highest, Math.max(lowest, Math.round((units * SLICE_MS) / elapsedMs)));
}

/**
 * Runs `stepper` to its answer, or until `cancelled()` says to stop, when
 * it frees the pass. An error from the engine is thrown on, after the
 * pass is freed.
 */
export async function runSteps<Output>(
  stepper: Stepper<Output>,
  report: StepReport,
  cancelled: () => boolean,
  clock: StepClock,
): Promise<StepOutcome<Output>> {
  const frames = new FrameGate(report);
  let units = FIRST_WORK_UNITS;
  let lastProgress = Number.NEGATIVE_INFINITY;
  try {
    for (;;) {
      if (cancelled()) {
        stepper.free();
        return { kind: 'cancelled' };
      }
      const started = clock.now();
      const step = stepper.advance(units);
      const ended = clock.now();
      for (const frame of step.frames) frames.offer(frame, ended);
      frames.release(ended);
      if (step.done) {
        frames.end(ended);
        return { kind: 'done', output: stepper.finish() };
      }
      if (ended - lastProgress >= PROGRESS_GAP_MS) {
        report.progress(step.stage, step.fraction);
        lastProgress = ended;
      }
      units = nextWorkUnits(units, ended - started);
      await clock.pause();
    }
  } catch (thrown) {
    freeQuietly(stepper);
    throw thrown;
  }
}

/** The worker's own clock: `performance.now()`, and a pause through a MessageChannel. */
export function workerClock(): StepClock {
  const channel = new MessageChannel();
  let wake: (() => void) | null = null;
  channel.port1.onmessage = () => {
    const resume = wake;
    wake = null;
    resume?.();
  };
  return {
    now: () => performance.now(),
    pause: () =>
      new Promise<void>((resolve) => {
        wake = resolve;
        channel.port2.postMessage(null);
      }),
  };
}

/** Lets keyframes straight through and thins the rest, newest kept. */
class FrameGate {
  readonly #report: StepReport;
  #held: WatchFrame | null = null;
  #lastThinned = Number.NEGATIVE_INFINITY;

  constructor(report: StepReport) {
    this.#report = report;
  }

  offer(frame: WatchFrame, now: number): void {
    if (!mayBeThinned(frame)) {
      this.release(now);
      this.#held = null;
      this.#report.frame(frame);
      return;
    }
    if (this.#held === null && now - this.#lastThinned >= THINNED_GAP_MS) {
      this.#send(frame, now);
    } else {
      this.#held = frame;
      this.release(now);
    }
  }

  /** Sends the held frame if its turn has come. */
  release(now: number): void {
    if (this.#held !== null && now - this.#lastThinned >= THINNED_GAP_MS) {
      const held = this.#held;
      this.#held = null;
      this.#send(held, now);
    }
  }

  /** At the end of the pass: the held frame goes if its turn has come, and is dropped if not. */
  end(now: number): void {
    this.release(now);
    this.#held = null;
  }

  #send(frame: WatchFrame, now: number): void {
    this.#lastThinned = now;
    this.#report.frame(frame);
  }
}

function freeQuietly(stepper: Stepper<unknown>): void {
  try {
    stepper.free();
  } catch {
    // An engine that has just failed may have nothing left to free.
  }
}
