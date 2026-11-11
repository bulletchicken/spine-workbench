import { describe, expect, it } from 'vitest';
import { floatToHalf, halfFloatValues, halfToFloat } from './half-float';

// Bit patterns from the IEEE 754 binary16 table, worked by hand:
// sign 1 bit, exponent 5 bits (bias 15), fraction 10 bits.
const KNOWN: [number, number][] = [
  [0x0000, 0],
  [0x3c00, 1],
  [0x3800, 0.5],
  [0x3555, 0.333251953125],
  [0xc000, -2],
  [0x7bff, 65504],
  [0x0400, 2 ** -14],
  [0x03ff, 2 ** -14 - 2 ** -24],
  [0x0001, 2 ** -24],
  [0x7c00, Infinity],
  [0xfc00, -Infinity],
];

describe('half floats', () => {
  it('reads the known bit patterns', () => {
    for (const [bits, value] of KNOWN) expect(halfToFloat(bits)).toBe(value);
    expect(halfToFloat(0x7e00)).toBeNaN();
    expect(Object.is(halfToFloat(0x8000), -0)).toBe(true);
  });

  it('writes the known bit patterns', () => {
    for (const [bits, value] of KNOWN) expect(floatToHalf(value)).toBe(bits);
    expect(floatToHalf(Number.NaN) & 0x7c00).toBe(0x7c00);
  });

  it('rounds to the nearest half, ties to even', () => {
    // 0.1 lies between 0x2e66 (0.0999755859375) and 0x2e67 (0.10003662109375), nearer the first.
    expect(floatToHalf(0.1)).toBe(0x2e66);
    // Halfway between 1 and the next half (1 + 2^-10) goes to the even one, 1.
    expect(floatToHalf(1 + 2 ** -11)).toBe(0x3c00);
    // Halfway between 1 + 2^-10 (odd) and 1 + 2^-9 (even) goes up.
    expect(floatToHalf(1 + 3 * 2 ** -11)).toBe(0x3c02);
    // 65520 is halfway between 65504 and the next step, 65536, which overflows.
    expect(floatToHalf(65520)).toBe(0x7c00);
    expect(floatToHalf(65519)).toBe(0x7bff);
    // Half the smallest subnormal ties to even: zero. A little more rounds up.
    expect(floatToHalf(2 ** -25)).toBe(0x0000);
    expect(floatToHalf(2 ** -25 * 1.5)).toBe(0x0001);
  });

  it('round-trips every half float there is', () => {
    const values = halfFloatValues();
    for (let bits = 0; bits < 0x10000; bits += 1) {
      const isNaN = (bits & 0x7c00) === 0x7c00 && (bits & 0x3ff) !== 0;
      if (isNaN) continue;
      expect(floatToHalf(halfToFloat(bits))).toBe(bits);
      expect(values[bits]).toBe(halfToFloat(bits));
    }
  });
});
