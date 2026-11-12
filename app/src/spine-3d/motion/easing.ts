// Small helpers for whoever animates the 3D view. The view itself never
// moves anything over time: every value is set from outside, each frame if
// need be. These turn one progress number into the values to set.

import { cubicBezier, type Curve } from '@/choreography/curve';
import { shortWayRound } from '../looking/named-views';
import type { View } from '../looking/view';

/** The curve things settle on: eases out and comes to rest, nothing bounces. */
export const settle: Curve = cubicBezier(0.22, 1, 0.36, 1);

/** How long the camera takes to move to another view, in milliseconds. */
export const VIEW_MOVE_MS = 600;

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

/** How far a stretch of a longer motion has come: 0 before `from`, 1 after `to`, both shares of the whole. */
export function stretch(progress: number, from: number, to: number): number {
  return to <= from ? (progress >= to ? 1 : 0) : clamp01((progress - from) / (to - from));
}

/**
 * One item's own progress within a motion that runs through `count` items
 * one after another, each starting a little after the one before. `overlap`
 * is how many items are moving at once: 1 is strictly one at a time.
 */
export function inTurn(progress: number, index: number, count: number, overlap = 3): number {
  if (count <= 1) return clamp01(progress);
  const each = overlap / (count - 1 + overlap);
  const starts = index / (count - 1 + overlap);
  return stretch(progress, starts, starts + each);
}

const between = (a: number, b: number, share: number) => a + (b - a) * share;

/** The view `share` of the way from one view to another, turning the short way round. */
export function viewBetween(from: View, to: View, share: number): View {
  return {
    azimuth: between(from.azimuth, shortWayRound(from.azimuth, to.azimuth), share),
    elevation: between(from.elevation, to.elevation, share),
    perspective: between(from.perspective, to.perspective, share),
    pxPerMm: [between(from.pxPerMm[0], to.pxPerMm[0], share), between(from.pxPerMm[1], to.pxPerMm[1], share)],
    aim: [between(from.aim[0], to.aim[0], share), between(from.aim[1], to.aim[1], share), between(from.aim[2], to.aim[2], share)],
    aimAt: [between(from.aimAt[0], to.aimAt[0], share), between(from.aimAt[1], to.aimAt[1], share)],
  };
}
