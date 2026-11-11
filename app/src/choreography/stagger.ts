// Where each thing stands in a cascade. A cascade is never a delay set from
// script: a part writes each item's place as `--i`, a whole number, and its
// CSS works out `animation-delay: calc(var(--i) * var(--stagger))`. So the
// gap between items stays a token, and reduced motion closes it.

import type { CSSProperties } from 'react';

/**
 * The place of each of `count` levels in a cascade that spreads from the
 * level that was edited: 0 for that level, 1 for the levels beside it, and
 * so on outward, so a change ripples away from where it was made.
 *
 * `editedAt` is a whole number, as every place then is. It may lie past
 * either end, as when the edited level is gone: the ripple then starts
 * that far from the nearest level.
 */
export function placesFromEdit(count: number, editedAt: number): number[] {
  if (!Number.isInteger(editedAt)) throw new RangeError('the edited level is a whole number');
  return Array.from({ length: count }, (_, level) => Math.abs(level - editedAt));
}

/** A place in a cascade as the style that carries it to CSS: `--i`. */
export function placeStyle(place: number): CSSProperties {
  return { '--i': place } as CSSProperties;
}
