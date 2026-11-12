import { expect, it, vi } from 'vitest';
import { createFindReplay } from './find-replay-clock';
import { replayEnds, type RevealPlan } from './find-replay';

it('reveals outlines at their moments, stops after the hold, and keeps a separate light signal', () => {
  const c = createFindReplay();
  const plan: RevealPlan = { at: new Map([['front:1', 0.1], ['front:2', 0.5], ['front:3', 0.9]]), counted: ['front:1', 'front:2', 'front:3'], last: 0.9, light: {} };
  const times = { lead: 700, travel: 4400, hold: 560 };
  c.getState().advance(1000);
  expect(c.getState().time).toBe(0);
  c.getState().start(plan, times);
  const hear = vi.fn();
  const leave = c.signal.subscribe(hear);
  expect(c.signal.share()).toBeNull();
  c.getState().advance(700);
  expect(c.signal.share()).toBe(0);
  let last = 700;
  for (const [share, count] of [[0.1, 1], [0.5, 2], [0.9, 3]] as const) {
    const time = 700 + share * 4400;
    c.getState().advance(time - last);
    last = time;
    expect(c.getState().revealed).toBe(count);
  }
  expect(c.getState().allShown).toBe(true);
  c.getState().pause();
  c.getState().advance(500);
  expect(c.getState().time).toBe(last);
  c.getState().start(plan, times);
  c.getState().advance(100000);
  expect(c.getState()).toMatchObject({ playing: false, time: replayEnds(plan, times), revealed: 3 });
  expect(c.signal.share()).toBeNull();
  expect(hear).toHaveBeenCalled();
  leave();
  c.getState().clear();
  expect(c.getState()).toMatchObject({ plan: null, time: 0, revealed: 0, allShown: false });
  expect(c.signal.share()).toBeNull();
});
