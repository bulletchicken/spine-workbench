import { describe, expect, it } from 'vitest';
import { boxWidthsFor, gaussianBlur } from './blur';
import { fillCoverage, strokeCoverage, type Coverage } from './coverage';
import { Path } from './path';
import { WhiteLayer } from './white-layer';

const total = ({ cover }: Coverage) => cover.reduce((sum, value) => sum + value, 0);
const at = ({ x0, y0, width, cover }: Coverage, x: number, y: number) => cover[(y - y0) * width + (x - x0)] ?? 0;

function box(x0: number, y0: number, x1: number, y1: number, clockwise = true): Path {
  const path = new Path();
  path.moveTo(x0, y0);
  if (clockwise) {
    path.lineTo(x1, y0);
    path.lineTo(x1, y1);
    path.lineTo(x0, y1);
  } else {
    path.lineTo(x0, y1);
    path.lineTo(x1, y1);
    path.lineTo(x1, y0);
  }
  path.closePath();
  return path;
}

describe('filling', () => {
  it('covers whole pixels wholly and cut pixels by the part inside', () => {
    // A box from (2, 3) to (6.5, 5): pixels 2 to 5 of rows 3 and 4 whole, pixel 6 half.
    const cover = fillCoverage(box(2, 3, 6.5, 5), 20, 20);
    expect(at(cover, 2, 3)).toBe(1);
    expect(at(cover, 5, 4)).toBe(1);
    expect(at(cover, 6, 3)).toBeCloseTo(0.5, 6);
    expect(total(cover)).toBeCloseTo(4.5 * 2, 6);
  });

  it('gives a disc the area of a circle', () => {
    const path = new Path();
    path.arc(50.3, 40.7, 20, 0, Math.PI * 2);
    expect(Math.abs(total(fillCoverage(path, 100, 100)) - Math.PI * 400) / (Math.PI * 400)).toBeLessThan(0.005);
  });

  it('follows the nonzero rule: two boxes the same way round cover once, opposite ways leave a hole', () => {
    const same = new Path();
    const opposite = new Path();
    for (const [path, clockwise] of [
      [same, true],
      [opposite, false],
    ] as const) {
      for (const piece of [box(0, 0, 10, 10), box(4, 4, 6, 6, clockwise)]) path.subpaths.push(...piece.subpaths);
    }
    expect(total(fillCoverage(same, 20, 20))).toBeCloseTo(100, 6);
    expect(total(fillCoverage(opposite, 20, 20))).toBeCloseTo(96, 6);
  });

  it('stops at the film edge', () => {
    expect(total(fillCoverage(box(-5, -5, 3, 2), 20, 20))).toBeCloseTo(3 * 2, 6);
  });
});

describe('stroking', () => {
  it('draws a line its width wide with round caps', () => {
    const path = new Path();
    path.moveTo(10, 10.5);
    path.lineTo(30, 10.5);
    const cover = strokeCoverage(path, 4, 50, 50);
    // Pixel centres up to 1.5 from the line are inside, 2 is on the edge, 3 is outside.
    expect(at(cover, 20, 10)).toBe(1);
    expect(at(cover, 20, 9)).toBe(1);
    expect(at(cover, 20, 11)).toBe(1);
    expect(at(cover, 20, 8)).toBeCloseTo(0.5, 6);
    expect(at(cover, 20, 12)).toBeCloseTo(0.5, 6);
    expect(at(cover, 20, 7)).toBe(0);
    expect(at(cover, 20, 13)).toBe(0);
    const area = 20 * 4 + Math.PI * 2 * 2;
    expect(Math.abs(total(cover) - area) / area).toBeLessThan(0.05);
  });

  it('closes a closed path', () => {
    const open = box(5, 5, 25, 25);
    open.subpaths.forEach((subpath) => (subpath.closed = false));
    const closed = box(5, 5, 25, 25);
    // The closing side adds about 20 × 2 pixels.
    expect(total(strokeCoverage(closed, 2, 40, 40)) - total(strokeCoverage(open, 2, 40, 40))).toBeGreaterThan(35);
  });
});

describe('compositing white', () => {
  it('lays white over, adds it, and takes it away as a canvas does', () => {
    const layer = new WhiteLayer(4, 4);
    layer.fillAlpha = 0.5;
    const square = () => {
      layer.beginPath();
      layer.moveTo(0, 0);
      layer.lineTo(4, 0);
      layer.lineTo(4, 4);
      layer.lineTo(0, 4);
      layer.closePath();
      layer.fill();
    };
    square();
    square();
    expect(layer.white[5]).toBeCloseTo(0.75, 6); // 0.5 + 0.5 × (1 − 0.5)
    layer.composite = 'lighter';
    square();
    expect(layer.white[5]).toBeCloseTo(1, 6); // 0.75 + 0.5, held at 1
    layer.composite = 'destination-out';
    layer.fillAlpha = 0.25;
    square();
    expect(layer.white[5]).toBeCloseTo(0.75, 6); // 1 × (1 − 0.25)
  });
});

describe('blurring', () => {
  it('chooses box widths whose spread matches the Gaussian', () => {
    // Worked by hand from Kovesi's formula.
    expect(boxWidthsFor(1.1)).toEqual([1, 3, 3]);
    expect(boxWidthsFor(17)).toEqual([33, 33, 35]);
    for (const sigma of [1.1, 2.5, 9, 17]) {
      const variance = boxWidthsFor(sigma).reduce((sum, w) => sum + (w * w - 1) / 12, 0);
      expect(Math.abs(Math.sqrt(variance) - sigma) / sigma).toBeLessThan(0.1);
    }
  });

  it('keeps the total away from the edges and spreads a point by sigma', () => {
    const size = 121;
    const values = new Float32Array(size * size);
    values[60 * size + 60] = 1;
    const blurred = gaussianBlur(values, size, size, 5);
    let sum = 0;
    let spread = 0;
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const v = blurred[y * size + x] ?? 0;
        sum += v;
        spread += v * (x - 60) * (x - 60);
      }
    }
    expect(sum).toBeCloseTo(1, 5);
    expect(Math.abs(Math.sqrt(spread) - 5)).toBeLessThan(0.3);
    expect(blurred[60 * size + 60]).toBeGreaterThan(blurred[60 * size + 65] ?? 1);
  });
});
