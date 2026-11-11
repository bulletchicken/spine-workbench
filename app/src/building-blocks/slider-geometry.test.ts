// The slider's arithmetic against numbers worked out by hand. The range
// is three parts, 0 to 2, 2 to 3 and 3 to 7, drawn as bars from 0 to 100,
// 110 to 160 and 170 to 370 on screen: 50, 50 and 50 pixels to the unit,
// with gaps of 10 pixels between them.

import { describe, expect, it } from 'vitest';
import { filledPart, segmentAt, valueAfterKey, valueAtPointer, type BarEdges, type SliderSegment } from './slider-geometry';

const PARTS: SliderSegment[] = [
  { from: 0, to: 2 },
  { from: 2, to: 3 },
  { from: 3, to: 7 },
];
const BARS: BarEdges[] = [
  { left: 0, right: 100 },
  { left: 110, right: 160 },
  { left: 170, right: 370 },
];

describe('the value under the pointer', () => {
  it('is the bar’s share of its part, so the fill ends under the pointer', () => {
    expect(valueAtPointer(50, BARS, PARTS)).toBe(1);
    expect(valueAtPointer(135, BARS, PARTS)).toBe(2.5);
    expect(valueAtPointer(270, BARS, PARTS)).toBe(5);
    expect(valueAtPointer(320, BARS, PARTS)).toBe(6);
  });

  it('is the shared end anywhere in the gap between two bars', () => {
    expect(valueAtPointer(101, BARS, PARTS)).toBe(2);
    expect(valueAtPointer(109, BARS, PARTS)).toBe(2);
    expect(valueAtPointer(165, BARS, PARTS)).toBe(3);
  });

  it('is exact at each bar’s own edges', () => {
    expect(valueAtPointer(0, BARS, PARTS)).toBe(0);
    expect(valueAtPointer(100, BARS, PARTS)).toBe(2);
    expect(valueAtPointer(110, BARS, PARTS)).toBe(2);
    expect(valueAtPointer(160, BARS, PARTS)).toBe(3);
    expect(valueAtPointer(370, BARS, PARTS)).toBe(7);
  });

  it('stops at the ends of the slider', () => {
    expect(valueAtPointer(-40, BARS, PARTS)).toBe(0);
    expect(valueAtPointer(999, BARS, PARTS)).toBe(7);
  });

  it('works for one part, whose bar need not start at zero', () => {
    expect(valueAtPointer(300, [{ left: 200, right: 600 }], [{ from: 0, to: 100 }])).toBe(25);
  });
});

describe('how much of a part is filled', () => {
  it('is nothing before the part, all of it after, and the share in between', () => {
    const part = { from: 2, to: 6 };
    expect(filledPart(part, 1)).toBe(0);
    expect(filledPart(part, 2)).toBe(0);
    expect(filledPart(part, 3)).toBe(0.25);
    expect(filledPart(part, 5)).toBe(0.75);
    expect(filledPart(part, 6)).toBe(1);
    expect(filledPart(part, 9)).toBe(1);
  });
});

describe('the part a value is in', () => {
  it('gives a shared end to the later part, and the range’s end to the last', () => {
    expect(segmentAt(PARTS, 0)).toBe(0);
    expect(segmentAt(PARTS, 1.9)).toBe(0);
    expect(segmentAt(PARTS, 2)).toBe(1);
    expect(segmentAt(PARTS, 3)).toBe(2);
    expect(segmentAt(PARTS, 7)).toBe(2);
  });
});

describe('where a key takes the slider', () => {
  it('moves one step with the arrows, either pair', () => {
    expect(valueAfterKey('ArrowRight', 2.5, 0.5, PARTS)).toBe(3);
    expect(valueAfterKey('ArrowUp', 2.5, 0.5, PARTS)).toBe(3);
    expect(valueAfterKey('ArrowLeft', 2.5, 0.5, PARTS)).toBe(2);
    expect(valueAfterKey('ArrowDown', 2.5, 0.5, PARTS)).toBe(2);
  });

  it('never leaves the range', () => {
    expect(valueAfterKey('ArrowRight', 6.8, 0.5, PARTS)).toBe(7);
    expect(valueAfterKey('ArrowLeft', 0.2, 0.5, PARTS)).toBe(0);
  });

  it('goes to the ends with Home and End', () => {
    expect(valueAfterKey('Home', 4, 0.5, PARTS)).toBe(0);
    expect(valueAfterKey('End', 4, 0.5, PARTS)).toBe(7);
  });

  it('jumps to the next part’s start with Page Up, and to the end from the last part', () => {
    expect(valueAfterKey('PageUp', 0, 0.5, PARTS)).toBe(2);
    expect(valueAfterKey('PageUp', 2, 0.5, PARTS)).toBe(3);
    expect(valueAfterKey('PageUp', 2.5, 0.5, PARTS)).toBe(3);
    expect(valueAfterKey('PageUp', 5, 0.5, PARTS)).toBe(7);
  });

  it('jumps back to the start of this part with Page Down, and from a start to the one before', () => {
    expect(valueAfterKey('PageDown', 5, 0.5, PARTS)).toBe(3);
    expect(valueAfterKey('PageDown', 3, 0.5, PARTS)).toBe(2);
    expect(valueAfterKey('PageDown', 2, 0.5, PARTS)).toBe(0);
    expect(valueAfterKey('PageDown', 0, 0.5, PARTS)).toBe(0);
  });

  it('has no answer for any other key', () => {
    expect(valueAfterKey('Enter', 4, 0.5, PARTS)).toBeNull();
    expect(valueAfterKey('a', 4, 0.5, PARTS)).toBeNull();
    expect(valueAfterKey('Tab', 4, 0.5, PARTS)).toBeNull();
  });
});
