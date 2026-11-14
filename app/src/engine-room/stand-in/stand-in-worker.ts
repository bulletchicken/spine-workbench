// A worker in the stand-in's crew, in the page itself: it speaks the
// worker protocol (messages.ts) exactly as a real engine worker does, one
// job at a time and in the order sent, but answers from the story after
// the delay its script names. Its passes send the story's frames through
// the real step loop, a frame at a time and the script's gap between, so
// frames are thinned and cancelled by the same rules as the engine's. A
// gap longer than `HEARTBEAT_MS` is spent in empty slices, whose progress
// tells the crew the pass is alive however slow the script makes it.
//
// It copies every input on the way in and every answer on the way out,
// as postMessage would, so nothing the screens hold is shared with it.
// An input it cannot read is refused with `engine-out-of-sync`, as the
// engine refuses a value that does not match the shapes.

import { CASE_FORMAT, CASE_VERSION, SHAPES_FINGERPRINT, type EngineError, type Step, type WatchFrame } from '@/shapes';
import type { WorkerEvents, WorkerLike } from '../crew/crew-member';
import { EngineFailure } from '../engine-client';
import type { JobInput, JobName, JobOutput } from '../job-table';
import type { FromWorker, ToWorker, WorkerRole } from '../messages';
import { runSteps, type Stepper } from '../workers/step-loop';

/** The longest a pass goes without a slice, so the crew never takes a slow script for a stuck pass. */
export const HEARTBEAT_MS = 400;

/** A job the stand-in answers after a pause. */
export interface StandInAnswer<Output> {
  kind: 'answer';
  delayMs: number;
  answer(): Output;
}

/** A pass the stand-in plays: its frames one at a time, `gapMs` apart, then its answer one gap after the last, so n frames take n gaps. */
export interface StandInPass<Output> {
  kind: 'pass';
  frames: WatchFrame[];
  gapMs: number;
  answer(): Output;
}

/** How the stand-in answers each of a worker's jobs, given its input and the worker's room, typed by the job table. */
export type StandInJobs<Room> = {
  [J in JobName]?: (input: JobInput<J>, room: Room) => StandInAnswer<JobOutput<J>> | StandInPass<JobOutput<J>>;
};

type AnyHandler<Room> = (input: unknown, room: Room) => StandInAnswer<unknown> | StandInPass<unknown>;

type RunMessage = Extract<ToWorker, { kind: 'run' }>;

export class StandInWorker<Room> implements WorkerLike {
  readonly #role: WorkerRole;
  readonly #events: WorkerEvents;
  readonly #jobs: StandInJobs<Room>;
  readonly #room: Room;
  readonly #waiting: RunMessage[] = [];
  readonly #timers = new Set<ReturnType<typeof setTimeout>>();
  #running: { id: number; cancelled: boolean } | null = null;
  #ready = false;
  #ended = false;

  constructor(role: WorkerRole, events: WorkerEvents, jobs: StandInJobs<Room>, room: Room) {
    this.#role = role;
    this.#events = events;
    this.#jobs = jobs;
    this.#room = room;
  }

  postMessage(message: ToWorker): void {
    if (this.#ended) return;
    switch (message.kind) {
      case 'boot':
        void this.#after(0).then(() => {
          if (this.#ready) return;
          this.#ready = true;
          this.#say({ kind: 'ready', info: { engineVersion: 'stand-in', shapesFingerprint: SHAPES_FINGERPRINT, caseFormat: CASE_FORMAT, caseVersion: CASE_VERSION, trainedSideFinder: false } });
          void this.#next();
        });
        return;
      case 'run':
        this.#waiting.push({ ...message, input: structuredClone(message.input) });
        void this.#next();
        return;
      case 'cancel': {
        if (this.#running?.id === message.id) {
          this.#running.cancelled = true;
          return;
        }
        const at = this.#waiting.findIndex((run) => run.id === message.id);
        if (at >= 0) {
          const [dropped] = this.#waiting.splice(at, 1);
          if (dropped !== undefined) this.#failed(dropped.id, { code: 'pass-cancelled', message: 'the job was cancelled before it ran', detail: dropped.job });
        }
        return;
      }
    }
  }

  terminate(): void {
    this.#ended = true;
    for (const timer of this.#timers) clearTimeout(timer);
    this.#timers.clear();
  }

  async #next(): Promise<void> {
    if (!this.#ready || this.#running !== null || this.#ended) return;
    const message = this.#waiting.shift();
    if (message === undefined) return;
    const job = { id: message.id, cancelled: false };
    this.#running = job;
    try {
      await this.#run(message, job);
    } finally {
      this.#running = null;
    }
    await this.#next();
  }

  async #run(message: RunMessage, job: { id: number; cancelled: boolean }): Promise<void> {
    const { id } = message;
    // The input crossed as a plain object; a handler that cannot read it throws, and the job is refused.
    const handler = this.#jobs[message.job] as AnyHandler<Room> | undefined;
    if (handler === undefined) {
      this.#failed(id, { code: 'engine-out-of-sync', message: `the ${this.#role} worker has no handler for this job`, detail: message.job });
      return;
    }
    try {
      const plan = handler(message.input, this.#room);
      if (plan.kind === 'answer') {
        await this.#after(plan.delayMs);
        if (this.#ended) return;
        this.#say({ kind: 'done', id, output: structuredClone(plan.answer()) });
        return;
      }
      const ticks = Math.max(1, Math.ceil(plan.gapMs / HEARTBEAT_MS));
      const outcome = await runSteps(
        playFrames(plan, ticks),
        {
          progress: (stage, fraction) => {
            this.#say({ kind: 'progress', id, stage, fraction });
          },
          frame: (frame) => {
            this.#say({ kind: 'frame', id, frame: structuredClone(frame) });
          },
        },
        () => job.cancelled || this.#ended,
        { now: () => performance.now(), pause: () => this.#after(plan.gapMs / ticks) },
      );
      if (this.#ended) return;
      if (outcome.kind === 'cancelled') this.#failed(id, { code: 'pass-cancelled', message: 'the pass was cancelled', detail: message.job });
      else this.#say({ kind: 'done', id, output: structuredClone(outcome.output) });
    } catch (thrown) {
      if (this.#ended) return;
      this.#failed(
        id,
        thrown instanceof EngineFailure ? thrown.error : { code: 'engine-out-of-sync', message: 'the stand-in could not read this input', detail: String(thrown) },
      );
    }
  }

  #failed(id: number, error: EngineError): void {
    this.#say({ kind: 'failed', id, error });
  }

  #say(message: FromWorker): void {
    if (!this.#ended) this.#events.message(message);
  }

  #after(ms: number): Promise<void> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        this.#timers.delete(timer);
        resolve();
      }, ms);
      this.#timers.add(timer);
    });
  }
}

/**
 * A pass that hands out one frame every `ticks` slices, empty slices
 * between, then its answer `ticks` slices after the last (or after the
 * first `ticks` slices when it has no frames); its fraction runs through
 * each stage's frames.
 */
function playFrames(pass: StandInPass<unknown>, ticks: number): Stepper<unknown> {
  let slice = 0;
  let sent = 0;
  let last: Step = { done: false, stage: pass.frames[0]?.stage ?? 'evidence', fraction: 0, frames: [] };
  return {
    advance(): Step {
      slice += 1;
      if ((slice - 1) % ticks !== 0) return last;
      const frame = pass.frames[sent];
      sent += 1;
      if (frame === undefined) return sent > Math.max(1, pass.frames.length) ? { ...last, done: true } : last;
      const ofStage = pass.frames.filter((each) => each.stage === frame.stage);
      const through = pass.frames.slice(0, sent).filter((each) => each.stage === frame.stage).length;
      last = { done: false, stage: frame.stage, fraction: through / ofStage.length, frames: [] };
      return { done: false, stage: frame.stage, fraction: last.fraction, frames: [frame] };
    },
    finish: () => pass.answer(),
    free: () => undefined,
  };
}
