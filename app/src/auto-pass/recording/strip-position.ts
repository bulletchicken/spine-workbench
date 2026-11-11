// Where a film point sits along the strip: the row whose centre on the
// film is nearest to it. Rows are half a millimetre apart along the
// spine, so for a point on the spine this is its row to within one.

import type { Px } from '@/shapes';
import type { StripShape } from './pass-recording';

/** The strip row nearest a film point, or null for a strip with no rows; the first of equals wins. */
export function nearestRow(shape: StripShape, point: Px): number | null {
  let best: number | null = null;
  let bestDistance = Infinity;
  for (const [row, [x, y]] of shape.centres.entries()) {
    const distance = (x - point[0]) ** 2 + (y - point[1]) ** 2;
    if (distance < bestDistance) {
      best = row;
      bestDistance = distance;
    }
  }
  return best;
}
