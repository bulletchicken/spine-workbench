// Film pixels and the screen, and what layers and HUDs drawn over a film
// receive. FROZEN: changes only through a contract-change pull request.
//
// Coordinates everywhere in the app are original film pixels: x right,
// y down, the centre of the top-left pixel at (0, 0). Films are never
// turned, flipped or resized. Showing a film upright, or mirrored, is a
// view transform, and this file is the only place that defines it, and the
// only place a half pixel is added for drawing:
//
//   film pixel ──turn and mirror──▶ upright pixel ──(+ ½) · zoom + pan──▶ screen
//
// Pixel (i, j) covers the square from (i − ½, j − ½) to (i + ½, j + ½), so
// the film's outer corner (−½, −½) lands exactly on `pan`.

import type { ComponentType } from 'react';
import type { Px, QuarterTurns } from '@/shapes';

/** A point on screen in CSS pixels, from the viewer's top-left corner. */
export type ScreenPoint = readonly [x: number, y: number];

export interface FilmSize {
  width: number;
  height: number;
}

/** How a film is shown. Neither changes a pixel. */
export interface FilmView {
  /** Clockwise quarter turns that show the film upright (the film's `geometry.quarterTurns`). */
  quarterTurns: QuarterTurns;
  /** Flipped left to right after turning: a viewing aid, never saved with the film. */
  mirrored: boolean;
}

/** Where a film sits on screen. */
export interface FilmPlacement {
  size: FilmSize;
  view: FilmView;
  /** Screen pixels per film pixel. */
  zoom: number;
  /** Where the upright film's outer top-left corner lands on screen. */
  pan: ScreenPoint;
}

/**
 * An affine map `[a, b, c, d, e, f]`: a point (x, y) goes to
 * (a·x + c·y + e, b·x + d·y + f). The same order as SVG's `matrix()`.
 */
export type Matrix = readonly [a: number, b: number, c: number, d: number, e: number, f: number];

/** The film's size once turned upright. */
export function uprightSize(size: FilmSize, quarterTurns: QuarterTurns): FilmSize {
  return quarterTurns % 2 === 0 ? size : { width: size.height, height: size.width };
}

/**
 * Film pixels → upright pixels: the pixel-centre grid turned and mirrored.
 * Pixel centres land on pixel centres.
 */
export function uprightMatrix(size: FilmSize, view: FilmView): Matrix {
  const w = size.width - 1;
  const h = size.height - 1;
  const turned: Matrix = (
    {
      0: [1, 0, 0, 1, 0, 0],
      1: [0, 1, -1, 0, h, 0],
      2: [-1, 0, 0, -1, w, h],
      3: [0, -1, 1, 0, 0, w],
    } as const
  )[view.quarterTurns];
  if (!view.mirrored) return turned;
  const across = uprightSize(size, view.quarterTurns).width - 1;
  const [a, b, c, d, e, f] = turned;
  return [-a, b, -c, d, across - e, f];
}

/** Film pixels → screen, half pixel included. */
export function screenMatrix(placement: FilmPlacement): Matrix {
  const [a, b, c, d, e, f] = uprightMatrix(placement.size, placement.view);
  const { zoom, pan } = placement;
  return [zoom * a, zoom * b, zoom * c, zoom * d, pan[0] + zoom * (e + 0.5), pan[1] + zoom * (f + 0.5)];
}

export function apply(m: Matrix, p: readonly [number, number]): [number, number] {
  return [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]];
}

export function invert(m: Matrix): Matrix {
  const [a, b, c, d, e, f] = m;
  const det = a * d - b * c;
  return [d / det, -b / det, -c / det, a / det, (c * f - d * e) / det, (b * e - a * f) / det];
}

/** A film as placed on screen: what every layer, HUD and tool converts with. */
export interface FilmSpace {
  readonly placement: FilmPlacement;
  /** Film pixels → screen; give it to an SVG group as `matrix(…)`. */
  readonly matrix: Matrix;
  toScreen(p: Px): ScreenPoint;
  toFilm(s: ScreenPoint): Px;
  /** A length on screen as film pixels: for strokes and handles that keep their screen size. */
  screenToFilmLength(screenPixels: number): number;
}

export function filmSpace(placement: FilmPlacement): FilmSpace {
  const matrix = screenMatrix(placement);
  const inverse = invert(matrix);
  return {
    placement,
    matrix,
    toScreen: (p) => apply(matrix, p),
    toFilm: (s) => apply(inverse, s),
    screenToFilmLength: (screenPixels) => screenPixels / placement.zoom,
  };
}

/** What a layer receives. */
export interface FilmLayerProps {
  space: FilmSpace;
}

/**
 * Something drawn on the film: outlines, marks, labels, landmarks, a
 * tool's preview, the answer key. It renders SVG inside a group already
 * set to `space.matrix`, so its coordinates are film pixels. A size meant
 * to look the same at every zoom uses `space.screenToFilmLength`.
 * Layers are drawn in the order given, first at the bottom.
 */
export interface FilmLayer {
  /** Unique among the layers on one viewer. */
  id: string;
  Draw: ComponentType<FilmLayerProps>;
}

/** What a HUD receives. */
export interface FilmHudProps {
  space: FilmSpace;
  /** The viewer's size in CSS pixels. */
  viewport: { width: number; height: number };
}

/**
 * Something drawn over the viewer in screen pixels and never transformed
 * with the film: the magnifier, the answer key's scores, a scale bar.
 */
export interface FilmHud {
  /** Unique among the HUDs on one viewer. */
  id: string;
  Draw: ComponentType<FilmHudProps>;
}
