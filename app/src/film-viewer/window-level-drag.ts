// The right button on a film: drag across for contrast, up and down for
// brightness. The frozen tool contract leaves the right button to the
// viewer, so this works whatever tool is in hand.
//
// Across, to the right, widens the window (less contrast). Up makes the
// film brighter and down darker, with the brightness flip on or off.

import type { Viewport } from './camera/camera';
import type { ScreenPoint } from './film-space';
import type { FilmLook } from './gpu/film-shader';

/** A drag the viewport's whole width changes the window's width by this, in display units. */
export const WIDTH_PER_VIEWPORT = 1;
/** A drag the viewport's whole height moves the window's centre by this. */
export const CENTRE_PER_VIEWPORT = 1;
/** The narrowest window: about a 10-bit display step. */
export const NARROWEST = 1 / 1024;

/** The look after dragging from `from` to `to`, starting from `start`. */
export function lookAfterDrag(start: FilmLook, from: ScreenPoint, to: ScreenPoint, viewport: Viewport): FilmLook {
  const across = (to[0] - from[0]) / Math.max(1, viewport.width);
  const down = (to[1] - from[1]) / Math.max(1, viewport.height);
  // A higher centre darkens an unflipped film and brightens a flipped one.
  const darker = start.flipped ? -1 : 1;
  return {
    ...start,
    width: Math.max(NARROWEST, start.width + across * WIDTH_PER_VIEWPORT),
    centre: start.centre + darker * down * CENTRE_PER_VIEWPORT,
  };
}
