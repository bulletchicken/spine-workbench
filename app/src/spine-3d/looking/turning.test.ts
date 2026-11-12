import { describe, expect, it } from 'vitest';
import { HIGHEST, LOWEST, ZOOM_IN, ZOOM_OUT, makeTurning } from './turning';

const start = { azimuth: 34, elevation: 14, zoom: 1 };

describe('turning the spine by hand', () => {
  it('turns with a drag: 0.4 degrees a pixel round, 0.3 over, the spine following the hand', () => {
    const turning = makeTurning(start);
    turning.press(1, 100, 100, 0);
    expect(turning.move(1, 150, 80, 16)).toBe(true);
    // Dragging right turns the near side of the spine to the right: the view goes round the other way.
    expect(turning.now()).toEqual({ azimuth: 34 - 20, elevation: 14 - 6, zoom: 1 });
    expect(turning.held()).toBe(true);
  });

  it('cannot be tipped past straight above or far below the horizon', () => {
    const turning = makeTurning(start);
    turning.press(1, 0, 0, 0);
    turning.move(1, 0, 1000, 16);
    expect(turning.now().elevation).toBe(HIGHEST);
    turning.move(1, 0, -2000, 32);
    expect(turning.now().elevation).toBe(LOWEST);
  });

  it('glides after a throw, slowing until it stops', () => {
    const turning = makeTurning(start);
    turning.press(1, 100, 100, 0);
    turning.move(1, 120, 100, 16);
    turning.release(1, 20);
    const thrown = turning.now().azimuth;
    expect(turning.glide(16)).toBe(true);
    const first = thrown - turning.now().azimuth;
    turning.glide(16);
    const second = thrown - first - turning.now().azimuth;
    expect(first).toBeGreaterThan(0);
    expect(second).toBeLessThan(first);
    // After a second, a twenty-fifth of the speed is left; after ten, nothing moves.
    for (let i = 0; i < 700; i += 1) turning.glide(16);
    expect(turning.glide(16)).toBe(false);
  });

  it('does not glide when the pointer was held still before letting go, or while one is down', () => {
    const turning = makeTurning(start);
    turning.press(1, 100, 100, 0);
    turning.move(1, 160, 100, 16);
    expect(turning.glide(16)).toBe(false);
    turning.release(1, 200);
    expect(turning.glide(16)).toBe(false);
  });

  it('tells a tap from a drag', () => {
    const turning = makeTurning(start);
    turning.press(1, 100, 100, 0);
    turning.move(1, 102, 101, 10);
    expect(turning.release(1, 20)).toEqual({ tapped: true });
    turning.press(1, 100, 100, 100);
    turning.move(1, 130, 100, 110);
    expect(turning.release(1, 120)).toEqual({ tapped: false });
    expect(turning.release(7, 130)).toEqual({ tapped: false });
  });

  it('zooms with the wheel and with a pinch, from 0.7 to 2.2 times', () => {
    const turning = makeTurning(start);
    turning.wheel(-100);
    expect(turning.now().zoom).toBeCloseTo(Math.exp(0.15), 12);
    turning.wheel(-100000);
    expect(turning.now().zoom).toBe(ZOOM_IN);
    turning.wheel(100000);
    expect(turning.now().zoom).toBe(ZOOM_OUT);

    turning.set(start);
    turning.press(1, 100, 100, 0);
    turning.press(2, 200, 100, 0);
    turning.move(2, 250, 100, 16);
    expect(turning.now().zoom).toBeCloseTo(1.5, 12);
    // A pinch does not turn the spine.
    expect(turning.now().azimuth).toBe(34);
    expect(turning.release(2, 30)).toEqual({ tapped: false });
  });

  it('starts again from a view set from outside, with no glide left over', () => {
    const turning = makeTurning(start);
    turning.press(1, 100, 100, 0);
    turning.move(1, 140, 100, 16);
    turning.release(1, 20);
    turning.set({ azimuth: 0, elevation: 0, zoom: 1 });
    expect(turning.glide(16)).toBe(false);
    expect(turning.now()).toEqual({ azimuth: 0, elevation: 0, zoom: 1 });
  });
});
