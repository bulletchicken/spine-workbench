// How a point in the patient lands on a film: the engine's parallel camera
// (engine/forward-model/src/cameras/parallel.rs), in TypeScript, and its
// way back from a film pixel to the patient. The view uses it to stand a
// film in the same space as the bones from the film's own geometry: its
// scale, its facing, its quarter turns and its size. Nothing here is ever
// fitted to the bones.
//
//   u = u0 + across / su        across = p · screenRight(imageRight)
//   v = v0 − z / sv
//
// on the film shown upright, with (u0, v0) the upright pixel where the
// patient origin lands and su, sv the upright pixel's width and height in
// millimetres. The upright pixel is then turned back to the film as stored.

import type { FilmGeometry, ImageRight, QuarterTurns } from '@/shapes';
import { type Vec3, along, cross, dot } from './vectors';

/** A film pixel, or an upright one: x right, y down. */
export type Px = readonly [number, number];

export interface FilmCamera {
  /** The patient direction that points to screen right once the film is upright. */
  imageRight: ImageRight;
  /** Clockwise quarter turns that show the film upright. */
  quarterTurns: QuarterTurns;
  /** The film's width and height in pixels, as stored. */
  size: readonly [number, number];
  /** The upright pixel's width and height, mm. */
  uprightMmPerPx: readonly [number, number];
  /** The upright pixel where the patient origin lands. */
  origin: Px;
}

const UP: Vec3 = [0, 0, 1];

/** The patient direction that points to screen right on a film shown upright. */
export function screenRight(imageRight: ImageRight): Vec3 {
  switch (imageRight) {
    case 'patient-left':
      return [0, 1, 0];
    case 'patient-right':
      return [0, -1, 0];
    case 'anterior':
      return [1, 0, 0];
    case 'posterior':
      return [-1, 0, 0];
  }
}

/** The patient direction from a film toward whoever reads it: out of the picture. */
export function towardReader(imageRight: ImageRight): Vec3 {
  return cross(screenRight(imageRight), UP);
}

/**
 * The camera of a film from its saved geometry, its stored size and the
 * upright pixel where the patient origin lands. A film that carries no
 * scale is drawn at the scale the fit worked at, given here as millimetres
 * per stored pixel; with neither, or with no facing, there is no camera.
 */
export function filmCamera(
  geometry: Pick<FilmGeometry, 'mmPerPx' | 'mmPerPxVertical' | 'quarterTurns' | 'imageRight'>,
  size: readonly [number, number],
  origin: Px,
  mmPerPxOfTheFit: number | null = null,
): FilmCamera | null {
  const alongRow = geometry.mmPerPx ?? mmPerPxOfTheFit;
  if (alongRow === null || geometry.imageRight === null) return null;
  const downColumn = geometry.mmPerPx === null ? alongRow : (geometry.mmPerPxVertical ?? alongRow);
  const usable = (spacing: number) => Number.isFinite(spacing) && spacing > 0;
  if (!usable(alongRow) || !usable(downColumn)) return null;
  return {
    imageRight: geometry.imageRight,
    quarterTurns: geometry.quarterTurns,
    size,
    uprightMmPerPx: geometry.quarterTurns % 2 === 0 ? [alongRow, downColumn] : [downColumn, alongRow],
    origin,
  };
}

/** An upright pixel as a pixel of the film as stored. */
export function uprightToFilm(camera: FilmCamera, [u, v]: Px): Px {
  const w = camera.size[0] - 1;
  const h = camera.size[1] - 1;
  switch (camera.quarterTurns) {
    case 0:
      return [u, v];
    case 1:
      return [v, h - u];
    case 2:
      return [w - u, h - v];
    case 3:
      return [w - v, u];
  }
}

/** A pixel of the film as stored, as an upright pixel. */
export function filmToUpright(camera: FilmCamera, [x, y]: Px): Px {
  const w = camera.size[0] - 1;
  const h = camera.size[1] - 1;
  switch (camera.quarterTurns) {
    case 0:
      return [x, y];
    case 1:
      return [h - y, x];
    case 2:
      return [w - x, h - y];
    case 3:
      return [y, w - x];
  }
}

/** Where a patient point lands on the film shown upright. */
export function toUpright(camera: FilmCamera, point: Vec3): Px {
  const [su, sv] = camera.uprightMmPerPx;
  return [camera.origin[0] + dot(point, screenRight(camera.imageRight)) / su, camera.origin[1] - point[2] / sv];
}

/** Where a patient point lands on the film, in pixels of the film as stored. */
export function toFilm(camera: FilmCamera, point: Vec3): Px {
  return uprightToFilm(camera, toUpright(camera, point));
}

/**
 * The patient point that lands on an upright pixel and lies `out`
 * millimetres from the patient origin toward the film's reader. Every
 * point along that line lands on the same pixel.
 */
export function fromUpright(camera: FilmCamera, [u, v]: Px, out: number): Vec3 {
  const [su, sv] = camera.uprightMmPerPx;
  const across = (u - camera.origin[0]) * su;
  const up = (camera.origin[1] - v) * sv;
  return along(along([0, 0, up], screenRight(camera.imageRight), across), towardReader(camera.imageRight), out);
}

/** The same, from a pixel of the film as stored. */
export function fromFilm(camera: FilmCamera, pixel: Px, out: number): Vec3 {
  return fromUpright(camera, filmToUpright(camera, pixel), out);
}
