// IEEE 754 half-precision bit patterns, as the film viewer's R16F texture
// takes them: 1 sign bit, 5 exponent bits (bias 15) and 10 fraction bits,
// rounded to the nearest, ties to even.

const scratch = new DataView(new ArrayBuffer(4));

/** The half-float bits nearest to `value`. */
export function halfFloatBits(value: number): number {
  scratch.setFloat32(0, value);
  const bits = scratch.getUint32(0);
  const sign = (bits >>> 16) & 0x8000;
  const exponent = (bits >>> 23) & 0xff;
  const fraction = bits & 0x7fffff;

  if (exponent === 0xff) return sign | 0x7c00 | (fraction === 0 ? 0 : 0x200); // infinity or NaN
  const halfExponent = exponent - 127 + 15;
  if (halfExponent >= 0x1f) return sign | 0x7c00; // too large: infinity

  if (halfExponent <= 0) {
    // Subnormal, or zero: shift the fraction, with its leading one, into place.
    if (halfExponent < -10) return sign;
    const full = fraction | 0x800000;
    const shift = 14 - halfExponent;
    const kept = full >>> shift;
    const rest = full & ((1 << shift) - 1);
    const halfway = 1 << (shift - 1);
    const roundUp = rest > halfway || (rest === halfway && (kept & 1) === 1);
    return sign | (kept + (roundUp ? 1 : 0));
  }

  const kept = fraction >>> 13;
  const rest = fraction & 0x1fff;
  const roundUp = rest > 0x1000 || (rest === 0x1000 && (kept & 1) === 1);
  // A carry out of the fraction rolls into the exponent, as it should.
  return sign | ((halfExponent << 10) + kept + (roundUp ? 1 : 0));
}
