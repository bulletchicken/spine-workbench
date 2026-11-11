import { describe, expect, it } from 'vitest';
import type { VertebraMarks, WandClick } from '@/shapes';
import { sameJson } from './same-json';
import { auto } from './sketch-case';
import { answeredClicks, MOST_WAND_CLICKS, vertebraHeight, withClick, withoutSent, withSent, type WandSessions } from './wand-sessions';

const add = (x: number, y: number): WandClick => ({ at: [x, y], polarity: 'add' });
const subtract = (x: number, y: number): WandClick => ({ at: [x, y], polarity: 'subtract' });

describe('vertebraHeight', () => {
  it('is the mean of the two side edges', () => {
    // Left edge (0, 0) to (0, 50) is 50 long; right edge (80, 0) to (80, 30) is 30: the mean is 40.
    const marks: VertebraMarks = { kind: 'side', points: { supA: auto(0, 0), supP: auto(80, 0), infA: auto(0, 50), infP: auto(80, 30) } };
    expect(vertebraHeight(marks)).toBe(40);
  });

  it('measures a slanted edge along its length', () => {
    // Both edges run 30 across and 40 down: 50 long.
    const marks: VertebraMarks = { kind: 'bending', lines: { sup: [auto(0, 0), auto(80, 0)], inf: [auto(30, 40), auto(110, 40)] } };
    expect(vertebraHeight(marks)).toBe(50);
  });
});

describe('withClick', () => {
  // A vertebra 40 pixels tall: a change of mind reaches 0.15 × 40 = 6 pixels.
  const height = 40;

  it('adds the click last', () => {
    expect(withClick([add(1, 1)], add(50, 50), height)).toEqual([add(1, 1), add(50, 50)]);
  });

  it('replaces an older click of the other polarity in nearly the same place', () => {
    // 3 across and 4 down: 5 pixels away, inside 6.
    expect(withClick([add(10, 10), add(90, 90)], subtract(13, 14), height)).toEqual([add(90, 90), subtract(13, 14)]);
  });

  it('keeps an older click of the other polarity farther away, and one of the same polarity anywhere', () => {
    // 7 pixels away, beyond 6.
    expect(withClick([add(10, 10)], subtract(17, 10), height)).toEqual([add(10, 10), subtract(17, 10)]);
    expect(withClick([add(10, 10)], add(10, 10), height)).toEqual([add(10, 10), add(10, 10)]);
  });

  it(`keeps at most ${String(MOST_WAND_CLICKS)} clicks, dropping the oldest`, () => {
    const many = Array.from({ length: MOST_WAND_CLICKS }, (_, index) => add(index * 100, 0));
    const after = withClick(many, add(5000, 0), height);
    expect(after).toHaveLength(MOST_WAND_CLICKS);
    expect(after[0]).toEqual(add(100, 0));
    expect(after.at(-1)).toEqual(add(5000, 0));
  });
});

describe('requests waiting for an answer', () => {
  const sessions = (sent: WandSessions['sent'], asks = 3): WandSessions => ({ clicks: {}, sent, asks });

  it('are answered oldest first among those asked at the answer version', () => {
    const waiting = sessions({
      2: [
        { ask: 0, askedAt: 3, clicks: [add(1, 1)] },
        { ask: 1, askedAt: 4, clicks: [add(2, 2)] },
        { ask: 2, askedAt: 4, clicks: [add(3, 3)] },
      ],
    });
    expect(answeredClicks(waiting, 2, 4)).toEqual([add(2, 2)]);
    expect(answeredClicks(waiting, 2, 3)).toEqual([add(1, 1)]);
    expect(answeredClicks(waiting, 2, 5)).toBeNull();
    expect(answeredClicks(waiting, 7, 4)).toBeNull();
  });

  it('wait in the order they were asked, each with the next number, and those asked before the film last changed go', () => {
    const waiting = sessions({ 2: [{ ask: 0, askedAt: 3, clicks: [add(1, 1)] }, { ask: 1, askedAt: 5, clicks: [add(2, 2)] }] }, 2);
    const added = withSent(waiting, 2, { askedAt: 6, clicks: [add(3, 3)] }, 5);
    expect(added.ask).toBe(2);
    expect(added.sessions.asks).toBe(3);
    expect(added.sessions.sent[2]).toEqual([
      { ask: 1, askedAt: 5, clicks: [add(2, 2)] },
      { ask: 2, askedAt: 6, clicks: [add(3, 3)] },
    ]);
  });

  it('stop waiting by number when they get no answer, so the next one at that version is the one answered', () => {
    const waiting = sessions({ 2: [{ ask: 0, askedAt: 4, clicks: [add(1, 1)] }, { ask: 1, askedAt: 4, clicks: [add(2, 2)] }], 5: [{ ask: 2, askedAt: 4, clicks: [add(9, 9)] }] });
    const refused = withoutSent(waiting, 0);
    expect(answeredClicks(refused, 2, 4)).toEqual([add(2, 2)]);
    expect(refused.sent[5]).toEqual(waiting.sent[5]);
    expect(withoutSent(withoutSent(refused, 1), 2).sent).toEqual({});
    expect(withoutSent(waiting, 7)).toBe(waiting);
  });
});

describe('sameJson', () => {
  it('compares by value, whatever order the keys were written in', () => {
    expect(sameJson({ a: [1, { b: null }], c: 'x' }, { c: 'x', a: [1, { b: null }] })).toBe(true);
  });

  it('tells apart values that differ anywhere, in kind, length or keys', () => {
    expect(sameJson({ a: [1, 2] }, { a: [1, 2, 3] })).toBe(false);
    expect(sameJson({ a: 1 }, { a: 1, b: 1 })).toBe(false);
    expect(sameJson({ a: 1 }, { b: 1 })).toBe(false);
    expect(sameJson([1], { 0: 1 })).toBe(false);
    expect(sameJson(null, {})).toBe(false);
    expect(sameJson(0, '0')).toBe(false);
  });
});
