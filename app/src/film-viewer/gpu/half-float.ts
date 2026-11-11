// IEEE 754 half floats, the 16-bit numbers the engine hands the viewer for
// each film pixel. The GPU reads them directly; the viewer converts them
// only to make a smaller copy of a film the GPU cannot hold whole, and to
// draw its own test film.

const single = new Float32Array(1);
const singleBits = new Uint32Array(single.buffer);

/** The number a half float's bits stand for. */
export function halfToFloat(bits: number): number {
  const sign = bits & 0x8000 ? -1 : 1;
  const exponent = (bits >> 10) & 0x1f;
  const fraction = bits & 0x3ff;
  if (exponent === 0) return sign * fraction * 2 ** -24;
  if (exponent === 0x1f) return fraction === 0 ? sign * Infinity : Number.NaN;
  return sign * (1 + fraction / 1024) * 2 ** (exponent - 15);
}

/** The half float nearest `value`, ties to even, as bits. Too large becomes infinity. */
export function floatToHalf(value: number): number {
  single[0] = value;
  const bits = singleBits[0] ?? 0;
  const sign = (bits >>> 16) & 0x8000;
  const exponent = (bits >>> 23) & 0xff;
  const mantissa = bits & 0x7fffff;
  if (exponent === 0xff) return sign | 0x7c00 | (mantissa === 0 ? 0 : 0x200);
  const halfExponent = exponent - 127 + 15;
  if (halfExponent >= 0x1f) return sign | 0x7c00;
  if (halfExponent <= 0) {
    // Too small for a normal half: a subnormal one, or zero.
    if (halfExponent < -10) return sign;
    const shift = 14 - halfExponent;
    const whole = mantissa | 0x800000;
    return sign | roundedShift(whole, shift);
  }
  // A carry out of the fraction lands in the exponent, which is the right answer.
  return sign | ((halfExponent << 10) + roundedShift(mantissa, 13));
}

/** `value >> shift`, rounded to nearest with ties to even. */
function roundedShift(value: number, shift: number): number {
  const kept = value >>> shift;
  const rest = value & ((1 << shift) - 1);
  const halfway = 1 << (shift - 1);
  return rest > halfway || (rest === halfway && (kept & 1) === 1) ? kept + 1 : kept;
}

let table: Float32Array | null = null;

/** Every half float's value, by its bits: a faster `halfToFloat` for whole films. */
export function halfFloatValues(): Float32Array {
  if (table === null) {
    table = new Float32Array(0x10000);
    for (let bits = 0; bits < 0x10000; bits += 1) table[bits] = halfToFloat(bits);
  }
  return table;
}
