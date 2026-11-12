// Fake workers for the crew's tests, never part of the app.
//
// Each fake worker runs the real worker start-up and job loop
// (workers/boot.ts) with job files written here instead of the engine,
// joined to the crew by message queues the test can hold and release. A
// film is its file's text: opening "front A" makes a desk named "front A",
// and the jobs answer from that name, so a test can tell which film
// answered. Names that start with "bad" cannot be read; names that start
// with "panic" panic when opened, and "panic again" only when opened a
// second time. A region wand click with case version −1 panics. A film
// whose name holds "slow" makes its passes long; one whose name holds
// "mixed" makes them mix frames the step loop may thin with keyframes;
// one whose name holds "no spine" fails its film pass or trained pass. A
// trained pass answers three vertebrae, its names flagged as estimated.

import {
  SHAPES_FINGERPRINT,
  type ChainAnswer,
  type EngineInfo,
  type FilmFacts,
  type FilmKind,
  type FilmMarks,
  type FilmPassResult,
  type GeometryGuess,
  type OpenedCase,
  type Step,
  type TrainedPassResult,
  type WatchFrame,
} from '@/shapes';
import { EngineFailure } from '../engine-client';
import type { FromWorker, ToWorker, WorkerRole } from '../messages';
import type { FilmDesk } from '../wasm-build/wasm_bridge';
import { defineJob, startWorker, type EngineStart, type WorkerPort, type WorkerRoom } from '../workers/boot';
import type { StepClock } from '../workers/step-loop';
import { Crew } from './crew';
import type { SpawnWorker, WorkerEvents, WorkerLike } from './crew-member';
import { RealEngine } from './real-engine';

export const ENGINE_INFO: EngineInfo = { engineVersion: '9.9.9', shapesFingerprint: SHAPES_FINGERPRINT, caseFormat: 'spine-case', caseVersion: 1 };
export const PANIC_MESSAGE = 'panicked at a fake job';

/** The smallest valid WebAssembly module, handed to fake workers that never use it. */
export const EMPTY_MODULE = new WebAssembly.Module(new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]));

/** A film's file: its name as text. */
export function filmFile(name: string): ArrayBuffer {
  const bytes = new TextEncoder().encode(name);
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
}

/** What a desk opened from `filmFile(name)` answers for `displayPixels`. */
export function pixelsOf(name: string): Uint16Array {
  return Uint16Array.from(name, (letter) => letter.charCodeAt(0));
}

const opened = new Set<string>();

class FakeDesk {
  readonly name: string;
  constructor(name: string) {
    this.name = name;
  }
  free(): void {
    // A fake desk holds no engine memory.
  }
}

function deskName(room: WorkerRoom): string {
  return (room.desk() as unknown as FakeDesk).name;
}

function panic(): never {
  throw new WebAssembly.RuntimeError('unreachable');
}

/**
 * The frames slice `slice` (counted from 1) of a fake pass sends: a
 * stage-started frame first, then a warning a slice; a mixed pass sends
 * two frames that may be thinned before each warning.
 */
export function fakePassFrames(film: FilmKind, stage: 'evidence' | 'chain', slice: number, mixed: boolean): WatchFrame[] {
  if (slice === 1) return [{ kind: 'stage-started', film, stage }];
  const warning: WatchFrame = { kind: 'warning', film, stage, warning: { code: 'low-evidence', film, level: null, region: null } };
  if (!mixed) return [warning];
  const thinnable = (part: number): WatchFrame =>
    stage === 'evidence'
      ? { kind: 'evidence', film, stage, map: 'ridge', step: 2 * slice + part, steps: 100, final: false, scaleMm: 2, tile: { width: 1, height: 1, origin: [0, 0], stepAcross: [1, 0], stepDown: [0, 1], encoding: 'unit', data: new Uint8Array([slice]) } }
      : { kind: 'joint-progress', film, stage, start: 0, round: 2 * slice + part, upper: 1, lower: 0 };
  return [thinnable(0), thinnable(1), warning];
}

/** How many slices a fake pass over a film of this name takes. */
export function fakePassSlices(name: string, quick: number): number {
  return name.includes('slow') ? 40 : name.includes('mixed') ? 12 : quick;
}

/** A pass of `slices` slices, sending `fakePassFrames`. */
function fakePass<Output>(film: FilmKind, stage: 'evidence' | 'chain', slices: number, mixed: boolean, finish: () => Output) {
  let done = 0;
  return {
    advance(): Step {
      done += 1;
      return { done: done >= slices, stage, fraction: done / slices, frames: fakePassFrames(film, stage, done, mixed) };
    },
    finish,
    free: () => undefined,
  };
}

export const FILM_JOBS: Record<string, unknown> = {
  'open-film': {
    default: defineJob('open-film', {
      run: (input, room) => {
        const name = new TextDecoder().decode(input.bytes);
        if (name.startsWith('bad')) throw new EngineFailure({ code: 'film-unknown-format', message: 'not a film', detail: name });
        if (name.startsWith('panic again') ? opened.has(name) : name.startsWith('panic')) panic();
        opened.add(name);
        room.holdDesk(new FakeDesk(name) as unknown as FilmDesk);
        return { facts: { kind: input.kind, width: name.length, height: 1 } as FilmFacts, thumbnail: new Uint8Array(input.bytes) };
      },
    }),
  },
  'display-pixels': { default: defineJob('display-pixels', { run: (_input, room) => pixelsOf(deskName(room)) }) },
  'sense-geometry': { default: defineJob('sense-geometry', { run: (_input, room) => ({ name: deskName(room) }) as unknown as GeometryGuess }) },
  'encode-png': { default: defineJob('encode-png', { run: (_input, room) => new TextEncoder().encode(deskName(room)) }) },
  'tool-preview': {
    default: defineJob('tool-preview', {
      run: (input, room) => {
        deskName(room);
        return { action: 'none', caseVersion: input.caseVersion, reason: 'off-spine' };
      },
    }),
  },
  'region-wand': {
    default: defineJob('region-wand', {
      run: (input, room) => {
        if (input.caseVersion === -1) panic();
        deskName(room);
        return { caseVersion: input.caseVersion, marks: input.marks, changed: [], outline: [], growth: [], solverIterations: 0, solverResidual: 0, warnings: [] };
      },
    }),
  },
  'film-pass': {
    default: defineJob('film-pass', {
      start: (input, room) => {
        const name = deskName(room);
        if (name.includes('no spine')) throw new EngineFailure({ code: 'pass-no-spine-found', message: 'no spine', detail: name });
        return fakePass(input.film.kind, 'evidence', fakePassSlices(name, 3), name.includes('mixed'), (): FilmPassResult => ({
          film: input.film.kind,
          strip: new TextEncoder().encode(name),
          scaleAssumed: false,
          warnings: [{ code: 'low-evidence', film: input.film.kind, level: null, region: null }],
        }));
      },
    }),
  },
  'place-points': {
    default: defineJob('place-points', {
      run: (input, room) =>
        ({
          film: input.chain.film,
          vertebrae: [{ id: input.firstVertebraId }, { id: input.firstVertebraId + 1 }],
          landmarks: { c7: null, sacrumTop: null, hips: [] },
          naming: input.chain.naming,
          namedFor: deskName(room),
        }) as unknown as FilmMarks,
    }),
  },
  'trained-pass': {
    default: defineJob('trained-pass', {
      start: (input, room) => {
        const name = deskName(room);
        if (name.includes('no spine')) throw new EngineFailure({ code: 'pass-no-spine-found', message: 'no spine', detail: name });
        const film = input.film.kind;
        return fakePass(film, 'evidence', fakePassSlices(name, 3), name.includes('mixed'), (): TrainedPassResult => ({
          marks: {
            film,
            vertebrae: [0, 1, 2].map((step) => ({ id: input.firstVertebraId + step })),
            landmarks: { c7: null, sacrumTop: null, hips: [] },
            naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: ['estimated'], l5ConfirmedOn: null },
            namedFor: name,
          } as unknown as FilmMarks,
          warnings: [{ code: 'low-evidence', film, level: null, region: null }],
        }));
      },
    }),
  },
};

export const SPINE_JOBS: Record<string, unknown> = {
  'chain-pass': {
    default: defineJob('chain-pass', {
      start: (input) => {
        const names = input.results.map((result) => new TextDecoder().decode(result.strip)).join(' ');
        return fakePass(input.results[0]?.film ?? 'front', 'chain', fakePassSlices(names, 2), names.includes('mixed'), (): ChainAnswer => ({
          films: input.results.map((result) => ({
            film: result.film,
            naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: null, fromBottom: null, flags: [], l5ConfirmedOn: null },
            chain: result.strip,
            warnings: [...result.warnings, { code: 'tortuous', film: result.film, level: 'T5', region: null }],
          })),
          twoFilms: null,
        }));
      },
    }),
  },
  'check-case': {
    default: defineJob('check-case', {
      run: (input) => [{ kind: 'vertebra-missing', severity: 'blocking', film: null, vertebraId: input.nextVertebraId, level: null }],
    }),
  },
  'pack-case': {
    default: defineJob('pack-case', {
      run: (input) => new TextEncoder().encode(input.films.map((film) => `${film.kind}=${new TextDecoder().decode(film.png)}`).join(';')),
    }),
  },
  'unpack-case': {
    default: defineJob('unpack-case', {
      run: (input) => ({ case: { text: new TextDecoder().decode(input.bytes) }, films: [] }) as unknown as OpenedCase,
    }),
  },
};

/** A worker whose job loop is the real one, with the crew's messages and its own in queues. */
export class FakeWorker implements WorkerLike {
  readonly role: WorkerRole;
  /** Every message the crew posted to it. */
  readonly received: ToWorker[] = [];
  /** Every message it posted to the crew, delivered or not. */
  readonly said: FromWorker[] = [];
  terminated = false;
  closed = false;
  /** A silent worker says nothing more, as one stuck in a slice. */
  silent = false;
  readonly #events: WorkerEvents;
  readonly #held: FromWorker[] = [];
  readonly #runWaiters: { job: string; resolve: () => void }[] = [];
  #holdAnswerTo: { job: string; resolve: () => void } | null = null;
  #holding = false;
  #listener: ((event: MessageEvent<ToWorker>) => void) | null = null;

  constructor(role: WorkerRole, events: WorkerEvents, files: Record<string, unknown>, engine: EngineStart, clock: StepClock) {
    this.role = role;
    this.#events = events;
    const port: WorkerPort = {
      postMessage: (message) => {
        if (this.terminated || this.silent) return;
        this.said.push(message);
        const awaited = this.#holdAnswerTo;
        if (awaited !== null && (message.kind === 'done' || message.kind === 'failed') && this.#jobOf(message.id) === awaited.job) {
          this.#holdAnswerTo = null;
          this.#holding = true;
          awaited.resolve();
        }
        if (this.#holding) this.#held.push(message);
        else
          setTimeout(() => {
            this.#deliver(message);
          }, 0);
      },
      addEventListener: (_type, listener) => {
        this.#listener = listener;
      },
      close: () => {
        this.closed = true;
      },
    };
    startWorker(port, role, files, { engine, clock });
  }

  postMessage(message: ToWorker): void {
    if (this.terminated) return;
    this.received.push(message);
    if (message.kind === 'run') {
      for (const waiter of this.#runWaiters.filter((waiting) => waiting.job === message.job)) {
        this.#runWaiters.splice(this.#runWaiters.indexOf(waiter), 1);
        waiter.resolve();
      }
    }
    const copy: ToWorker = message.kind === 'run' ? { ...message, input: structuredClone(message.input) } : message;
    setTimeout(() => {
      if (!this.terminated && !this.closed) this.#listener?.({ data: copy } as MessageEvent<ToWorker>);
    }, 0);
  }

  terminate(): void {
    this.terminated = true;
  }

  /** Resolves when the crew sends it a `job`. */
  sentRun(job: string): Promise<void> {
    return new Promise((resolve) => this.#runWaiters.push({ job, resolve }));
  }

  /** Starts holding what it says from its answer to `job`; resolves once that answer is held. */
  holdAnswerTo(job: string): Promise<void> {
    return new Promise((resolve) => {
      this.#holdAnswerTo = { job, resolve };
    });
  }

  /** From now on, keep what it says until `release`. */
  hold(): void {
    this.#holding = true;
  }

  /** Delivers everything held, at once and in order. */
  release(): void {
    this.#holding = false;
    for (const message of this.#held.splice(0)) this.#deliver(message);
  }

  /** The worker fails outright, as a script error does. */
  fail(detail: string): void {
    this.#events.failed(detail);
  }

  /** The jobs the crew has sent it, by name. */
  jobsRun(): string[] {
    return this.received.flatMap((message) => (message.kind === 'run' ? [message.job] : []));
  }

  #jobOf(id: number): string | undefined {
    for (const message of this.received) if (message.kind === 'run' && message.id === id) return message.job;
    return undefined;
  }

  #deliver(message: FromWorker): void {
    if (!this.terminated) this.#events.message(message);
  }
}

export interface RigSetup {
  /** What the fake engine reports at start-up; a wrong fingerprint makes every worker refuse to start. */
  info?: EngineInfo;
  /** The compiled engine, or why it could not be loaded. */
  engine?: () => Promise<WebAssembly.Module>;
}

/** A crew of fake workers, the real engine over it, and every worker it has started. */
export function fakeCrew(setup: RigSetup = {}) {
  const workers: FakeWorker[] = [];
  const engine: EngineStart = { start: () => undefined, info: () => setup.info ?? ENGINE_INFO, lastPanic: () => PANIC_MESSAGE };
  const clock: StepClock = { now: () => performance.now(), pause: () => new Promise((resolve) => setTimeout(resolve, 0)) };
  const spawn: SpawnWorker = (role, events) => {
    const worker = new FakeWorker(role, events, role === 'film' ? FILM_JOBS : SPINE_JOBS, engine, clock);
    workers.push(worker);
    return worker;
  };
  const crew = new Crew({ spawn, engine: setup.engine ?? (() => Promise.resolve(EMPTY_MODULE)) });
  return {
    crew,
    engine: new RealEngine(crew),
    workers,
    /** The live film workers, oldest first. */
    filmWorkers: () => workers.filter((worker) => worker.role === 'film' && !worker.terminated),
    spine: () => workers.find((worker) => worker.role === 'spine' && !worker.terminated),
  };
}

/** Lets every queued message, job and slice run. */
export async function settle(rounds = 60): Promise<void> {
  for (let i = 0; i < rounds; i += 1) await new Promise((resolve) => setTimeout(resolve, 0));
}

/** The error code a promise rejected with, or 'answered'. */
export async function outcome(promise: Promise<unknown>): Promise<string> {
  try {
    await promise;
    return 'answered';
  } catch (thrown) {
    return thrown instanceof EngineFailure ? thrown.error.code : `threw ${String(thrown)}`;
  }
}
