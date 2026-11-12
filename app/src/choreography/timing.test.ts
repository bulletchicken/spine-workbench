// Reading CSS times and curves. Each answer is the CSS value written out
// by hand: 2.6 s is 2600 ms; a curve is checked at the one point its
// definition gives without solving anything (see curve.test.ts).

import { describe, expect, it, vi } from 'vitest';
import { curveIn, durationOf, millisecondsIn, numberOf } from './timing';

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

  it('reads a --dur-, a --delay- and a --hold- token as any other, where a part stands', () => {
    // These tests run without a page, so the computed style is handed over as a browser would.
    const set: Record<string, string> = { '--dur-push': ' 800ms', '--delay-in': ' 100ms', '--hold-leave': ' 260ms' };
    vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: (name: string) => set[name] ?? '' }));
    const part = {} as Element;
    expect(durationOf(part, '--dur-push')).toBe(800);
    expect(durationOf(part, '--delay-in')).toBe(100);
    expect(durationOf(part, '--hold-leave')).toBe(260);
    vi.unstubAllGlobals();
  });

  it('refuses what is not a time, so a mistyped name fails loudly', () => {
    expect(() => millisecondsIn('')).toThrow('not a CSS time');
    expect(() => millisecondsIn('700')).toThrow('not a CSS time');
    expect(() => millisecondsIn('12px')).toThrow('not a CSS time');
    expect(() => millisecondsIn('cubic-bezier(.2, .8, .2, 1)')).toThrow('not a CSS time');
  });
});

describe('numberOf', () => {
  // Without a page, the computed style is handed over as a browser would, spaces and all.
  const reading = (value: string) => {
    vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: () => value }));
    try {
      return numberOf({} as Element, '--zoom-find');
    } finally {
      vi.unstubAllGlobals();
    }
  };

  it('reads the design’s plain numbers', () => {
    expect(reading(' 1.3')).toBe(1.3);
    expect(reading(' 0.05')).toBe(0.05);
  });

  it('refuses a length and an unset name, so a mistyped name fails loudly', () => {
    expect(() => reading(' 24px')).toThrow('not a CSS number');
    expect(() => reading('')).toThrow('not a CSS number');
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
