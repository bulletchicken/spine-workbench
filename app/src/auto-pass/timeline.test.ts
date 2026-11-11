import type { Phase } from '@/look-and-feel/data-attributes';
import { LEVELS } from '@/shapes';
import type { ChainBody, Level, Px, WatchFrame } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { PASS_END } from './beats';
import { recordFrames } from './recording/record-frames';
import { TEST_MOVED_LEVEL, testBodies, testFrames } from './recording/test-frames';
import { passTimeline, type FilmPicture } from './timeline';

const PHASES: Phase[] = ['hidden', 'ghost', 'locked', 'named', 'coloured'];

function testTimeline() {
  return passTimeline(recordFrames(['front', 'side'], [...testFrames('front'), ...testFrames('side')]));
}

function picture(t: number, film: 'front' | 'side' = 'front'): FilmPicture {
  const shown = testTimeline().at(film, t);
  if (shown === null) throw new Error('the film is in the pass');
  return shown;
}

/** A straight strip one film pixel per row, down x = 100 from y = 0. */
function straightStrip(rows: number): WatchFrame {
  return {
    kind: 'strip-shape',
    film: 'front',
    stage: 'strip',
    centres: Array.from({ length: rows }, (_, r): Px => [100, r]),
    normals: Array.from({ length: rows }, (): Px => [1, 0]),
    columnPx: 1,
    columns: 21,
  };
}

function body(level: Level, top: number, bottom: number): ChainBody {
  return { level, sup: { a: [90, top], b: [110, top] }, inf: { a: [90, bottom], b: [110, bottom] }, inferred: false };
}

describe('the search on the test pass', () => {
  it('shows nothing before the sweep reaches the first body', () => {
    const shown = picture(0);
    expect(shown.guesses).toEqual([]);
    expect(shown.vertebrae.map((v) => v.phase)).toEqual(new Array(18).fill('hidden'));
    expect([shown.searched, shown.searchTotal, shown.locked]).toEqual([0, 18, 0]);
  });

  it('brings in the first guess when the sweep line reaches it', () => {
    // C7's middle is at y = 112 + 14 = 126, on strip row 11 (y = 100 + 2.4·11
    // = 126.4). The strip's last row is 299, so the line reaches it at
    // 5400 + 3600·11/299 = 5532.4 ms.
    expect(picture(5532).guesses).toEqual([]);
    expect(picture(5533).guesses.map((g) => g.level)).toEqual(['C7']);
    expect(picture(5533).searched).toBe(1);
  });

  it('shows the newest sweep frame’s whole chain, the wrong guess included', () => {
    // T12 is body 12: the middle of its guess, which runs 9 px low, is at
    // y = 112 + 39·12 + 14 + 4.5 = 598.5, on row 208 (y = 100 + 2.4·208 =
    // 599.2), reached at 5400 + 3600·208/299.
    const t = Math.ceil(5400 + (3600 * 208) / 299);
    const shown = picture(t);
    expect(shown.guesses).toHaveLength(13);
    const guess = shown.guesses[12];
    const truth = testBodies('front')[12];
    expect(guess?.level).toBe(TEST_MOVED_LEVEL);
    expect(guess?.corners[2]).toEqual([truth?.inf.b[0], (truth?.inf.b[1] ?? 0) + 9]);
    expect(picture(t - 1).guesses).toHaveLength(12);
  });

  it('gives out when each sweep frame arrives, the times the search counts by', () => {
    // C7 on row 11 as above; T12's guess on row 208 as above.
    const times = testTimeline().sweepTimes('front');
    expect(times).toHaveLength(18);
    expect(times[0]).toBeCloseTo(5400 + (3600 * 11) / 299, 6);
    expect(times[12]).toBeCloseTo(5400 + (3600 * 208) / 299, 6);
    times.forEach((t, k) => {
      expect(picture(t).searched).toBeGreaterThanOrEqual(k + 1);
    });
  });

  it('hands every guess to its locked body when the search ends', () => {
    const shown = picture(9000);
    expect(shown.guesses).toEqual([]);
    expect(shown.vertebrae.every((v) => v.phase === 'ghost')).toBe(true);
    expect(shown.searched).toBe(18);
  });
});

describe('lock-in on the test pass', () => {
  it('locks from the pelvis up, 36 ms apart', () => {
    const times = testTimeline().times('front');
    expect(times.map((v) => v.lockAt)).toEqual(Array.from({ length: 18 }, (_, i) => 9150 + 36 * (17 - i)));
    expect(picture(9150).vertebrae.map((v) => v.phase).slice(16)).toEqual(['ghost', 'locked']);
    expect(picture(9150).locked).toBe(1);
  });

  it('names from both ends at once and colours from the top', () => {
    const times = testTimeline().times('front');
    // Body 0 and body 17 are both at an end: 9800 ms. Body 8 is eight from
    // the top and nine from the bottom: 9800 + 38·8.
    expect(times[0]?.nameAt).toBe(9800);
    expect(times[17]?.nameAt).toBe(9800);
    expect(times[8]?.nameAt).toBe(9800 + 38 * 8);
    expect(times.map((v) => v.colourAt)).toEqual(Array.from({ length: 18 }, (_, i) => 10400 + 34 * i));
  });

  it('reports exactly one moved outline, T12, from its own lock on', () => {
    const before = picture(9150 + 36 * 4);
    const after = picture(9150 + 36 * 5);
    const t12 = (shown: FilmPicture) => shown.vertebrae[12];
    expect(t12(before)?.movedFrom).toBeNull();
    expect(t12(after)?.movedFrom).toEqual(picture(9000).vertebrae[12]?.corners);
    expect(picture(PASS_END).vertebrae.filter((v) => v.movedFrom !== null).map((v) => v.index)).toEqual([12]);
  });

  it('gives the corners a body was locked on from its colouring on', () => {
    // Body 3 colours at 10400 + 34·3; its locked corners are its test body's.
    const t3 = (t: number) => picture(t).vertebrae[3];
    const truth = testBodies('front')[3];
    expect(t3(10400 + 34 * 3 - 1)?.colouredFrom).toBeNull();
    expect(t3(10400 + 34 * 3)?.colouredFrom).toEqual([truth?.sup.a, truth?.sup.b, truth?.inf.b, truth?.inf.a]);
    expect(t3(10400 + 34 * 3)?.corners).not.toEqual(t3(10400 + 34 * 3)?.colouredFrom);
  });

  it('ends with every body coloured, named and on its final outline', () => {
    const shown = picture(PASS_END);
    const outlines = testFrames('front').find((frame) => frame.kind === 'outlines');
    expect(shown.vertebrae.map((v) => v.phase)).toEqual(new Array(18).fill('coloured'));
    expect(shown.vertebrae.map((v) => v.level)).toEqual(testBodies('front').map((b) => b.level));
    expect(shown.vertebrae.map((v) => v.corners)).toEqual(outlines?.kind === 'outlines' ? outlines.quads.map((q) => q.corners) : null);
    expect(shown.vertebrae[12]?.band).toBe('low');
    expect(shown.vertebrae[0]?.band).toBe('high');
  });

  it('has nothing for a film the pass did not run', () => {
    expect(testTimeline().at('bend-left', 5000)).toBeNull();
    expect(testTimeline().times('bend-left')).toEqual([]);
    expect(testTimeline().sweepTimes('bend-left')).toEqual([]);
  });
});

describe('a locked chain of another count than the search’s', () => {
  // The search guessed five standard bodies, C7 to T4. The locked chain has
  // four, named T1 to T4: the first three sit near the first, second and
  // fourth guesses, and the last sits far below every guess.
  const guesses = [body('C7', 10, 30), body('T1', 40, 60), body('T2', 70, 90), body('T3', 100, 120), body('T4', 130, 150)];
  const lockedTopDown = [body('T1', 11, 31), body('T2', 41, 61), body('T3', 100, 121), body('T4', 158, 178)];
  const frames: WatchFrame[] = [
    straightStrip(200),
    ...guesses.map((_, k): WatchFrame => ({ kind: 'sweep', film: 'front', stage: 'chain', level: guesses[k]?.level ?? 'C7', heat: new Uint8Array(256), tentative: guesses.slice(0, k + 1) })),
    { kind: 'chain-locked', film: 'front', stage: 'chain', bodies: lockedTopDown.slice().reverse() },
  ];
  const timeline = passTimeline(recordFrames(['front'], frames));

  it('matches each locked body to a guess by where it sits on the strip, never by name', () => {
    const shown = timeline.at('front', 9000);
    // T1 locked sits where the search guessed C7, so it stands as that guess.
    expect(shown?.vertebrae[0]?.corners).toEqual([[90, 10], [110, 10], [110, 30], [90, 30]]);
    expect(shown?.vertebrae[1]?.corners).toEqual([[90, 40], [110, 40], [110, 60], [90, 60]]);
    expect(shown?.vertebrae[2]?.corners).toEqual([[90, 100], [110, 100], [110, 120], [90, 120]]);
    expect(shown?.vertebrae.map((v) => v.movedFrom)).toEqual([null, null, null, null]);
  });

  it('brings in a body with no guess within half its height at its lock', () => {
    const times = timeline.times('front');
    expect(times[3]?.ghostAt).toBeNull();
    const lockAt = times[3]?.lockAt ?? 0;
    expect(timeline.at('front', lockAt - 1)?.vertebrae[3]?.phase).toBe('hidden');
    expect(timeline.at('front', lockAt)?.vertebrae[3]?.phase).toBe('locked');
    expect(timeline.at('front', lockAt)?.vertebrae[3]?.corners).toEqual([[90, 158], [110, 158], [110, 178], [90, 178]]);
  });

  it('keeps the guesses no body took until the lock-in reaches them', () => {
    // The T2 and T4 guesses (middles at rows 80 and 140) were taken by no
    // body. T2's nearest body is the second (row 51, 29 rows away, against
    // 30.5 for the third); T4's is the fourth (row 168, 28 rows away,
    // against 29.5 for the third).
    const times = timeline.times('front');
    const left = (t: number) => timeline.at('front', t)?.guesses.map((g) => g.level);
    expect(left(9000)).toEqual(['T2', 'T4']);
    expect(left((times[3]?.lockAt ?? 0) - 1)).toEqual(['T2', 'T4']);
    expect(left(times[3]?.lockAt ?? 0)).toEqual(['T2']);
    expect(left(times[1]?.lockAt ?? 0)).toEqual([]);
  });

  it('shows no names when the chain was never named', () => {
    expect(timeline.at('front', PASS_END)?.vertebrae.map((v) => [v.phase, v.level])).toEqual(new Array(4).fill(['locked', null]));
  });
});

describe('matching a locked body to a guess', () => {
  // One guess, rows 10 to 30, its middle on row 20. Each locked body is 20
  // rows high, so a guess is taken only within 10 rows of its middle.
  function ghostAt(top: number): number | null {
    const frames: WatchFrame[] = [
      straightStrip(100),
      { kind: 'sweep', film: 'front', stage: 'chain', level: 'C7', heat: new Uint8Array(256), tentative: [body('C7', 10, 30)] },
      { kind: 'chain-locked', film: 'front', stage: 'chain', bodies: [body('T1', top, top + 20)] },
    ];
    return passTimeline(recordFrames(['front'], frames)).times('front')[0]?.ghostAt ?? null;
  }

  it('takes the guess within half the body’s height', () => {
    expect(ghostAt(19)).toBe(9000);
    expect(ghostAt(20)).toBe(9000);
  });

  it('leaves the guess between half and a whole height away', () => {
    expect(ghostAt(21)).toBeNull();
    expect(ghostAt(26)).toBeNull();
  });
});

describe('any locked chain', () => {
  const chains = fc.integer({ min: 1, max: 26 }).chain((n) =>
    fc.record({
      n: fc.constant(n),
      heights: fc.array(fc.integer({ min: 4, max: 30 }), { minLength: n, maxLength: n }),
      gaps: fc.array(fc.integer({ min: 1, max: 12 }), { minLength: n, maxLength: n }),
      shifts: fc.array(fc.integer({ min: -12, max: 12 }), { minLength: n, maxLength: n }),
      times: fc.array(fc.integer({ min: 0, max: PASS_END + 200 }), { minLength: 2, maxLength: 12 }),
    }),
  );

  function pass(n: number, heights: number[], gaps: number[], shifts: number[]) {
    const levels = LEVELS.slice(0, n);
    let y = 5;
    const bodies = levels.map((level, i) => {
      y += gaps[i] ?? 1;
      const top = y;
      y += heights[i] ?? 4;
      return body(level, top, y);
    });
    const guessed = bodies.map((b, i) => body(b.level, b.sup.a[1] + (shifts[i] ?? 0), b.inf.a[1] + (shifts[i] ?? 0)));
    const frames: WatchFrame[] = [
      straightStrip(y + 20),
      ...guessed.map((b, k): WatchFrame => ({ kind: 'sweep', film: 'front', stage: 'chain', level: b.level, heat: new Uint8Array(256), tentative: guessed.slice(0, k + 1) })),
      { kind: 'chain-locked', film: 'front', stage: 'chain', bodies: bodies.slice().reverse() },
      {
        kind: 'levels-named',
        film: 'front',
        stage: 'naming',
        naming: { template: 'standard', chosenBy: 'engine', alternatives: [], fromTop: levels, fromBottom: levels, flags: [], l5ConfirmedOn: null },
        levels,
      },
      { kind: 'confidence', film: 'front', stage: 'confidence', levels: levels.map((level) => ({ level, score: 0.95, band: 'high' })) },
      { kind: 'outlines', film: 'front', stage: 'confidence', quads: bodies.map((b) => ({ level: b.level, corners: [b.sup.a, b.sup.b, b.inf.b, b.inf.a] })), final: true },
    ];
    return passTimeline(recordFrames(['front'], frames));
  }

  it('gives every body its times in order: ghost, lock, name, colour', () => {
    fc.assert(
      fc.property(chains, ({ n, heights, gaps, shifts }) => {
        for (const v of pass(n, heights, gaps, shifts).times('front')) {
          const order = [v.ghostAt, v.lockAt, v.nameAt, v.colourAt].filter((t): t is number => t !== null);
          expect(order).toEqual(order.slice().sort((a, b) => a - b));
          expect(v.nameAt).not.toBeNull();
          expect(v.colourAt).not.toBeNull();
        }
      }),
    );
  });

  it('never takes a body back a phase as time goes on', () => {
    fc.assert(
      fc.property(chains, ({ n, heights, gaps, shifts, times }) => {
        const timeline = pass(n, heights, gaps, shifts);
        const sorted = times.slice().sort((a, b) => a - b);
        const ranks = sorted.map((t) => timeline.at('front', t)?.vertebrae.map((v) => PHASES.indexOf(v.phase)) ?? []);
        ranks.slice(1).forEach((later, k) => {
          later.forEach((rank, i) => {
            expect(rank).toBeGreaterThanOrEqual(ranks[k]?.[i] ?? 0);
          });
        });
      }),
    );
  });

  it('gives the same picture each time a moment is asked for, whatever was asked in between', () => {
    fc.assert(
      fc.property(chains, ({ n, heights, gaps, shifts, times }) => {
        const timeline = pass(n, heights, gaps, shifts);
        const first = times.map((t) => timeline.at('front', t));
        const again = times
          .slice()
          .reverse()
          .map((t) => timeline.at('front', t))
          .reverse();
        expect(again).toEqual(first);
      }),
    );
  });

  it('changes the picture only when the change index changes', () => {
    fc.assert(
      fc.property(chains, ({ n, heights, gaps, shifts, times }) => {
        const timeline = pass(n, heights, gaps, shifts);
        const sorted = times.slice().sort((a, b) => a - b);
        sorted.slice(1).forEach((t, k) => {
          const before = sorted[k] ?? 0;
          expect(timeline.changeIndex(t)).toBeGreaterThanOrEqual(timeline.changeIndex(before));
          if (timeline.changeIndex(t) === timeline.changeIndex(before)) expect(timeline.at('front', t)).toEqual(timeline.at('front', before));
        });
      }),
    );
  });
});

describe('the change index on the test pass', () => {
  it('rises a few dozen times over the pass, not once a frame', () => {
    const timeline = testTimeline();
    expect(timeline.changeIndex(0)).toBe(0);
    const total = timeline.changeIndex(PASS_END);
    expect(total).toBeGreaterThan(36);
    expect(total).toBeLessThan(250);
  });
});
