// How an engine worker starts and runs its jobs.
//
// Starting: it waits for the compiled engine, makes an instance with
// `initSync`, checks the engine was built from the same shapes as the
// app, and reports `ready` with the engine's info.
//
// Running: each job has exactly one file, `<worker>/jobs/<job name>.ts`,
// whose default export is made with `defineJob`. The worker entry finds
// them with `import.meta.glob` and hands them to `startWorker`, which
// checks every file against the job table before it reports `ready`. A
// job the worker has no file for is answered `failed` with
// `engine-out-of-sync`.
//
// Jobs run one at a time, in the order they arrive. A plain job runs to
// its answer; a stepped job runs through the step loop, a slice at a time,
// and stops between slices when cancelled. Answers and frames go out with
// their buffers in the transfer list.
//
// A panic reaches here as a WebAssembly trap: the worker posts `crashed`
// with the message the panic hook kept, and closes. The crew replaces it.

import { SHAPES_FINGERPRINT, type EngineError, type EngineInfo } from '@/shapes';
import { EngineFailure } from '../engine-client';
import { JOBS, type JobInput, type JobName, type JobOutput } from '../job-table';
import type { FromWorker, ToWorker, WorkerRole } from '../messages';
import { transferList } from '../payloads';
import { engine_info, initSync, last_panic, type FilmDesk } from '../wasm-build/wasm_bridge';
import { runSteps, workerClock, type StepClock, type Stepper } from './step-loop';

/** The parts of a dedicated worker's global scope that boot uses. */
export interface WorkerPort {
  postMessage(message: FromWorker, transfer?: Transferable[]): void;
  addEventListener(type: 'message', listener: (event: MessageEvent<ToWorker>) => void): void;
  close(): void;
}

/**
 * This worker's own global scope as a WorkerPort. Each part is reached by
 * name, since the network ban never lets the global object itself be
 * passed around.
 */
export function ownScope(): WorkerPort {
  return {
    postMessage: (message, transfer = []) => {
      self.postMessage(message, { transfer });
    },
    addEventListener: (type, listener) => {
      self.addEventListener(type, listener);
    },
    close: () => {
      self.close();
    },
  };
}

/**
 * What a job's handler is handed besides its input: the worker's own
 * state. A film worker holds one film's desk; the spine worker holds
 * nothing.
 */
export class WorkerRoom {
  #desk: FilmDesk | null = null;

  /** The film this worker holds. Asking before one is open fails the job. */
  desk(): FilmDesk {
    if (this.#desk === null) {
      throw new EngineFailure({ code: 'engine-out-of-sync', message: 'no film is open in this worker', detail: null });
    }
    return this.#desk;
  }

  /**
   * Hold `desk` from now on and free the one held before. `open-film`
   * calls it only once the new film has opened, so a film that cannot be
   * read leaves the open one as it was.
   */
  holdDesk(desk: FilmDesk): void {
    const before = this.#desk;
    this.#desk = desk;
    if (before !== null && before !== desk) before.free();
  }
}

/** A job that answers in one go. */
export interface PlainJob<J extends JobName> {
  run(input: JobInput<J>, room: WorkerRoom): JobOutput<J>;
}

/** A job that runs as a pass, a slice at a time. */
export interface SteppedJob<J extends JobName> {
  start(input: JobInput<J>, room: WorkerRoom): Stepper<JobOutput<J>>;
}

/** What a job file default-exports. */
export type JobFile<J extends JobName> = { readonly job: J } & (PlainJob<J> | SteppedJob<J>);

/**
 * A job file's default export: the job's name, as in the job table, and
 * its handler. A stepped job (the table says which) gives `start`, every
 * other job gives `run`.
 */
export function defineJob<J extends JobName>(job: J, handler: PlainJob<J> | SteppedJob<J>): JobFile<J> {
  return { job, ...handler };
}

/** How the worker reaches the engine: the wasm build, unless a test says otherwise. */
export interface EngineStart {
  start(module: WebAssembly.Module): void;
  info(): unknown;
  lastPanic(): string | undefined;
}

const WASM_ENGINE: EngineStart = {
  start: (module) => {
    initSync({ module });
  },
  info: (): unknown => engine_info() as unknown,
  lastPanic: () => last_panic(),
};

export interface WorkerSetup {
  engine?: EngineStart;
  clock?: StepClock;
}

/** A handler as the worker calls it, its types already checked by `defineJob`. */
type Handler =
  | { run(input: unknown, room: WorkerRoom): unknown }
  | { start(input: unknown, room: WorkerRoom): Stepper<unknown> };

/** The handlers found in a worker's job files, and what is wrong with any of them. */
export function routeJobs(role: WorkerRole, files: Record<string, unknown>): { handlers: Map<JobName, Handler>; problems: string[] } {
  const handlers = new Map<JobName, Handler>();
  const problems: string[] = [];
  for (const [path, module] of Object.entries(files)) {
    const named = /([^/]+)\.ts$/.exec(path)?.[1] ?? path;
    const file = typeof module === 'object' && module !== null ? (module as { default?: unknown }).default : undefined;
    if (typeof file !== 'object' || file === null || typeof (file as { job?: unknown }).job !== 'string') {
      problems.push(`${path}: its default export is not made with defineJob`);
      continue;
    }
    const { job } = file as { job: string };
    if (job !== named) {
      problems.push(`${path}: it names the job ${job}, but a job's file is named after its job`);
      continue;
    }
    if (!Object.hasOwn(JOBS, job)) {
      problems.push(`${path}: ${job} is not in the job table`);
      continue;
    }
    const rules = JOBS[job as JobName];
    if (rules.worker !== role) {
      problems.push(`${path}: ${job} runs in the ${rules.worker} worker`);
      continue;
    }
    const plain = typeof (file as { run?: unknown }).run === 'function';
    const stepped = typeof (file as { start?: unknown }).start === 'function';
    if (rules.stepped ? !stepped || plain : !plain || stepped) {
      problems.push(`${path}: ${job} ${rules.stepped ? 'is stepped, so it gives start' : 'is not stepped, so it gives run'}`);
      continue;
    }
    handlers.set(job as JobName, file as Handler);
  }
  return { handlers, problems };
}

type RunMessage = Extract<ToWorker, { kind: 'run' }>;

export function startWorker(port: WorkerPort, role: WorkerRole, files: Record<string, unknown> = {}, setup: WorkerSetup = {}): void {
  const engine = setup.engine ?? WASM_ENGINE;
  const { handlers, problems } = routeJobs(role, files);
  const room = new WorkerRoom();
  const waiting: RunMessage[] = [];
  let state: 'waiting-for-engine' | 'ready' | 'closed' = 'waiting-for-engine';
  let running: { id: number; cancelled: boolean } | null = null;
  let clock: StepClock | null = setup.clock ?? null;

  const post = (message: FromWorker, transfer: Transferable[] = []) => {
    if (state !== 'closed') port.postMessage(message, transfer);
  };

  const crash = (error: EngineError) => {
    post({ kind: 'crashed', error });
    state = 'closed';
    waiting.length = 0;
    port.close();
  };

  /** What an engine call threw, as an EngineError, or null for a trap, after which the worker has crashed. */
  const failureOf = (thrown: unknown): EngineError | null => {
    if (thrown instanceof WebAssembly.RuntimeError) {
      crash({ code: 'engine-crashed', message: 'the engine stopped', detail: engine.lastPanic() ?? thrown.message });
      return null;
    }
    return fromThrown(thrown);
  };

  const runNext = async (): Promise<void> => {
    if (running !== null) return;
    const message = waiting.shift();
    if (message === undefined || state === 'closed') return;
    const job = { id: message.id, cancelled: false };
    running = job;
    try {
      await runJob(message, job);
    } finally {
      running = null;
    }
    await runNext();
  };

  const runJob = async (message: RunMessage, job: { id: number; cancelled: boolean }): Promise<void> => {
    const { id } = message;
    const handler = handlers.get(message.job);
    if (handler === undefined) {
      post({ kind: 'failed', id, error: { code: 'engine-out-of-sync', message: `the ${role} worker has no handler for this job`, detail: message.job } });
      return;
    }
    if (state !== 'ready') {
      post({ kind: 'failed', id, error: { code: 'engine-out-of-sync', message: 'the engine has not started in this worker', detail: message.job } });
      return;
    }
    try {
      if ('run' in handler) {
        const output = handler.run(message.input, room);
        post({ kind: 'done', id, output }, transferList(output));
        return;
      }
      clock ??= workerClock();
      const outcome = await runSteps(
        handler.start(message.input, room),
        {
          progress: (stage, fraction) => {
            post({ kind: 'progress', id, stage, fraction });
          },
          frame: (frame) => {
            post({ kind: 'frame', id, frame }, transferList(frame));
          },
        },
        () => job.cancelled || state === 'closed',
        clock,
      );
      if (outcome.kind === 'cancelled') {
        post({ kind: 'failed', id, error: { code: 'pass-cancelled', message: 'the pass was cancelled', detail: message.job } });
      } else {
        post({ kind: 'done', id, output: outcome.output }, transferList(outcome.output));
      }
    } catch (thrown) {
      const error = failureOf(thrown);
      if (error !== null) post({ kind: 'failed', id, error });
    }
  };

  port.addEventListener('message', ({ data: message }) => {
    if (state === 'closed') return;
    switch (message.kind) {
      case 'boot':
        if (state !== 'waiting-for-engine') return;
        boot(message.module);
        return;
      case 'run':
        waiting.push(message);
        void runNext();
        return;
      case 'cancel': {
        if (running?.id === message.id) {
          running.cancelled = true;
          return;
        }
        const queued = waiting.findIndex((run) => run.id === message.id);
        if (queued >= 0) {
          const [dropped] = waiting.splice(queued, 1);
          if (dropped !== undefined) {
            post({ kind: 'failed', id: dropped.id, error: { code: 'pass-cancelled', message: 'the job was cancelled before it ran', detail: dropped.job } });
          }
        }
        return;
      }
    }
  });

  function boot(module: WebAssembly.Module): void {
    let info: EngineInfo;
    try {
      engine.start(module);
    } catch (cause) {
      crash({ code: 'engine-load-failed', message: 'the engine could not start', detail: String(cause) });
      return;
    }
    try {
      info = engine.info() as EngineInfo;
    } catch (cause) {
      const error = failureOf(cause);
      if (error !== null) crash(error);
      return;
    }
    if (info.shapesFingerprint !== SHAPES_FINGERPRINT) {
      crash({
        code: 'engine-out-of-sync',
        message: 'the engine was built from different shapes than the app',
        detail: `engine ${info.shapesFingerprint}, app ${SHAPES_FINGERPRINT}`,
      });
      return;
    }
    if (problems.length > 0) {
      crash({ code: 'engine-out-of-sync', message: `the ${role} worker's job files do not match the job table`, detail: problems.join('; ') });
      return;
    }
    state = 'ready';
    post({ kind: 'ready', info });
  }
}

/** What an engine call threw, when it was not a trap, as an EngineError. */
function fromThrown(thrown: unknown): EngineError {
  if (thrown instanceof EngineFailure) return thrown.error;
  if (isEngineError(thrown)) return thrown;
  return { code: 'engine-crashed', message: 'the engine threw something unexpected', detail: String(thrown) };
}

function isEngineError(value: unknown): value is EngineError {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as Partial<EngineError>).code === 'string' &&
    typeof (value as Partial<EngineError>).message === 'string'
  );
}
