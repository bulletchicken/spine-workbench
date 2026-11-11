// The pointer on a film, turned into what the camera should do. The wheel
// zooms at the pointer, a drag pans, a double-click fits. These are
// intents: the viewer hands them to whoever holds the cameras, so the
// same gestures move a linked partner too.
//
// A press with a tool in hand is shared with the tool by one rule. Where
// the tool's cursor says something here can be grabbed (`grab`), the
// press is the tool's at once, for a drag. Anywhere else the press waits:
// released within DRAG_SLOP screen pixels it is a click and goes to the
// tool; moved further, it pans and the tool never hears of it.

import type { ScreenPoint } from '../film-space';

export type CameraIntent = { kind: 'zoom'; factor: number; at: ScreenPoint } | { kind: 'pan'; by: ScreenPoint } | { kind: 'fit' };

/** How far a press may wander, in screen pixels, and still be a click. */
export const DRAG_SLOP = 4;

/** Zoom per wheel pixel: one notch of a mouse wheel (100 pixels) zooms 16%. */
const WHEEL_RATE = Math.log(1.16) / 100;
/** A pinch on a trackpad arrives as small wheel steps with Ctrl held, and should feel as quick as the fingers. */
const PINCH_RATE = 0.01;
/** Wheel steps counted in lines or pages, in pixels. */
const LINE = 40;
const PAGE = 800;

/** The zoom factor for one wheel event. `deltaMode` is the event's own: 0 pixels, 1 lines, 2 pages. */
export function wheelZoom(deltaY: number, deltaMode: number, pinch: boolean): number {
  const pixels = deltaY * (deltaMode === 1 ? LINE : deltaMode === 2 ? PAGE : 1);
  return Math.exp(-pixels * (pinch ? PINCH_RATE : WHEEL_RATE));
}

/** True when a tool's cursor says a press here grabs something on the film. */
export function toolGrabs(cursor: string): boolean {
  return cursor === 'grab' || cursor === 'grabbing';
}

/** A press of the primary button that is not the tool's: still a click, or already a pan. */
export type CameraPress = { kind: 'undecided'; from: ScreenPoint } | { kind: 'panning'; last: ScreenPoint };

/** Where a press stands after the pointer moved to `to`, and how far to pan for this move. */
export function pressMoved(press: CameraPress, to: ScreenPoint): { press: CameraPress; pan: ScreenPoint | null } {
  if (press.kind === 'panning') return { press: { kind: 'panning', last: to }, pan: [to[0] - press.last[0], to[1] - press.last[1]] };
  if (Math.hypot(to[0] - press.from[0], to[1] - press.from[1]) <= DRAG_SLOP) return { press, pan: null };
  // The whole way from the press, so the film stays under the pointer.
  return { press: { kind: 'panning', last: to }, pan: [to[0] - press.from[0], to[1] - press.from[1]] };
}
