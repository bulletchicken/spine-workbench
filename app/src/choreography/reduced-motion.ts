// Whether motion is full or reduced. Two things can ask for less: the
// system's own setting, and the app's preference. Either is enough.

import type { MotionMode } from '@/look-and-feel/data-attributes';

/** The media query that is true while the system asks for reduced motion. */
export const SYSTEM_ASKS_FOR_REDUCED = '(prefers-reduced-motion: reduce)';

/** The value of `data-motion` on the app root: reduced when the system or the app asks for it. */
export function motionMode(systemAsksForReduced: boolean, appAsksForReduced: boolean): MotionMode {
  return systemAsksForReduced || appAsksForReduced ? 'reduced' : 'full';
}
