// Where a film sits in its viewport, as pure functions over the frozen
// FilmPlacement: fit it, zoom at the pointer, pan, travel to a point, and
// keep it inside the viewport. Nothing here holds state or reads a clock.
//
// Every function returns a placement that `keepInside` has already
// judged, so a camera can never show the room around a zoomed film:
// - zoom stays between the fit and MAX_ZOOM_OVER_FIT times the fit;
// - along an axis where the zoomed film is larger than the box inside
//   the margins, the film covers that box;
// - along an axis where it is smaller, it is centred in the viewport and
//   kept inside the margins, exactly as `fit` places it.

import type { Px } from '@/shapes';
import { filmSpace, uprightSize, type FilmPlacement, type FilmSize, type FilmView, type ScreenPoint } from '../film-space';

/** The viewer's size in CSS pixels. */
export interface Viewport {
  width: number;
  height: number;
}

/** Screen pixels a fitted film keeps clear on each side, for an edge ruler or a flag. */
export interface Margins {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

/** Everything a camera knows about a film besides where it is looking. */
export interface CameraFrame {
  size: FilmSize;
  view: FilmView;
  viewport: Viewport;
  margins: Margins;
}

/** The closest a camera goes: six times the fit. */
export const MAX_ZOOM_OVER_FIT = 6;

/** Where a travelled-to point lands, as a fraction of the viewport's height from its top. */
export const LOOK_HEIGHT = 0.47;

/** Zooms this close to the fit are the fit. */
const FIT_TOLERANCE = 1e-9;

/** The smallest zoom a viewport with no room left inside its margins gets, so the placement stays invertible. */
const LEAST_ZOOM = 1e-6;

/** Screen pixels per film pixel that show the whole upright film inside the margins. */
export function fitZoom(frame: CameraFrame): number {
  const upright = uprightSize(frame.size, frame.view.quarterTurns);
  const { viewport, margins } = frame;
  const across = (viewport.width - margins.left - margins.right) / upright.width;
  const down = (viewport.height - margins.top - margins.bottom) / upright.height;
  return Math.max(LEAST_ZOOM, Math.min(across, down));
}

/** The whole film, as large as the margins allow, centred. */
export function fit(frame: CameraFrame): FilmPlacement {
  const zoom = fitZoom(frame);
  const upright = uprightSize(frame.size, frame.view.quarterTurns);
  const { viewport, margins } = frame;
  return place(frame, zoom, [
    centred(upright.width * zoom, viewport.width, margins.left, margins.right),
    centred(upright.height * zoom, viewport.height, margins.top, margins.bottom),
  ]);
}

/** The zoom as a multiple of the fit: 1 at fit, up to MAX_ZOOM_OVER_FIT. */
export function zoomOverFit(frame: CameraFrame, placement: FilmPlacement): number {
  return placement.zoom / fitZoom(frame);
}

/** True when the placement shows the film at its fit. */
export function isFit(frame: CameraFrame, placement: FilmPlacement): boolean {
  return Math.abs(zoomOverFit(frame, placement) - 1) <= FIT_TOLERANCE;
}

/** The placement with its zoom and pan brought back inside the rules above. */
export function keepInside(frame: CameraFrame, zoom: number, pan: ScreenPoint): FilmPlacement {
  const least = fitZoom(frame);
  const wanted = Math.min(Math.max(zoom, least), least * MAX_ZOOM_OVER_FIT);
  if (wanted <= least * (1 + FIT_TOLERANCE)) return fit(frame);
  const upright = uprightSize(frame.size, frame.view.quarterTurns);
  const { viewport, margins } = frame;
  return place(frame, wanted, [
    keepAxis(pan[0], upright.width * wanted, viewport.width, margins.left, margins.right),
    keepAxis(pan[1], upright.height * wanted, viewport.height, margins.top, margins.bottom),
  ]);
}

/** Zoom by `factor`, keeping the film point under `at` (screen) where it is. */
export function zoomAt(frame: CameraFrame, from: FilmPlacement, factor: number, at: ScreenPoint): FilmPlacement {
  const least = fitZoom(frame);
  const zoom = Math.min(Math.max(from.zoom * factor, least), least * MAX_ZOOM_OVER_FIT);
  // The point under `at`, measured on the upright film from its outer corner.
  const q = [(at[0] - from.pan[0]) / from.zoom, (at[1] - from.pan[1]) / from.zoom] as const;
  return keepInside(frame, zoom, [at[0] - zoom * q[0], at[1] - zoom * q[1]]);
}

/** Move the film by `by` screen pixels. */
export function panBy(frame: CameraFrame, from: FilmPlacement, by: ScreenPoint): FilmPlacement {
  return keepInside(frame, from.zoom, [from.pan[0] + by[0], from.pan[1] + by[1]]);
}

/** The spot on screen a travelled-to point lands on: across the middle, LOOK_HEIGHT down. */
export function lookSpot(viewport: Viewport): ScreenPoint {
  return [viewport.width / 2, viewport.height * LOOK_HEIGHT];
}

/** The film point under the look spot. */
export function lookPoint(frame: CameraFrame, placement: FilmPlacement): Px {
  return filmSpace(placement).toFilm(lookSpot(frame.viewport));
}

/** Zoom to `over` times the fit with the film point `point` on the look spot. */
export function travelTo(frame: CameraFrame, point: Px, over: number): FilmPlacement {
  const zoom = fitZoom(frame) * Math.min(Math.max(over, 1), MAX_ZOOM_OVER_FIT);
  // Where the point would be with the film's outer corner at the screen's; the pan moves it onto the spot.
  const unmoved = filmSpace(place(frame, zoom, [0, 0])).toScreen(point);
  const spot = lookSpot(frame.viewport);
  return keepInside(frame, zoom, [spot[0] - unmoved[0], spot[1] - unmoved[1]]);
}

function place(frame: CameraFrame, zoom: number, pan: ScreenPoint): FilmPlacement {
  return { size: frame.size, view: frame.view, zoom, pan };
}

/** Where a film `extent` long sits along an axis it fits in: centred, then kept inside the margins. */
function centred(extent: number, length: number, before: number, after: number): number {
  return Math.min(Math.max((length - extent) / 2, before), length - after - extent);
}

function keepAxis(pan: number, extent: number, length: number, before: number, after: number): number {
  const room = length - before - after;
  if (extent <= room) return centred(extent, length, before, after);
  return Math.min(Math.max(pan, length - after - extent), before);
}
