// A Gaussian blur of one channel, as a canvas's `blur()` filter gives:
// three box blurs across and three down, whose widths are chosen so their
// spread matches the Gaussian's (Kovesi, "Fast almost-Gaussian
// filtering", 2010). Outside the picture counts as empty.

/** The widths of three boxes whose spread is `sigma`. */
export function boxWidthsFor(sigma: number): [number, number, number] {
  const n = 3;
  const ideal = Math.sqrt((12 * sigma * sigma) / n + 1);
  let lower = Math.floor(ideal);
  if (lower % 2 === 0) lower--;
  const upper = lower + 2;
  const lowerCount = Math.round((12 * sigma * sigma - n * lower * lower - 4 * n * lower - 3 * n) / (-4 * lower - 4));
  const widths = [0, 1, 2].map((k) => (k < lowerCount ? lower : upper));
  return [widths[0] ?? lower, widths[1] ?? lower, widths[2] ?? lower];
}

/** One box blur along rows (`step` 1) or columns (`step` the row length), in place through `scratch`. */
function boxPass(values: Float32Array, scratch: Float32Array, lines: number, length: number, along: number, across: number, radius: number) {
  if (radius <= 0) return;
  const size = 2 * radius + 1;
  for (let line = 0; line < lines; line++) {
    const base = line * across;
    let sum = 0;
    for (let k = 0; k < radius && k < length; k++) sum += values[base + k * along] ?? 0;
    for (let k = 0; k < length; k++) {
      const entering = k + radius;
      const leaving = k - radius - 1;
      if (entering < length) sum += values[base + entering * along] ?? 0;
      if (leaving >= 0) sum -= values[base + leaving * along] ?? 0;
      scratch[k] = sum / size;
    }
    for (let k = 0; k < length; k++) values[base + k * along] = scratch[k] ?? 0;
  }
}

/** A blurred copy of a `width` × `height` channel. */
export function gaussianBlur(values: Float32Array, width: number, height: number, sigma: number): Float32Array {
  const out = Float32Array.from(values);
  if (sigma <= 0) return out;
  const scratch = new Float32Array(Math.max(width, height));
  for (const boxWidth of boxWidthsFor(sigma)) {
    const radius = (boxWidth - 1) / 2;
    boxPass(out, scratch, height, width, 1, width, radius);
    boxPass(out, scratch, width, height, width, 1, radius);
  }
  return out;
}
