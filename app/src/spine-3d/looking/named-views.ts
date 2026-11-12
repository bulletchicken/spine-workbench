// The views with names, and the rule for how much perspective a view has.
//
// Front, side and top look along an axis, as a film does, so they have no
// perspective: a bone then sits on its outline. The three-quarter view the
// spine opens in is turned off every axis and uses the narrow lens.

import type { ImageRight } from '@/shapes';
import { smoothStep } from '../placing/vectors';
import { viewAxes } from './view';

export type ViewName = 'front' | 'side' | 'top' | 'three-quarter';

export interface NamedView {
  azimuth: number;
  elevation: number;
  /** 1 fits the spine to the canvas. */
  zoom: number;
  perspective: number;
}

export const NAMED_VIEWS: Record<ViewName, NamedView> = {
  // As on a front film named for the patient's left: the patient's left on the right of the picture.
  front: { azimuth: 0, elevation: 0, zoom: 1, perspective: 0 },
  // As on a side film named for anterior: the front of the body on the right of the picture.
  side: { azimuth: -90, elevation: 0, zoom: 1, perspective: 0 },
  // From above, the front of the body toward the bottom of the picture.
  top: { azimuth: 0, elevation: 90, zoom: 1.7, perspective: 0 },
  // From the front and the patient's left, a little above.
  'three-quarter': { azimuth: 34, elevation: 14, zoom: 1.02, perspective: 1 },
};

/** The azimuth from which a film with this facing is read: looking along its beam. */
export function azimuthAlong(imageRight: ImageRight): number {
  switch (imageRight) {
    case 'patient-left':
      return 0;
    case 'patient-right':
      return 180;
    case 'anterior':
      return -90;
    case 'posterior':
      return 90;
  }
}

/** A view turned this many degrees off every axis has the whole lens. */
export const LENS_EASES_OVER = 24;

/**
 * How much perspective a view turned by hand has: none looking along an
 * axis (from the front, the back, either side or above), easing to the
 * whole lens as it turns away. It depends only on where the view is, so
 * turning back to an axis flattens the picture again with no jump.
 */
export function perspectiveFor(azimuth: number, elevation: number): number {
  const { toViewer } = viewAxes(azimuth, elevation);
  // The axis nearest the view: the largest of |X|, |Z| and Y of the direction to the viewer.
  const nearest = Math.max(Math.abs(toViewer[0]), Math.abs(toViewer[2]), toViewer[1]);
  const off = (Math.acos(Math.min(1, nearest)) * 180) / Math.PI;
  return smoothStep(off / LENS_EASES_OVER);
}

/** The whole number of turns nearest `to` that reaches it from `from` the short way round. */
export function shortWayRound(from: number, to: number): number {
  return from + ((((to - from) % 360) + 540) % 360) - 180;
}
