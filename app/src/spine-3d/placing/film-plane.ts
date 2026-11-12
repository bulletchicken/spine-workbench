// Where a film stands in the patient: a flat sheet across its beam, with
// every pixel straight behind (or in front of) the anatomy that lands on
// it. Its place across and up comes from the film's camera; how far along
// the beam it stands is the caller's choice, since a parallel beam does not
// say.

import { type FilmCamera, fromFilm, towardReader } from './film-camera';
import type { Vec3 } from './vectors';

export interface FilmPlane {
  /** The unit direction out of the film toward its reader. */
  towardReader: Vec3;
  /** How far the sheet is from the patient origin along that direction, mm (negative: behind the patient, as its reader sees it). */
  out: number;
  /**
   * The outer corners of the film as stored, in the patient frame, in the
   * order of the stored picture: top left, top right, bottom right,
   * bottom left. A turned film's "top left" is wherever its first pixel
   * ends up.
   */
  corners: readonly [Vec3, Vec3, Vec3, Vec3];
}

/** The film's sheet, standing `out` millimetres from the patient origin toward its reader. */
export function filmPlane(camera: FilmCamera, out: number): FilmPlane {
  const [width, height] = camera.size;
  // A pixel's centre is a whole number, so the film's outer edge is half a pixel outside its first and last.
  return {
    towardReader: towardReader(camera.imageRight),
    out,
    corners: [
      fromFilm(camera, [-0.5, -0.5], out),
      fromFilm(camera, [width - 0.5, -0.5], out),
      fromFilm(camera, [width - 0.5, height - 0.5], out),
      fromFilm(camera, [-0.5, height - 0.5], out),
    ],
  };
}
