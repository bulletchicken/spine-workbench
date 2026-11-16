// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import type { FrameStep } from '@/choreography/one-loop';
import { attachHandTurning } from './hand-turning';
import type { Stage } from './stage';
const loop = vi.hoisted(() => new Set<FrameStep>());
const fitted = vi.hoisted(() => ({ calls: 0 }));
vi.mock('../looking/framing', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../looking/framing')>();
  return { ...actual, fittedScale: (...args: Parameters<typeof actual.fittedScale>) => { fitted.calls++; return actual.fittedScale(...args); } };
});
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
it('fits the box to the canvas once while the stage gives the same box, however often the turned view is asked for', () => {
  const { canvas, stage, setView, pointer } = setup();
  let box = { low: [0, 0, 0], high: [100, 100, 100] };
  Object.assign(stage, { bounds: () => box });
  const hand = attachHandTurning(stage, canvas, { reducedMotion: () => true });
  pointer('pointerdown', 0, 0); pointer('pointermove', 30, 20);
  const rule = (setView.mock.calls.at(-1) as unknown[] | undefined)?.[0] as ((size: { width: number; height: number }) => { pxPerMm: readonly [number, number] }) | undefined;
  if (rule === undefined) throw new Error('No view was set');
  const first = rule({ width: 400, height: 600 }), calls = fitted.calls;
  for (let i = 0; i < 100; i++) expect(rule({ width: 400, height: 600 })).toEqual(first);
  expect(fitted.calls).toBe(calls);
  box = { low: [0, 0, 0], high: [200, 200, 200] };
  expect(rule({ width: 400, height: 600 }).pxPerMm[0]).toBeCloseTo(first.pxPerMm[0] / 2);
  expect(fitted.calls).toBe(calls + 1);
  rule({ width: 200, height: 600 }); expect(fitted.calls).toBe(calls + 2);
  pointer('pointerup', 30, 21); hand.detach();
});
it('asks what is under a still pointer once on the frame after a drawing, and not at all when the pointer moves first', () => {
  const { canvas, stage, pointer } = setup();
  let drawn: () => void = () => { /* Set when the hand listens. */ };
  const pick = vi.fn(() => null);
  Object.assign(stage, { pick, onDrawn: (listener: () => void) => { drawn = listener; return () => undefined; } });
  const hand = attachHandTurning(stage, canvas);
  pointer('pointermove', 10, 0); expect(pick).toHaveBeenCalledTimes(1);
  drawn(); drawn(); expect(pick).toHaveBeenCalledTimes(1); expect(loop.size).toBe(1);
  for (const tick of [...loop]) tick(16, 16);
  expect(pick).toHaveBeenCalledTimes(2); expect(loop.size).toBe(0);
  drawn(); pointer('pointermove', 12, 20); expect(pick).toHaveBeenCalledTimes(3); expect(loop.size).toBe(0);
  drawn(); hand.detach(); expect(loop.size).toBe(0); expect(pick).toHaveBeenCalledTimes(3);
});
