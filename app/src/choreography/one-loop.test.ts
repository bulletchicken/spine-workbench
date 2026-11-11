// The loop on frames a test hands out by hand, so every number here is
// worked out from the frame times written in the test.

import { describe, expect, it } from 'vitest';
import { LONGEST_FRAME_MS, makeLoop, type Frames } from './one-loop';

/** Frames that arrive only when the test says so. */
function handFrames() {
  let next = 1;
  const waiting = new Map<number, (now: number) => void>();
  const frames: Frames = {
    request(callback) {
      const handle = next;
      next += 1;
      waiting.set(handle, callback);
      return handle;
    },
    cancel(handle) {
      waiting.delete(handle);
    },
  };
  return {
    frames,
    /** How many frames have been asked for and neither sent nor cancelled. */
    asked: () => waiting.size,
    /** Sends the frame the loop is waiting for, at time `now`. */
    send: (now: number) => {
      const [first] = waiting;
      if (first === undefined) throw new Error('no frame was asked for');
      const [handle, callback] = first;
      waiting.delete(handle);
      callback(now);
    },
  };
}

describe('the one loop', () => {
  it('asks for no frame until something joins', () => {
    const { frames, asked } = handFrames();
    makeLoop(frames);
    expect(asked()).toBe(0);
  });

  it('starts with its first joiner and stops with its last', () => {
    const { frames, asked, send } = handFrames();
    const loop = makeLoop(frames);

    const leaveFirst = loop.join(() => undefined);
    expect(asked()).toBe(1);
    const leaveSecond = loop.join(() => undefined);
    expect(asked()).toBe(1);

    send(1000);
    expect(asked()).toBe(1);

    leaveFirst();
    expect(asked()).toBe(1);
    leaveSecond();
    expect(asked()).toBe(0);
  });

  it('starts again when something joins after it stopped', () => {
    const { frames, asked, send } = handFrames();
    const loop = makeLoop(frames);
    const seen: number[] = [];

    loop.join(() => undefined)();
    expect(asked()).toBe(0);

    loop.join((dt) => seen.push(dt));
    expect(asked()).toBe(1);
    send(5000);
    send(5020);
    expect(seen).toEqual([0, 20]);
  });

  it('tells each joiner the time since the frame before, and 0 on its first', () => {
    const { frames, send } = handFrames();
    const loop = makeLoop(frames);
    const early: [number, number][] = [];
    const late: [number, number][] = [];

    loop.join((dt, now) => early.push([dt, now]));
    send(1000);
    send(1016);
    loop.join((dt, now) => late.push([dt, now]));
    send(1049);
    send(1066);

    expect(early).toEqual([
      [0, 1000],
      [16, 1016],
      [33, 1049],
      [17, 1066],
    ]);
    expect(late).toEqual([
      [0, 1049],
      [17, 1066],
    ]);
  });

  it('counts a long absence as one long frame', () => {
    const { frames, send } = handFrames();
    const loop = makeLoop(frames);
    const seen: number[] = [];

    loop.join((dt) => seen.push(dt));
    send(1000);
    send(1016);
    // The tab was hidden for a minute.
    send(61_016);
    send(61_032);

    expect(LONGEST_FRAME_MS).toBe(100);
    expect(seen).toEqual([0, 16, 100, 16]);
  });

  it('stops calling a joiner that has left, and keeps calling the others', () => {
    const { frames, send } = handFrames();
    const loop = makeLoop(frames);
    let stayed = 0;
    let left = 0;

    loop.join(() => {
      stayed += 1;
    });
    const leave = loop.join(() => {
      left += 1;
    });
    send(1000);
    leave();
    send(1016);

    expect(stayed).toBe(2);
    expect(left).toBe(1);
  });

  it('lets a joiner leave in the middle of a frame', () => {
    const { frames, asked, send } = handFrames();
    const loop = makeLoop(frames);
    const order: string[] = [];

    const leaveA = loop.join(() => {
      order.push('a');
      leaveB();
    });
    const leaveB = loop.join(() => order.push('b'));
    send(1000);
    // b left before its turn came, so it never ran; a is still in.
    expect(order).toEqual(['a']);
    expect(asked()).toBe(1);

    leaveA();
    expect(asked()).toBe(0);
  });

  it('stops when its only joiner leaves from inside its own step', () => {
    const { frames, asked, send } = handFrames();
    const loop = makeLoop(frames);
    let calls = 0;

    const leave = loop.join(() => {
      calls += 1;
      if (calls === 2) leave();
    });
    send(1000);
    send(1016);

    expect(calls).toBe(2);
    expect(asked()).toBe(0);
  });

  it('gives a joiner that joins during a frame its first step on the next one', () => {
    const { frames, send } = handFrames();
    const loop = makeLoop(frames);
    const seen: number[] = [];
    let joined = false;

    loop.join(() => {
      if (joined) return;
      joined = true;
      loop.join((dt) => seen.push(dt));
    });
    send(1000);
    expect(seen).toEqual([]);
    send(1016);
    send(1040);
    expect(seen).toEqual([0, 24]);
  });

  it('leaving twice is the same as leaving once', () => {
    const { frames, asked } = handFrames();
    const loop = makeLoop(frames);

    const leaveFirst = loop.join(() => undefined);
    loop.join(() => undefined);
    leaveFirst();
    leaveFirst();

    expect(asked()).toBe(1);
  });
});
