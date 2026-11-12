// Where the spine is looked at from, as numbers anyone can set. Nothing in
// the 3D view moves the camera by itself over time: a view is a value, and
// whoever animates it sets a new one each frame.

import type { Vec3 } from '../placing/vectors';

export interface View {
  /** Degrees round the spine's upright: 0 from in front of the patient, +90 from the patient's left, -90 from the right. */
  azimuth: number;
  /** Degrees above the horizon: 0 level, 90 from straight above. */
  elevation: number;
  /** 0 is no perspective at all, as a film sees; 1 is the narrow lens the spine is turned in. */
  perspective: number;
  /** Screen pixels (CSS) for each millimetre at the aim: across the picture, and up it. */
  pxPerMm: readonly [number, number];
  /** The patient point the view is aimed at, mm. */
  aim: Vec3;
  /** Where that point is on the canvas, in CSS pixels from its top-left corner. */
  aimAt: readonly [number, number];
}

/** The canvas, in CSS pixels. */
export interface CanvasSize {
  width: number;
  height: number;
}

/** The unit directions of a view, in scene axes (X the patient's left, Y up, Z forward). */
export interface ViewAxes {
  /** Toward the right of the picture. */
  right: Vec3;
  /** Toward the top of the picture. */
  up: Vec3;
  /** Out of the picture, toward the one looking. */
  toViewer: Vec3;
}

const RADIANS = Math.PI / 180;

/**
 * The directions of a view from its two angles. Written out, not found by
 * "look at", so looking from straight above is as well defined as any
 * other view: the patient's front is then toward the bottom of the picture.
 */
export function viewAxes(azimuth: number, elevation: number): ViewAxes {
  const [sinA, cosA] = [Math.sin(azimuth * RADIANS), Math.cos(azimuth * RADIANS)];
  const [sinE, cosE] = [Math.sin(elevation * RADIANS), Math.cos(elevation * RADIANS)];
  return {
    right: [cosA, 0, -sinA],
    up: [-sinA * sinE, cosE, -cosA * sinE],
    toViewer: [sinA * cosE, sinE, cosA * cosE],
  };
}
