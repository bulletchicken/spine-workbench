import { SHAPES_FINGERPRINT, type EngineInfo } from '@/shapes';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FromWorker, ToWorker } from '../messages';
import { startWorker, type WorkerPort } from './boot';

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
  let listener: ((event: MessageEvent<ToWorker>) => void) | null = null;
  const port: WorkerPort & { closed: boolean } = {
    closed: false,
    postMessage: (message) => posted.push(message),
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
  return { port, posted, send };
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
