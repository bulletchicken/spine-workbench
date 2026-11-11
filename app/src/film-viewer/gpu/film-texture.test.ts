import { describe, expect, it } from 'vitest';
import { shrinkToFit } from './film-texture';
import { floatToHalf, halfToFloat } from './half-float';

describe('a film too large for the GPU', () => {
  it('is left as it is when it fits', () => {
    const pixels = new Uint16Array(6);
    expect(shrinkToFit(pixels, { width: 3, height: 2 }, 3)).toEqual({ pixels, width: 3, height: 2, step: 1 });
  });

  it('becomes the mean of each square of pixels, cut short at the edge', () => {
    // Pixel (x, y) holds x + 5y on a 5 × 3 film. With at most 2 texels a side
    // the step is 3: texel (0, 0) covers x 0–2, y 0–2, mean 1 + 5 · 1 = 6;
    // texel (1, 0) covers x 3–4, y 0–2, mean 3.5 + 5 = 8.5.
    const pixels = new Uint16Array(15);
    for (let y = 0; y < 3; y += 1) for (let x = 0; x < 5; x += 1) pixels[y * 5 + x] = floatToHalf(x + 5 * y);
    const fitted = shrinkToFit(pixels, { width: 5, height: 3 }, 2);
    expect(fitted.step).toBe(3);
    expect([fitted.width, fitted.height]).toEqual([2, 1]);
    expect([...fitted.pixels].map(halfToFloat)).toEqual([6, 8.5]);
  });
});
