import { SHAPES_FINGERPRINT, type ReadingsInput } from '@/shapes';
import { describe, expect, it, vi } from 'vitest';
import { JOBS } from '../../../job-table';
import type { FromWorker, ToWorker } from '../../../messages';
import { startWorker, type WorkerPort } from '../../boot';
import measurement from './measure-spine';

const bridge = vi.hoisted(() => ({ measure_spine: vi.fn(), initSync: vi.fn(), engine_info: vi.fn(), last_panic: vi.fn() }));
vi.mock('../../../wasm-build/wasm_bridge', () => bridge);

// Transport deliberately knows no geometry: it forwards this object intact.
const input = { versions: 32, seed: 7 } as ReadingsInput;

function worker() {
  const posted: FromWorker[] = [];
  let receive: ((event: MessageEvent<ToWorker>) => void) | null = null;
  const port: WorkerPort = {
    postMessage: (message) => { posted.push(message); },
    addEventListener: (_type, listener) => { receive = listener; },
    close: vi.fn(),
  };
  const send = (message: ToWorker) => receive?.({ data: message } as MessageEvent<ToWorker>);
  startWorker(port, 'spine', { './jobs/measure-spine.ts': { default: measurement } });
  return { posted, send };
}

describe('the measurement job', () => {
  it('boots on the spine worker and forwards the exact input and output', async () => {
    const output = { tier: 'front-film-only', versions: 32 };
    bridge.engine_info.mockReturnValue({ engineVersion: 'test', shapesFingerprint: SHAPES_FINGERPRINT, caseFormat: 'spine-case', caseVersion: 1 });
    bridge.measure_spine.mockReturnValue(output);
    const { posted, send } = worker();
    send({ kind: 'boot', module: {} });
    expect(posted[0]).toMatchObject({ kind: 'ready' });
    send({ kind: 'run', id: 4, job: 'measure-spine', input });
    await Promise.resolve();
    await Promise.resolve();
    expect(bridge.measure_spine).toHaveBeenCalledWith(input);
    expect(posted).toContainEqual({ kind: 'done', id: 4, output });
    expect(JOBS['measure-spine']).toEqual({ worker: 'spine', stepped: false, replaceable: true });
  });
});
