// How large the spine is drawn so that it fits the canvas, and the whole
// view for a named view or for a film.

import type { Vec3 } from '../placing/vectors';
import type { FilmCamera } from '../placing/film-camera';
import { toScene } from '../placing/scene-axes';
import { NAMED_VIEWS, type NamedView, type ViewName, azimuthAlong } from './named-views';
import type { CanvasSize, View } from './view';

/** A box in the scene: its lowest and highest corner. */
export interface Bounds {
  low: Vec3;
  high: Vec3;
}

/** The spine's height takes this share of the canvas's height at most, and its footprint this share of the width. */
const HEIGHT_ROOM = 1.24;
const WIDTH_ROOM = 1.4;

export const centreOf = (bounds: Bounds): Vec3 => [
  (bounds.low[0] + bounds.high[0]) / 2,
  (bounds.low[1] + bounds.high[1]) / 2,
  (bounds.low[2] + bounds.high[2]) / 2,
];

/** Screen pixels per millimetre at which the box fits the canvas from any side. */
export function fittedScale(bounds: Bounds, canvas: CanvasSize): number {
  const tall = bounds.high[1] - bounds.low[1];
  const wide = Math.max(bounds.high[0] - bounds.low[0], bounds.high[2] - bounds.low[2]);
  return Math.min(canvas.height / (HEIGHT_ROOM * tall), canvas.width / (WIDTH_ROOM * wide));
}

/** A view of the box from two angles, aimed at its middle, in the middle of the canvas. */
export function viewOf(bounds: Bounds, canvas: CanvasSize, from: NamedView): View {
  const scale = fittedScale(bounds, canvas) * from.zoom;
  return {
    azimuth: from.azimuth,
    elevation: from.elevation,
    perspective: from.perspective,
    pxPerMm: [scale, scale],
    aim: centreOf(bounds),
    aimAt: [canvas.width / 2, canvas.height / 2],
  };
}

/** One of the named views of the box. */
export function namedView(bounds: Bounds, canvas: CanvasSize, name: ViewName): View {
  return viewOf(bounds, canvas, NAMED_VIEWS[name]);
}

/** Where a film is shown on the page, for a spine to be drawn at its scale and height. */
export interface FilmOnPage {
  /** CSS pixels on screen for each upright film pixel. */
  pxPerFilmPx: number;
  /**
   * Where the film's upright pixel (0, 0) is drawn, in CSS pixels from the
   * canvas's top-left corner. Give the film's own place to draw the spine
   * over it; shift it sideways to draw the spine beside it at the same
   * height.
   */
  firstPixelAt: readonly [number, number];
}

/**
 * The view that looks along a film's beam with no perspective and draws a
 * millimetre exactly as large as the film on the page does, with the
 * patient origin where the film shows it. A bone then lies on its outline
 * pixel for pixel. It comes from the film's own geometry and nothing else.
 */
export function viewAlongFilm(camera: FilmCamera, shown: FilmOnPage): View {
  const [su, sv] = camera.uprightMmPerPx;
  return {
    azimuth: azimuthAlong(camera.imageRight),
    elevation: 0,
    perspective: 0,
    pxPerMm: [shown.pxPerFilmPx / su, shown.pxPerFilmPx / sv],
    aim: toScene([0, 0, 0]),
    aimAt: [shown.firstPixelAt[0] + camera.origin[0] * shown.pxPerFilmPx, shown.firstPixelAt[1] + camera.origin[1] * shown.pxPerFilmPx],
  };
}
