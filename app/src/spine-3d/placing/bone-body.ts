// A bone's own vertebral body, read as the vertebra model reads one.
//
// bone-prep measured each bone's two endplate rims in the bone's frame.
// The model wants a body in a frame of its own: z along the line between
// the two endplate centres, the origin half way. On a bone that line is
// within a millimetre of the bone's z axis but not on it, so the body's
// frame is the bone's turned the least that puts z on that line.

import type { BoneBody } from '../bones/read-bones';
import { type BodyNumbers, type Endplates, endplatesOf } from './endplates';
import { IDENTITY, type Mat3, type Vec3, add, apply, cross, length, mix, scale, sub, times, transpose } from './vectors';

export interface BodyFrame {
  /** The body's origin in the bone's frame: half way between its endplate centres. */
  origin: Vec3;
  /** The body's x, y and z axes as columns, in the bone's frame. */
  axes: Mat3;
  /** The body's own numbers: its height between the endplate centres, its wedges and half-sizes. */
  numbers: BodyNumbers;
  /** Its two endplates in its own frame. */
  endplates: Endplates;
}

/** The rotation that takes ẑ to the unit vector `up` by the shortest way. */
function leastTurnOntoUp(up: Vec3): Mat3 {
  const v = cross([0, 0, 1], up);
  // R = I + K + K² / (1 + ẑ · up), with K the cross-product matrix of v.
  const k: Mat3 = [
    [0, -v[2], v[1]],
    [v[2], 0, -v[0]],
    [-v[1], v[0], 0],
  ];
  const squared = times(k, k);
  const share = 1 / (1 + up[2]);
  const row = (i: 0 | 1 | 2): Vec3 => [
    IDENTITY[i][0] + k[i][0] + share * squared[i][0],
    IDENTITY[i][1] + k[i][1] + share * squared[i][1],
    IDENTITY[i][2] + k[i][2] + share * squared[i][2],
  ];
  return [row(0), row(1), row(2)];
}

/** The frame of a bone's body and its endplates in that frame. */
export function bodyFrameOf(body: BoneBody): BodyFrame {
  const between = sub(body.top.centre, body.bottom.centre);
  const height = length(between);
  const numbers: BodyNumbers = {
    height,
    wedgeFront: body.wedgeFront,
    wedgeSide: body.wedgeSide,
    topHalfWidth: body.top.halfWidth,
    topHalfDepth: body.top.halfDepth,
    bottomHalfWidth: body.bottom.halfWidth,
    bottomHalfDepth: body.bottom.halfDepth,
  };
  return {
    origin: mix(body.bottom.centre, body.top.centre, 0.5),
    axes: leastTurnOntoUp(scale(between, 1 / height)),
    numbers,
    endplates: endplatesOf(numbers),
  };
}

/** A point of the bone, in its body's frame. */
export function intoBody(frame: BodyFrame, point: Vec3): Vec3 {
  return apply(transpose(frame.axes), sub(point, frame.origin));
}

/** A point of the body's frame, back in the bone's. */
export function outOfBody(frame: BodyFrame, point: Vec3): Vec3 {
  return add(frame.origin, apply(frame.axes, point));
}
