import { describe, expect, it } from 'vitest';
import { DRAG_SLOP, pressMoved, toolGrabs, wheelZoom, type CameraPress } from './gestures';

describe('the wheel', () => {
  it('zooms in 16% for a notch toward the screen and back out for one away', () => {
    expect(wheelZoom(-100, 0, false)).toBeCloseTo(1.16, 12);
    expect(wheelZoom(100, 0, false)).toBeCloseTo(1 / 1.16, 12);
    expect(wheelZoom(-100, 0, false) * wheelZoom(100, 0, false)).toBeCloseTo(1, 12);
  });

  it('counts lines and pages as pixels', () => {
    // Three lines are 120 pixels: 1.16 to the power 1.2.
    expect(wheelZoom(-3, 1, false)).toBeCloseTo(1.16 ** 1.2, 12);
    expect(wheelZoom(-1, 2, false)).toBeCloseTo(1.16 ** 8, 9);
  });

  it('follows a pinch more quickly', () => {
    // A pinch of 10 is e^0.1.
    expect(wheelZoom(-10, 0, true)).toBeCloseTo(Math.exp(0.1), 12);
  });
});

describe('a press', () => {
  const start: CameraPress = { kind: 'undecided', from: [100, 100] };

  it('stays a click while it wanders no more than the slop', () => {
    expect(pressMoved(start, [100 + DRAG_SLOP, 100])).toEqual({ press: start, pan: null });
    expect(pressMoved(start, [103, 102])).toEqual({ press: start, pan: null });
  });

  it('becomes a pan once it goes further, keeping every pixel it moved', () => {
    const first = pressMoved(start, [103, 104]);
    expect(first.pan).toEqual([3, 4]);
    expect(first.press).toEqual({ kind: 'panning', last: [103, 104] });
    // 5 pixels away, just past the slop: the whole 5 pans.
    const next = pressMoved(first.press, [110, 90]);
    expect(next.pan).toEqual([7, -14]);
  });

  it('is the tool’s at once where its cursor says something can be grabbed', () => {
    expect(toolGrabs('grab')).toBe(true);
    expect(toolGrabs('grabbing')).toBe(true);
    expect(toolGrabs('crosshair')).toBe(false);
    expect(toolGrabs('default')).toBe(false);
  });
});
