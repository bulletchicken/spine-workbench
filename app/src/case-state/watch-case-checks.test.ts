import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CaseFileV1, CheckFinding } from '@/shapes';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { createCaseStore } from './case-store';
import { sketchCase } from './sketch-case';
import { CHECK_DELAY_MS, watchCaseChecks, type CaseChecks } from './watch-case-checks';

const missing: CheckFinding = { kind: 'l5-unconfirmed', severity: 'warning', film: 'front', vertebraId: null, level: null };

/** An engine whose checks are answered by hand: each call waits until the test settles it. */
function handEngine() {
  const calls: { c: CaseFileV1; answer(findings: CheckFinding[]): void; refuse(code: 'pass-cancelled' | 'engine-crashed'): void }[] = [];
  const checkCase = vi.fn(
    (c: CaseFileV1) =>
      new Promise<CheckFinding[]>((resolve, reject) => {
        calls.push({
          c,
          answer: resolve,
          refuse: (code) => {
            reject(new EngineFailure({ code, message: 'refused', detail: null }));
          },
        });
      }),
  );
  return { engine: { checkCase } as unknown as EngineClient, checkCase, calls };
}

function watched() {
  const store = createCaseStore(sketchCase(['front']));
  const { engine, checkCase, calls } = handEngine();
  const seen: CaseChecks[] = [];
  const stop = watchCaseChecks(store, engine, (checks) => seen.push(checks));
  const flip = () => store.getState().apply([{ kind: 'flip-brightness', film: 'front' }]);
  return { store, checkCase, calls, seen, stop, flip };
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('watchCaseChecks', () => {
  it('checks the case once it has been still for the delay, and says which version the findings describe', async () => {
    const { checkCase, calls, seen, store } = watched();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS - 1);
    expect(checkCase).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(calls[0]?.c).toBe(store.getState().case);
    calls[0]?.answer([missing]);
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toEqual([{ findings: [missing], checkedAt: 0, fault: null }]);
  });

  it('checks changes that come close together once', async () => {
    const { checkCase, flip } = watched();
    for (let change = 0; change < 3; change += 1) {
      flip();
      await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS / 2);
    }
    expect(checkCase).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS / 2);
    expect(checkCase).toHaveBeenCalledTimes(1);
  });

  it('drops findings for a version the case has left, and shows the next check', async () => {
    const { calls, seen, flip } = watched();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS);
    flip();
    calls[0]?.answer([missing]);
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toEqual([]);
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS);
    calls[1]?.answer([]);
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toEqual([{ findings: [], checkedAt: 1, fault: null }]);
  });

  it('lets a replaced check go by, and reports another failure with the findings it had', async () => {
    const { calls, seen, flip } = watched();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS);
    calls[0]?.answer([missing]);
    await vi.advanceTimersByTimeAsync(0);
    flip();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS);
    calls[1]?.refuse('pass-cancelled');
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toHaveLength(1);
    flip();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS);
    calls[2]?.refuse('engine-crashed');
    await vi.advanceTimersByTimeAsync(0);
    expect(seen.at(-1)).toEqual({ findings: [missing], checkedAt: 0, fault: { code: 'engine-crashed', message: 'refused', detail: null } });
  });

  it('asks nothing more once stopped, and drops an answer that arrives after', async () => {
    const { checkCase, calls, seen, stop, flip } = watched();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS);
    stop();
    calls[0]?.answer([missing]);
    flip();
    await vi.advanceTimersByTimeAsync(CHECK_DELAY_MS * 2);
    expect(checkCase).toHaveBeenCalledTimes(1);
    expect(seen).toEqual([]);
  });
});
