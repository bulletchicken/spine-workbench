// The straight line behind a place on the canvas: everything in the scene
// that is drawn there lies on it. Used to find the bone under the pointer.

import { type Vec3, along, sub, unit } from '../placing/vectors';
import { eyeDistance } from './lens';
import { type CanvasSize, type View, viewAxes } from './view';

export interface Ray {
  /** A point of the line nearer the viewer than anything drawn. */
  from: Vec3;
  /** The unit direction away from the viewer. */
  toward: Vec3;
}

/**
 * The line through a canvas place, in CSS pixels from the canvas's top-left
 * corner. `reach` is how far from the aim, in mm, anything drawn can be.
 */
export function rayThrough(view: View, canvas: CanvasSize, x: number, y: number, reach: number): Ray {
  const axes = viewAxes(view.azimuth, view.elevation);
  // The point of the plane through the aim that is drawn at (x, y).
  const across = (x - view.aimAt[0]) / view.pxPerMm[0];
  const up = (view.aimAt[1] - y) / view.pxPerMm[1];
  const onAimPlane = along(along(view.aim, axes.right, across), axes.up, up);
  const distance = eyeDistance(view, canvas);
  if (distance === null) return { from: along(onAimPlane, axes.toViewer, 2 * reach), toward: [-axes.toViewer[0], -axes.toViewer[1], -axes.toViewer[2]] };
  const eye = along(view.aim, axes.toViewer, distance);
  return { from: eye, toward: unit(sub(onAimPlane, eye)) };
}
