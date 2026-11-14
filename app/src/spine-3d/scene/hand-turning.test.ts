// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import type { FrameStep } from '@/choreography/one-loop';
import { attachHandTurning } from './hand-turning';
import type { Stage } from './stage';
const loop = vi.hoisted(() => new Set<FrameStep>());
vi.mock('@/choreography/one-loop', () => ({ joinLoop: (step: FrameStep) => { loop.add(step); return () => { loop.delete(step); }; } }));
afterEach(() => { loop.clear(); });
function setup() {
  const canvas = document.createElement('canvas');
  const captured = new Set<number>();
  canvas.setPointerCapture = (id) => { captured.add(id); };
  canvas.hasPointerCapture = (id) => captured.has(id);
  canvas.releasePointerCapture = (id) => { captured.delete(id); };
  let count = 0;
  const setView = vi.fn(() => { count++; });
  const stage = {
    view: () => ({ azimuth: 0, elevation: 0, perspective: 0, pxPerMm: [1, 1], aim: [0, 0, 0], aimAt: [200, 300] }),
    size: () => ({ width: 400, height: 600 }), bounds: () => ({ low: [0, 0, 0], high: [100, 100, 100] }),
    viewsSet: () => count, setView, pick: () => null, onDrawn: () => () => undefined,
  } as unknown as Stage;
  const pointer = (name: string, x: number, time: number) => {
    const event = new Event(name);
    Object.defineProperties(event, Object.fromEntries(Object.entries({ pointerId: 1, pointerType: 'mouse', button: 0, clientX: x, clientY: 20, timeStamp: time }).map(([key, value]) => [key, { value }])));
    canvas.dispatchEvent(event);
  };
  return { canvas, captured, stage, setView, pointer };
}
it('keeps direct dragging under reduced motion and stops an ordinary glide before its next movement', () => {
  const { canvas, stage, setView, pointer } = setup();
  let reduced = true;
  const hand = attachHandTurning(stage, canvas, { reducedMotion: () => reduced });
  pointer('pointerdown', 0, 0); pointer('pointermove', 30, 20); pointer('pointerup', 30, 21);
  expect(setView).toHaveBeenCalledOnce(); expect(loop.size).toBe(0);
  reduced = false;
  pointer('pointerdown', 0, 30); pointer('pointermove', 30, 50); pointer('pointerup', 30, 51);
  expect(loop.size).toBe(1);
  for (const tick of [...loop]) tick(16, 16);
  const before = setView.mock.calls.length;
  reduced = true;
  for (const tick of [...loop]) tick(16, 32);
  expect(setView).toHaveBeenCalledTimes(before); expect(loop.size).toBe(0);
  hand.detach();
});
it('never glides or chooses on cancellation, and detach releases a held pointer', () => {
  const { canvas, captured, stage, pointer } = setup();
  const choose = vi.fn(); const hand = attachHandTurning(stage, canvas, { onChoose: choose });
  pointer('pointerdown', 0, 0); pointer('pointermove', 30, 20); pointer('pointercancel', 30, 21);
  expect(loop.size).toBe(0); expect(choose).not.toHaveBeenCalled(); expect(captured.size).toBe(0);
  pointer('pointerdown', 0, 30); expect(captured.size).toBe(1);
  hand.detach(); expect(captured.size).toBe(0);
  pointer('pointerup', 0, 40); expect(choose).not.toHaveBeenCalled();
});
