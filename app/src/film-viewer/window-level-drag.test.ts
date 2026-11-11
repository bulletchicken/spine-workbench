import { describe, expect, it } from 'vitest';
import { lookAfterDrag, NARROWEST } from './window-level-drag';

const viewport = { width: 400, height: 800 };
const plain = { centre: 0.5, width: 1, flipped: false };

/** What a display value shows as, 0 black to 1 white: the shader's rule, written out again. */
function shade(look: typeof plain, value: number) {
  const s = Math.min(1, Math.max(0, (value - (look.centre - look.width / 2)) / look.width));
  return look.flipped ? 1 - s : s;
}

describe('right-drag for brightness and contrast', () => {
  it('widens the window by a quarter for a quarter of the width to the right', () => {
    expect(lookAfterDrag(plain, [100, 100], [200, 100], viewport)).toEqual({ centre: 0.5, width: 1.25, flipped: false });
  });

  it('narrows it to the left, but never to nothing', () => {
    expect(lookAfterDrag(plain, [300, 100], [200, 100], viewport).width).toBe(0.75);
    expect(lookAfterDrag(plain, [400, 100], [0, 100], viewport).width).toBe(NARROWEST);
  });

  it('brightens going up and darkens going down, flipped or not', () => {
    for (const flipped of [false, true]) {
      const look = { ...plain, flipped };
      const up = lookAfterDrag(look, [200, 400], [200, 200], viewport);
      const down = lookAfterDrag(look, [200, 400], [200, 600], viewport);
      expect(shade(up, 0.5)).toBeGreaterThan(shade(look, 0.5));
      expect(shade(down, 0.5)).toBeLessThan(shade(look, 0.5));
    }
    // A quarter of the height up moves an unflipped window's centre down by a quarter.
    expect(lookAfterDrag(plain, [200, 400], [200, 200], viewport).centre).toBe(0.25);
  });
});
