import { SHAPES_FINGERPRINT, type EngineError, type RebuildInput } from '@/shapes';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { JOBS } from '../../../job-table';
import type { FromWorker, ToWorker } from '../../../messages';
import { startWorker, type WorkerPort } from '../../boot';
import rebuild from './rebuild-layout';

const bridge = vi.hoisted(() => ({ rebuild_layout: vi.fn(), initSync: vi.fn(), engine_info: vi.fn(), last_panic: vi.fn() }));
vi.mock('../../../wasm-build/wasm_bridge', () => bridge);

// Transport forwards this object without reading its geometry.
const input = { upper: 'T8', lower: 'T9', anchors: [] } as unknown as RebuildInput;

function worker() {
  const posted: FromWorker[] = [];
  let receive: ((event: MessageEvent<ToWorker>) => void) | null = null;
  const port: WorkerPort = {
    postMessage: (message) => { posted.push(message); },
    addEventListener: (_type, listener) => { receive = listener; },
    close: vi.fn(),
  };
  const send = (message: ToWorker) => receive?.({ data: message } as MessageEvent<ToWorker>);
  startWorker(port, 'spine', { './jobs/rebuild-layout.ts': { default: rebuild } });
  return { posted, send };
}

describe('the rebuild job', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    bridge.engine_info.mockReturnValue({ engineVersion: 'test', shapesFingerprint: SHAPES_FINGERPRINT, caseFormat: 'spine-case', caseVersion: 1 });
  });

  it('forwards the whole request and placement on the replaceable spine job', async () => {
    const output = { id: 'changed-by-hand', change: { kind: 'changed-by-hand' } };
    bridge.rebuild_layout.mockReturnValue(output);
    const { posted, send } = worker();
    send({ kind: 'boot', module: {} });
    expect(posted[0]).toMatchObject({ kind: 'ready' });
    send({ kind: 'run', id: 4, job: 'rebuild-layout', input });
    await Promise.resolve();
    await Promise.resolve();
    expect(bridge.rebuild_layout).toHaveBeenCalledWith(input);
    expect(posted).toContainEqual({ kind: 'done', id: 4, output });
    expect(JOBS['rebuild-layout']).toEqual({ worker: 'spine', stepped: false, replaceable: true });
  });

  it('carries the engine refusal through without changing its code or detail', async () => {
    const error: EngineError = { code: 'tool-refused', message: 'rebuild-not-built', detail: 'rebuild-not-built' };
    bridge.rebuild_layout.mockImplementation(() => {
      // The wasm bridge throws serialised data, whose thrown type is unknown.
      throw error as unknown;
    });
    const { posted, send } = worker();
    send({ kind: 'boot', module: {} });
    send({ kind: 'run', id: 9, job: 'rebuild-layout', input });
    await Promise.resolve();
    await Promise.resolve();
    expect(posted).toContainEqual({ kind: 'failed', id: 9, error });
  });
});
