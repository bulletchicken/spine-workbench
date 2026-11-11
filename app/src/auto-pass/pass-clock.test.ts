import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { PASS_END } from './beats';
import { createPassClock, holdPoint, isHeld, STILLS } from './pass-clock';
import { readyBeats, recordFrame, recordFrames } from './recording/record-frames';
import { testFrames } from './recording/test-frames';

const ALL_READY = 6;

function clock(readyBeats = ALL_READY, reduced = false) {
  return createPassClock({ readyBeats, reduced });
}

describe('playing and pausing', () => {
  it('starts paused at the beginning, at normal speed', () => {
    const state = clock().getState();
    expect([state.time, state.playing, state.speed]).toEqual([0, false, 1]);
  });

  it('moves on by the time that passed, times the speed', () => {
    const c = clock();
    c.getState().play();
    c.getState().advance(1000);
    expect(c.getState().time).toBe(1000);
    c.getState().setSpeed(2);
    c.getState().advance(500);
    expect(c.getState().time).toBe(2000);
    c.getState().setSpeed(0.5);
    c.getState().advance(1000);
    expect(c.getState().time).toBe(2500);
  });

  it('stands still while paused, and toggles back on', () => {
    const c = clock();
    c.getState().play();
    c.getState().advance(300);
    c.getState().pause();
    c.getState().advance(5000);
    expect(c.getState().time).toBe(300);
    c.getState().toggle();
    expect(c.getState().playing).toBe(true);
    c.getState().advance(100);
    expect(c.getState().time).toBe(400);
    c.getState().toggle();
    expect(c.getState().playing).toBe(false);
  });

  it('stops at the end, and plays from the start when asked again', () => {
    const c = clock();
    c.getState().play();
    c.getState().advance(PASS_END + 5000);
    expect([c.getState().time, c.getState().playing]).toEqual([PASS_END, false]);
    c.getState().play();
    expect([c.getState().time, c.getState().playing]).toEqual([0, true]);
  });

  it('replays from the start, wherever it was', () => {
    const c = clock();
    c.getState().seek(7000);
    c.getState().replay();
    expect([c.getState().time, c.getState().playing]).toEqual([0, true]);
  });
});

describe('scrubbing and skipping', () => {
  it('scrubs both ways and pauses', () => {
    const c = clock();
    c.getState().play();
    c.getState().seek(9000);
    expect([c.getState().time, c.getState().playing]).toEqual([9000, false]);
    c.getState().seek(2500);
    expect(c.getState().time).toBe(2500);
    c.getState().seek(-40);
    expect(c.getState().time).toBe(0);
    c.getState().seek(PASS_END + 1);
    expect(c.getState().time).toBe(PASS_END);
  });

  it('skips to the end when everything has arrived', () => {
    const c = clock();
    c.getState().play();
    c.getState().skip();
    expect([c.getState().time, c.getState().playing, c.getState().skipping]).toEqual([PASS_END, false, false]);
  });

  it('lands in the same state when seeking to the same time twice', () => {
    fc.assert(
      fc.property(fc.double({ min: -1000, max: PASS_END + 1000, noNaN: true }), fc.integer({ min: 0, max: 6 }), (t, ready) => {
        const c = clock(ready);
        c.getState().seek(t);
        const first = { ...c.getState() };
        c.getState().seek(t / 3);
        c.getState().seek(t);
        expect({ ...c.getState() }).toEqual(first);
        expect(c.getState().time).toBeGreaterThanOrEqual(0);
        expect(c.getState().time).toBeLessThanOrEqual(holdPoint(ready));
      }),
    );
  });

  it('stays where it was when given a time, step or count that is not a number', () => {
    const c = clock();
    c.getState().seek(3000);
    c.getState().seek(Number.NaN);
    c.getState().seek(Number.POSITIVE_INFINITY);
    expect([c.getState().time, c.getState().playing]).toEqual([3000, false]);
    c.getState().play();
    c.getState().advance(Number.NaN);
    c.getState().advance(Number.NEGATIVE_INFINITY);
    c.getState().setReady(Number.NaN);
    expect([c.getState().time, c.getState().readyBeats]).toEqual([3000, ALL_READY]);
    c.getState().advance(500);
    expect([c.getState().time, c.getState().playing]).toEqual([3500, true]);
  });
});

describe('holding for the engine', () => {
  it('holds at the end of the last beat whose data has arrived', () => {
    // Evidence, centre line and unroll are in: hold at the unroll's end.
    expect(holdPoint(3)).toBe(5400);
    expect(holdPoint(0)).toBe(400);
    expect(holdPoint(5)).toBe(10400);
    expect(holdPoint(6)).toBe(PASS_END);
  });

  it('holds with the last frames withheld, and resumes when they come', () => {
    const c = clock(5);
    c.getState().play();
    c.getState().advance(20000);
    expect(c.getState().time).toBe(10400);
    expect(c.getState().playing).toBe(true);
    expect(isHeld(c.getState())).toBe(true);
    c.getState().setReady(6);
    expect(isHeld(c.getState())).toBe(false);
    c.getState().advance(100);
    expect(c.getState().time).toBe(10500);
    c.getState().advance(20000);
    expect([c.getState().time, c.getState().playing]).toEqual([PASS_END, false]);
  });

  it('holds on a recording whose last frames have not come, and plays on when they do', () => {
    const frames = [...testFrames('front'), ...testFrames('side')];
    const late = frames.filter((frame) => frame.film === 'side' && (frame.kind === 'confidence' || frame.kind === 'outlines'));
    let recording = recordFrames(['front', 'side'], frames.filter((frame) => !late.includes(frame)));
    const c = clock(readyBeats(recording));
    c.getState().play();
    c.getState().advance(30000);
    expect(c.getState().time).toBe(10400);
    expect(isHeld(c.getState())).toBe(true);

    recording = late.reduce(recordFrame, recording);
    c.getState().setReady(readyBeats(recording));
    c.getState().advance(600);
    expect(c.getState().time).toBe(11000);
  });

  it('holds before the first beat while nothing has arrived', () => {
    const c = clock(0);
    c.getState().play();
    c.getState().advance(3000);
    expect(c.getState().time).toBe(400);
    expect(isHeld(c.getState())).toBe(true);
  });

  it('seeks no further than the data allows', () => {
    const c = clock(2);
    c.getState().seek(8000);
    expect(c.getState().time).toBe(3700);
  });

  it('skips as far as the data allows, then on to the end as it arrives', () => {
    const c = clock(3);
    c.getState().play();
    c.getState().skip();
    expect([c.getState().time, c.getState().skipping, c.getState().playing]).toEqual([5400, true, false]);
    c.getState().setReady(5);
    expect(c.getState().time).toBe(10400);
    c.getState().setReady(6);
    expect([c.getState().time, c.getState().skipping]).toEqual([PASS_END, false]);
  });

  it('stops skipping ahead when the person scrubs', () => {
    const c = clock(3);
    c.getState().skip();
    c.getState().seek(1000);
    c.getState().setReady(6);
    expect(c.getState().time).toBe(1000);
  });

  it('is never past its hold point, whatever is pressed', () => {
    const actions = fc.oneof(
      fc.record({ kind: fc.constant('advance' as const), ms: fc.integer({ min: 0, max: 3000 }) }),
      fc.record({ kind: fc.constant('seek' as const), t: fc.integer({ min: -500, max: PASS_END + 500 }) }),
      fc.record({ kind: fc.constant('ready' as const), beats: fc.integer({ min: 0, max: 6 }) }),
      fc.record({ kind: fc.constantFrom('play' as const, 'pause' as const, 'skip' as const, 'replay' as const) }),
      fc.record({ kind: fc.constant('speed' as const), speed: fc.constantFrom(0.5 as const, 1 as const, 2 as const) }),
    );
    fc.assert(
      fc.property(fc.boolean(), fc.array(actions, { maxLength: 40 }), (reduced, steps) => {
        const c = clock(0, reduced);
        for (const step of steps) {
          const s = c.getState();
          if (step.kind === 'advance') s.advance(step.ms);
          else if (step.kind === 'seek') s.seek(step.t);
          else if (step.kind === 'ready') s.setReady(step.beats);
          else if (step.kind === 'speed') s.setSpeed(step.speed);
          else s[step.kind]();
          expect(c.getState().time).toBeLessThanOrEqual(holdPoint(c.getState().readyBeats));
          expect(c.getState().time).toBeGreaterThanOrEqual(0);
        }
      }),
    );
  });
});

describe('under reduced motion', () => {
  it('steps through the end of each beat, then the end, 750 ms apart', () => {
    expect(STILLS).toEqual([2400, 3700, 5400, 9000, 10400, 11600, PASS_END]);
    const c = clock(ALL_READY, true);
    c.getState().play();
    const seen = [c.getState().time];
    for (let i = 0; i < 6; i++) {
      c.getState().advance(749);
      expect(c.getState().time).toBe(seen.at(-1));
      c.getState().advance(1);
      seen.push(c.getState().time);
    }
    expect(seen).toEqual(STILLS);
    expect(c.getState().playing).toBe(false);
  });

  it('ignores the speed', () => {
    const c = clock(ALL_READY, true);
    c.getState().setSpeed(2);
    c.getState().play();
    c.getState().advance(749);
    expect(c.getState().time).toBe(2400);
  });

  it('holds on the last still whose data has arrived and shows the next as soon as it comes', () => {
    const c = clock(2, true);
    c.getState().play();
    c.getState().advance(750);
    expect(c.getState().time).toBe(3700);
    c.getState().advance(5000);
    expect(c.getState().time).toBe(3700);
    expect(isHeld(c.getState())).toBe(true);
    c.getState().setReady(3);
    c.getState().advance(16);
    expect(c.getState().time).toBe(5400);
  });

  it('scrubs to any time, and plays on from there to the next still', () => {
    const c = clock(ALL_READY, true);
    c.getState().seek(4100);
    expect(c.getState().time).toBe(4100);
    c.getState().play();
    expect(c.getState().time).toBe(5400);
  });
});
