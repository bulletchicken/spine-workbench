import { SHAPES_FINGERPRINT, type EngineInfo, type FilmPassResult, type Step, type WatchFrame } from '@/shapes';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FromWorker, ToWorker } from '../messages';
import type { FilmDesk } from '../wasm-build/wasm_bridge';
import { defineJob, routeJobs, startWorker, WorkerRoom, type WorkerPort } from './boot';
import type { StepClock } from './step-loop';

const engine = vi.hoisted(() => ({
  initSync: vi.fn(),
  engine_info: vi.fn(),
  last_panic: vi.fn(),
}));
vi.mock('../wasm-build/wasm_bridge', () => engine);

function info(fingerprint: string): EngineInfo {
  return { engineVersion: '0.1.0', shapesFingerprint: fingerprint, caseFormat: 'spine-case', caseVersion: 1 };
}

/** A worker scope that records what the worker posts. */
function fakePort() {
  const posted: FromWorker[] = [];
  const transfers: Transferable[][] = [];
  let listener: ((event: MessageEvent<ToWorker>) => void) | null = null;
  const port: WorkerPort & { closed: boolean } = {
    closed: false,
    postMessage: (message, transfer = []) => {
      posted.push(message);
      transfers.push(transfer);
    },
    addEventListener: (_type, l) => {
      listener = l;
    },
    close() {
      this.closed = true;
    },
  };
  const send = (message: ToWorker) => {
    listener?.({ data: message } as MessageEvent<ToWorker>);
  };
  return { port, posted, transfers, send };
}

const module = {} as WebAssembly.Module;

describe('worker boot', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('starts the engine from the posted module and reports its info', () => {
    engine.engine_info.mockReturnValue(info(SHAPES_FINGERPRINT));
    const { port, posted, send } = fakePort();
    startWorker(port, 'film');
    send({ kind: 'boot', module });
    expect(engine.initSync).toHaveBeenCalledWith({ module });
    expect(posted).toEqual([{ kind: 'ready', info: info(SHAPES_FINGERPRINT) }]);
    expect(port.closed).toBe(false);
  });

  it('refuses an engine built from other shapes', () => {
    engine.engine_info.mockReturnValue(info('stale'));
    const { port, posted, send } = fakePort();
    startWorker(port, 'spine');
    send({ kind: 'boot', module });
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ kind: 'crashed', error: { code: 'engine-out-of-sync' } });
    expect(port.closed).toBe(true);
  });

  it('reports a module that will not start', () => {
    engine.initSync.mockImplementation(() => {
      throw new WebAssembly.LinkError('import missing');
    });
    const { port, posted, send } = fakePort();
    startWorker(port, 'film');
    send({ kind: 'boot', module });
    expect(posted[0]).toMatchObject({ kind: 'crashed', error: { code: 'engine-load-failed' } });
    expect(port.closed).toBe(true);
  });

  it('reports a panic with the message the hook kept', () => {
    engine.engine_info.mockImplementation(() => {
      throw new WebAssembly.RuntimeError('unreachable');
    });
    engine.last_panic.mockReturnValue('panicked at src/exports/info.rs:1:1');
    const { port, posted, send } = fakePort();
    startWorker(port, 'film');
    send({ kind: 'boot', module });
    expect(posted).toEqual([
      {
        kind: 'crashed',
        error: { code: 'engine-crashed', message: 'the engine stopped', detail: 'panicked at src/exports/info.rs:1:1' },
      },
    ]);
  });

  it('answers a job it has no handler for with engine-out-of-sync', () => {
    const { port, posted, send } = fakePort();
    startWorker(port, 'spine');
    send({ kind: 'run', id: 7, job: 'check-case', input: null });
    expect(posted).toEqual([
      {
        kind: 'failed',
        id: 7,
        error: { code: 'engine-out-of-sync', message: 'the spine worker has no handler for this job', detail: 'check-case' },
      },
    ]);
  });
});

/** Lets the worker's queued jobs and slices run. */
async function settle(): Promise<void> {
  for (let i = 0; i < 20; i += 1) await new Promise((resolve) => setTimeout(resolve, 0));
}

/** A clock each slice moves on by 5 ms. */
function sliceClock(): StepClock & { at: number } {
  const clock = { at: 0, now: () => clock.at, pause: () => Promise.resolve() };
  return clock;
}

/** A film desk stand-in for the room: it only records being freed. */
function fakeDesk(name: string) {
  const desk = {
    name,
    freed: false,
    free: () => {
      desk.freed = true;
    },
  };
  return desk as unknown as FilmDesk & { name: string; freed: boolean };
}

/** Throws a plain object, as the engine's exports do. */
function throwPlain(value: unknown): never {
  throw value;
}

function stage(frame: number): WatchFrame {
  return { kind: 'stage-started', film: 'front', stage: frame === 0 ? 'evidence' : 'centre-line' };
}

/** A film pass of `slices` slices, one stage-started frame per slice, each slice 5 ms. */
function filmPass(clock: { at: number }, slices: number, record: { freed: number; advanced: number }) {
  return defineJob('film-pass', {
    start: () => ({
      advance(): Step {
        record.advanced += 1;
        clock.at += 5;
        return { done: record.advanced === slices, stage: 'evidence', fraction: record.advanced / slices, frames: [stage(record.advanced - 1)] };
      },
      finish: (): FilmPassResult => ({ film: 'front', strip: new Uint8Array([1, 2, 3]), scaleAssumed: false, warnings: [] }),
      free: () => {
        record.freed += 1;
      },
    }),
  });
}

function bootedWorker(files: Record<string, unknown>, clock: StepClock = sliceClock()) {
  engine.engine_info.mockReturnValue(info(SHAPES_FINGERPRINT));
  const worker = fakePort();
  startWorker(worker.port, 'film', files, { clock });
  worker.send({ kind: 'boot', module });
  worker.posted.length = 0;
  worker.transfers.length = 0;
  return worker;
}

describe('job routing', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('runs a job from its file and hands its buffers over', async () => {
    const pixels = new Uint16Array([0x3c00, 0x3800]);
    const { posted, transfers, send } = bootedWorker({
      './jobs/display-pixels.ts': { default: defineJob('display-pixels', { run: () => pixels }) },
    });
    send({ kind: 'run', id: 3, job: 'display-pixels', input: null });
    await settle();
    expect(posted).toEqual([{ kind: 'done', id: 3, output: pixels }]);
    expect(transfers).toEqual([[pixels.buffer]]);
  });

  it('runs a stepped job slice by slice, then answers', async () => {
    const clock = sliceClock();
    const record = { freed: 0, advanced: 0 };
    const { posted, send } = bootedWorker({ './jobs/film-pass.ts': { default: filmPass(clock, 3, record) } }, clock);
    send({ kind: 'run', id: 1, job: 'film-pass', input: { film: {} } });
    await settle();
    expect(posted.map((message) => message.kind)).toEqual(['frame', 'progress', 'frame', 'frame', 'done']);
    expect(posted[1]).toEqual({ kind: 'progress', id: 1, stage: 'evidence', fraction: 1 / 3 });
    expect(posted.at(-1)).toMatchObject({ kind: 'done', id: 1, output: { film: 'front', strip: new Uint8Array([1, 2, 3]) } });
    expect(record.freed).toBe(0);
  });

  it('stops a cancelled pass between slices and frees it', async () => {
    const clock: StepClock & { at: number } = { at: 0, now: () => clock.at, pause: () => new Promise((resolve) => setTimeout(resolve, 0)) };
    const record = { freed: 0, advanced: 0 };
    const { posted, send } = bootedWorker({ './jobs/film-pass.ts': { default: filmPass(clock, 1000, record) } }, clock);
    send({ kind: 'run', id: 4, job: 'film-pass', input: { film: {} } });
    await new Promise((resolve) => setTimeout(resolve, 0));
    send({ kind: 'cancel', id: 4 });
    await settle();
    expect(record.advanced).toBeLessThan(1000);
    expect(record.freed).toBe(1);
    expect(posted.at(-1)).toEqual({ kind: 'failed', id: 4, error: { code: 'pass-cancelled', message: 'the pass was cancelled', detail: 'film-pass' } });
    expect(posted.filter((message) => message.kind === 'done' || message.kind === 'failed')).toHaveLength(1);
  });

  it('ignores a cancel that comes after the answer', async () => {
    const { posted, send } = bootedWorker({ './jobs/sense-geometry.ts': { default: defineJob('sense-geometry', { run: () => ({}) as never }) } });
    send({ kind: 'run', id: 9, job: 'sense-geometry', input: null });
    await settle();
    send({ kind: 'cancel', id: 9 });
    await settle();
    expect(posted).toEqual([{ kind: 'done', id: 9, output: {} }]);
  });

  it('runs jobs one at a time, in the order they came', async () => {
    const clock = sliceClock();
    const record = { freed: 0, advanced: 0 };
    const { posted, send } = bootedWorker(
      {
        './jobs/film-pass.ts': { default: filmPass(clock, 2, record) },
        './jobs/display-pixels.ts': { default: defineJob('display-pixels', { run: () => new Uint16Array(1) }) },
      },
      clock,
    );
    send({ kind: 'run', id: 1, job: 'film-pass', input: { film: {} } });
    send({ kind: 'run', id: 2, job: 'display-pixels', input: null });
    await settle();
    expect(posted.map((message) => [message.kind, 'id' in message ? message.id : null])).toEqual([
      ['frame', 1],
      ['progress', 1],
      ['frame', 1],
      ['done', 1],
      ['done', 2],
    ]);
  });

  it('answers a job that fails on purpose with its error, and carries on', async () => {
    const refusal = { code: 'scale-too-short', message: 'the ruler is too short', detail: '3 px' } as const;
    const { posted, send } = bootedWorker({
      './jobs/scale-from-ruler.ts': {
        default: defineJob('scale-from-ruler', {
          run: () => throwPlain(refusal),
        }),
      },
      './jobs/display-pixels.ts': { default: defineJob('display-pixels', { run: () => new Uint16Array(1) }) },
    });
    send({ kind: 'run', id: 1, job: 'scale-from-ruler', input: { a: [0, 0], b: [0, 3], mm: 10 } });
    send({ kind: 'run', id: 2, job: 'display-pixels', input: null });
    await settle();
    expect(posted).toEqual([
      { kind: 'failed', id: 1, error: refusal },
      { kind: 'done', id: 2, output: new Uint16Array(1) },
    ]);
  });

  it('reports a panic during a job as a crash, and closes', async () => {
    engine.last_panic.mockReturnValue('panicked at the region wand');
    const { port, posted, send } = bootedWorker({
      './jobs/region-wand.ts': {
        default: defineJob('region-wand', {
          run: () => {
            throw new WebAssembly.RuntimeError('unreachable');
          },
        }),
      },
    });
    send({ kind: 'run', id: 1, job: 'region-wand', input: {} });
    send({ kind: 'run', id: 2, job: 'region-wand', input: {} });
    await settle();
    expect(posted).toEqual([{ kind: 'crashed', error: { code: 'engine-crashed', message: 'the engine stopped', detail: 'panicked at the region wand' } }]);
    expect(port.closed).toBe(true);
  });

  it('holds one desk at a time and frees the one it replaces', () => {
    const room = new WorkerRoom();
    expect(() => room.desk()).toThrow('no film is open in this worker');
    const first = fakeDesk('first');
    const second = fakeDesk('second');
    room.holdDesk(first);
    room.holdDesk(second);
    expect(first.freed).toBe(true);
    expect(room.desk()).toBe(second);
    room.holdDesk(second);
    expect(second.freed).toBe(false);
  });

  it('fails a film job sent before any film is open', async () => {
    const { posted, send } = bootedWorker({
      './jobs/sense-geometry.ts': { default: defineJob('sense-geometry', { run: (_input, room) => room.desk() as never }) },
    });
    send({ kind: 'run', id: 5, job: 'sense-geometry', input: null });
    await settle();
    expect(posted).toEqual([{ kind: 'failed', id: 5, error: { code: 'engine-out-of-sync', message: 'no film is open in this worker', detail: null } }]);
  });

  it('finds what is wrong with job files', () => {
    const { handlers, problems } = routeJobs('film', {
      './jobs/display-pixels.ts': { default: defineJob('display-pixels', { run: () => new Uint16Array(0) }) },
      './jobs/encode-png.ts': { default: defineJob('display-pixels', { run: () => new Uint16Array(0) }) },
      './jobs/check-case.ts': { default: defineJob('check-case', { run: () => [] }) },
      './jobs/film-pass.ts': { default: { job: 'film-pass', run: () => null } },
      './jobs/sense-geometry.ts': { default: () => null },
      './jobs/look-sideways.ts': { default: { job: 'look-sideways', run: () => null } },
    });
    expect([...handlers.keys()]).toEqual(['display-pixels']);
    expect(problems).toEqual([
      './jobs/encode-png.ts: it names the job display-pixels, but a job\'s file is named after its job',
      './jobs/check-case.ts: check-case runs in the spine worker',
      './jobs/film-pass.ts: film-pass is stepped, so it gives start',
      './jobs/sense-geometry.ts: its default export is not made with defineJob',
      './jobs/look-sideways.ts: look-sideways is not in the job table',
    ]);
  });

  it('refuses to start with job files that do not match the job table', () => {
    engine.engine_info.mockReturnValue(info(SHAPES_FINGERPRINT));
    const { port, posted, send } = fakePort();
    startWorker(port, 'spine', { './jobs/region-wand.ts': { default: defineJob('region-wand', { run: () => ({}) as never }) } });
    send({ kind: 'boot', module });
    expect(posted).toEqual([
      {
        kind: 'crashed',
        error: { code: 'engine-out-of-sync', message: "the spine worker's job files do not match the job table", detail: './jobs/region-wand.ts: region-wand runs in the film worker' },
      },
    ]);
    expect(port.closed).toBe(true);
  });

  it('finds nothing wrong with the job files in both workers', () => {
    const film = import.meta.glob(['./film-worker/jobs/*.ts', '!./film-worker/jobs/*.test.ts'], { eager: true });
    const spine = import.meta.glob(['./spine-worker/jobs/*.ts', '!./spine-worker/jobs/*.test.ts'], { eager: true });
    expect(routeJobs('film', film).problems).toEqual([]);
    expect(routeJobs('spine', spine).problems).toEqual([]);
  });
});
