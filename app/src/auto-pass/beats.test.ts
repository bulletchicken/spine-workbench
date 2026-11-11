import { describe, expect, it } from 'vitest';
import { BEATS, PASS_END, PASS_TIMES, beatAt, filmLightAt, stackLightAt } from './beats';

describe('the six beats', () => {
  it('are the design table: evidence from 0.4 s, confidence until 11.6 s, each beginning where the last ends', () => {
    expect(BEATS.map((beat) => [beat.id, beat.start, beat.end])).toEqual([
      ['evidence', 400, 2400],
      ['centre-line', 2400, 3700],
      ['unroll', 3700, 5400],
      ['search', 5400, 9000],
      ['lock-in', 9000, 10400],
      ['confidence', 10400, 11600],
    ]);
    expect(PASS_END).toBe(11800);
  });

  it('name the beat playing at a time, and none outside them', () => {
    expect(beatAt(0)).toBeNull();
    expect(beatAt(399)).toBeNull();
    expect(beatAt(400)?.id).toBe('evidence');
    expect(beatAt(2399)?.id).toBe('evidence');
    expect(beatAt(2400)?.id).toBe('centre-line');
    expect(beatAt(9000)?.id).toBe('lock-in');
    expect(beatAt(11599)?.id).toBe('confidence');
    expect(beatAt(11600)).toBeNull();
    expect(beatAt(PASS_END)).toBeNull();
  });

  it('fit the three evidence wipes inside the evidence beat', () => {
    const last = PASS_TIMES.wipeStarts[2] + PASS_TIMES.wipeLasts;
    expect(last).toBe(2360);
    expect(last).toBeLessThanOrEqual(2400);
  });
});

describe('the light on the films', () => {
  // Worked by hand: the films fall from 1 to 0.58 over 0 to 450 ms and
  // climb back over 10 400 to 11 300 ms, in straight lines.
  it('dims Front and Side to 58% and brings them back in the last beat', () => {
    expect(filmLightAt(0)).toBe(1);
    expect(filmLightAt(225)).toBeCloseTo(0.79, 12);
    expect(filmLightAt(450)).toBeCloseTo(0.58, 12);
    expect(filmLightAt(5000)).toBeCloseTo(0.58, 12);
    expect(filmLightAt(10400)).toBeCloseTo(0.58, 12);
    expect(filmLightAt(10850)).toBeCloseTo(0.79, 12);
    expect(filmLightAt(11300)).toBeCloseTo(1, 12);
    expect(filmLightAt(PASS_END)).toBeCloseTo(1, 12);
  });

  it('dims the bending stack to 38% on the same schedule', () => {
    expect(stackLightAt(0)).toBe(1);
    expect(stackLightAt(225)).toBeCloseTo(0.69, 12);
    expect(stackLightAt(3000)).toBeCloseTo(0.38, 12);
    expect(stackLightAt(10850)).toBeCloseTo(0.69, 12);
    expect(stackLightAt(PASS_END)).toBeCloseTo(1, 12);
  });
});
