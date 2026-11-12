import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import type { FrameStep } from '@/choreography/one-loop';
import { ReplayPlayer } from './ReplayPlayer';
import { createFindReplay } from './find-replay-clock';

const loop = vi.hoisted(() => ({ step: null as FrameStep | null, leave: vi.fn() }));
vi.mock('@/choreography/one-loop', () => ({ joinLoop: (step: FrameStep) => { loop.step = step; return loop.leave; } }));
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
it('joins the shared loop only while playing, with no keys or visible controls', () => {
  const clock = createFindReplay();
  const host = document.createElement('div');
  const root = createRoot(host);
  act(() => { root.render(<ReplayPlayer clock={clock} />); });
  expect(loop.step).toBeNull();
  act(() => { clock.getState().start({ at: new Map([['front:0', 0.5]]), counted: ['front:0'], last: 0.5, light: {} }, { lead: 700, travel: 4400, hold: 560 }); });
  expect(loop.step).not.toBeNull();
  act(() => { loop.step?.(10000, 10000); });
  expect(loop.leave).toHaveBeenCalledOnce();
  expect(clock.getState().playing).toBe(false);
  expect(host.childNodes.length).toBe(0);
  act(() => { root.unmount(); });
});
