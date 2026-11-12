// Reads the design's durations and curves for motion that script drives.
// The values are the same custom properties CSS uses, read where a part
// stands, so reduced motion shortens script-driven motion too: under
// `data-motion="reduced"` an outline's refit reads 1 ms.
//
// A camera or a bay is the exception. Under reduced motion it does not
// travel for a shorter time, it lands at once: whatever moves one checks
// the motion mode and skips the glide, and does not read a duration here.

import { cubicBezier, linear, type Curve } from './curve';

/**
 * A duration or a gap: a token such as `--dur-push`, `--delay-in` or `--hold-leave`, an
 * earlier part's `--t-slow`, or one of the named timings.
 */
export type DurationName = `--t-${string}` | `--dur-${string}` | `--delay-${string}` | `--hold-${string}` | `--stagger${string}` | '--breathe';

/** One of the design's plain numbers: how far a film is pushed in, the light's measures, a film's height. */
export type NumberName = `--zoom-${string}` | `--light-${string}` | '--name-room' | '--film-h';

/** One of the design's curves, such as `--ease-in-out`. */
export type CurveName = `--ease-${string}`;

/** A CSS time, `700ms` or `2.6s`, as milliseconds. */
export function millisecondsIn(time: string): number {
  const read = /^(-?(?:\d+\.?\d*|\.\d+))(ms|s)$/.exec(time.trim());
  if (read === null) throw new Error(`not a CSS time: "${time}"`);
  return Number(read[1]) * (read[2] === 's' ? 1000 : 1);
}

/** A plain CSS number, `1.3` or `0.05`, as a number. */
function numberIn(value: string): number {
  const text = value.trim();
  if (!/^-?(?:\d+\.?\d*|\.\d+)$/.test(text)) throw new Error(`not a CSS number: "${value}"`);
  return Number(text);
}

/** A CSS easing, `cubic-bezier(.2, .8, .2, 1)` or `linear`, as a function. */
export function curveIn(easing: string): Curve {
  const text = easing.trim();
  if (text === 'linear') return linear;
  const inside = /^cubic-bezier\(([^)]*)\)$/.exec(text)?.[1];
  const [x1, y1, x2, y2, ...more] = (inside ?? '').split(',').map((point) => (point.trim() === '' ? Number.NaN : Number(point)));
  if (x1 === undefined || y1 === undefined || x2 === undefined || y2 === undefined || more.length > 0 || [x1, y1, x2, y2].some(Number.isNaN)) {
    throw new Error(`not a CSS curve: "${easing}"`);
  }
  return cubicBezier(x1, y1, x2, y2);
}

/**
 * How long a named duration lasts where `part` stands, in milliseconds.
 * Read it once when a motion starts, not on every frame.
 */
export function durationOf(part: Element, name: DurationName): number {
  return millisecondsIn(getComputedStyle(part).getPropertyValue(name));
}

/** A named curve where `part` stands, as a function. Read it once when a motion starts. */
export function curveOf(part: Element, name: CurveName): Curve {
  return curveIn(getComputedStyle(part).getPropertyValue(name));
}

/** A named number where `part` stands. Read it once when a motion starts. */
export function numberOf(part: Element, name: NumberName): number {
  return numberIn(getComputedStyle(part).getPropertyValue(name));
}

/**
 * A named duration on the page itself, for script that has no part to stand
 * on: a store, or a run started at module scope.
 */
export function durationOnPage(name: DurationName): number {
  return durationOf(document.documentElement, name);
}

/** A named number on the page itself, for script that has no part to stand on. */
export function numberOnPage(name: NumberName): number {
  return numberOf(document.documentElement, name);
}
