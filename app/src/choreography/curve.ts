// A CSS cubic-bezier curve as a function, so motion driven by script eases
// exactly as motion driven by CSS does.

/** How much of the way a motion has come, given how much of its time has passed: both from 0 to 1. */
export type Curve = (time: number) => number;

/** One coordinate of the curve at parameter `u`, given that coordinate of its two control points. */
function along(u: number, p1: number, p2: number): number {
  const v = 1 - u;
  return 3 * v * v * u * p1 + 3 * v * u * u * p2 + u * u * u;
}

/**
 * The curve CSS writes as `cubic-bezier(x1, y1, x2, y2)`.
 *
 * As in CSS, x1 and x2 lie between 0 and 1, so the curve never turns back
 * in time; y1 and y2 may go outside, which is how a curve overshoots. Time
 * before 0 answers 0 and time after 1 answers 1.
 */
export function cubicBezier(x1: number, y1: number, x2: number, y2: number): Curve {
  // Asked as "is it inside", so that a point that is not a number is refused too.
  if (!(x1 >= 0 && x1 <= 1 && x2 >= 0 && x2 <= 1)) throw new RangeError('a cubic-bezier curve needs x1 and x2 from 0 to 1');
  if (!Number.isFinite(y1) || !Number.isFinite(y2)) throw new RangeError('a cubic-bezier curve needs y1 and y2 to be numbers');
  return (time) => {
    if (time <= 0) return 0;
    if (time >= 1) return 1;
    // x rises with u, so halving the interval finds the u at which x is
    // `time`. Fifty halvings leave nothing a double can tell apart.
    let low = 0;
    let high = 1;
    for (let i = 0; i < 50; i += 1) {
      const middle = (low + high) / 2;
      if (along(middle, x1, x2) < time) low = middle;
      else high = middle;
    }
    return along((low + high) / 2, y1, y2);
  };
}

/** A straight line: no easing. */
export const linear: Curve = (time) => Math.min(1, Math.max(0, time));
