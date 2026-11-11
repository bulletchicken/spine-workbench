// Reading CSS times and curves. Each answer is the CSS value written out
// by hand: 2.6 s is 2600 ms; a curve is checked at the one point its
// definition gives without solving anything (see curve.test.ts).

import { describe, expect, it } from 'vitest';
import { curveIn, millisecondsIn } from './timing';

describe('millisecondsIn', () => {
  it('reads milliseconds and seconds', () => {
    expect(millisecondsIn('700ms')).toBe(700);
    expect(millisecondsIn('1ms')).toBe(1);
    expect(millisecondsIn('0ms')).toBe(0);
    expect(millisecondsIn('2.6s')).toBe(2600);
    expect(millisecondsIn('.5s')).toBe(500);
  });

  it('reads a value as getComputedStyle hands it over, spaces and all', () => {
    expect(millisecondsIn(' 420ms')).toBe(420);
    expect(millisecondsIn('160ms ')).toBe(160);
  });

  it('refuses what is not a time, so a mistyped name fails loudly', () => {
    expect(() => millisecondsIn('')).toThrow('not a CSS time');
    expect(() => millisecondsIn('700')).toThrow('not a CSS time');
    expect(() => millisecondsIn('12px')).toThrow('not a CSS time');
    expect(() => millisecondsIn('cubic-bezier(.2, .8, .2, 1)')).toThrow('not a CSS time');
  });
});

describe('curveIn', () => {
  it('reads the design’s curves as CSS writes them', () => {
    // --ease-out reaches .8 of the way at .275 of the time; --ease-snap is past its end, at 1.025, at .35.
    expect(curveIn('cubic-bezier(.2, .8, .2, 1)')(0.275)).toBeCloseTo(0.8, 12);
    expect(curveIn(' cubic-bezier(.2, 1.4, .4, 1)')(0.35)).toBeCloseTo(1.025, 12);
    expect(curveIn('cubic-bezier(0.65,0,0.35,1)')(0.5)).toBeCloseTo(0.5, 12);
  });

  it('reads linear', () => {
    expect(curveIn('linear')(0.25)).toBe(0.25);
  });

  it('refuses what is not a curve', () => {
    expect(() => curveIn('')).toThrow('not a CSS curve');
    expect(() => curveIn('700ms')).toThrow('not a CSS curve');
    expect(() => curveIn('cubic-bezier(.2, .8, .2)')).toThrow('not a CSS curve');
    expect(() => curveIn('cubic-bezier(.2, .8, .2, 1, 0)')).toThrow('not a CSS curve');
    expect(() => curveIn('cubic-bezier(.2, , .2, 1)')).toThrow('not a CSS curve');
    expect(() => curveIn('cubic-bezier(a, b, c, d)')).toThrow('not a CSS curve');
  });
});
