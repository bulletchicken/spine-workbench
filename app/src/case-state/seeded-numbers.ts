// Seeded numbers for the random histories, so that fast-check's two
// numbers per step choose everything a step does and a failing history
// plays again the same way. Tests only.

import type { Px } from '@/shapes';
import { FILM_HEIGHT, FILM_WIDTH } from './sketch-case';

/** A small seeded random number generator (mulberry32): numbers from 0 to 1. */
export function randomNumbers(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function pick<T>(items: readonly T[], n: number): T | undefined {
  return items.length === 0 ? undefined : items[n % items.length];
}

export const between = (rng: () => number, low: number, high: number) => low + Math.floor(rng() * (high - low + 1));

/** A point on a film, off the pixel grid as a pointer would leave it. */
export function onFilm(rng: () => number): Px {
  return [between(rng, 0, FILM_WIDTH - 1) + 0.25, between(rng, 0, FILM_HEIGHT - 1) + 0.5];
}
