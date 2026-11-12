// The two endplates of the vertebra model, as the engine builds them
// (engine/forward-model/src/endplates.rs): each takes half of each wedge,
// the top one way and the bottom the other, and its rim is an oval.
//
//   top normal     n_t = rotX(+ωf/2) · rotY(+ωs/2) · ẑ
//   bottom normal  n_b = rotX(−ωf/2) · rotY(−ωs/2) · ẑ
//   across         ê_w = normalise(ŷ − (ŷ · n) n)     toward the left
//   front to back  ê_d = n × ê_w                      toward the back
//
// Everything here is in the body's own frame: x forward, y to its left,
// z up along the line between its endplate centres, origin half way.

import { type Vec3, cross, scale } from './vectors';

/** The numbers that describe a vertebral body, whoever's they are: a fitted vertebra's or a bone's own. */
export interface BodyNumbers {
  /** The distance between the two endplate centres, mm. */
  height: number;
  /** Radians: positive when the patient's left side is taller. */
  wedgeFront: number;
  /** Radians: positive when the front is shorter. */
  wedgeSide: number;
  topHalfWidth: number;
  topHalfDepth: number;
  bottomHalfWidth: number;
  bottomHalfDepth: number;
}

/** One endplate: where its rim's centre is, which way it faces, and its oval. */
export interface Endplate {
  centre: Vec3;
  /** Unit normal, pointing up. */
  normal: Vec3;
  /** Unit direction in the plate, across it, toward the left. */
  widthAxis: Vec3;
  /** Unit direction in the plate, front to back: `normal × widthAxis`. */
  depthAxis: Vec3;
  halfWidth: number;
  halfDepth: number;
}

export interface Endplates {
  top: Endplate;
  bottom: Endplate;
}

function endplate(centre: Vec3, normal: Vec3, halfWidth: number, halfDepth: number): Endplate {
  const ny = normal[1];
  // |ŷ − n_y n|² = 1 − n_y² for a unit normal.
  const widthAxis = scale([-ny * normal[0], 1 - ny * normal[1], -ny * normal[2]], 1 / Math.sqrt(1 - ny * ny));
  return { centre, normal, widthAxis, depthAxis: cross(normal, widthAxis), halfWidth, halfDepth };
}

/** Both endplates of a body, in the body's own frame. */
export function endplatesOf(body: BodyNumbers): Endplates {
  const [sinF, cosF] = [Math.sin(0.5 * body.wedgeFront), Math.cos(0.5 * body.wedgeFront)];
  const [sinS, cosS] = [Math.sin(0.5 * body.wedgeSide), Math.cos(0.5 * body.wedgeSide)];
  // rotX(θ) · rotY(φ) · ẑ = (sin φ, −sin θ cos φ, cos θ cos φ); the bottom takes −θ and −φ.
  return {
    top: endplate([0, 0, 0.5 * body.height], [sinS, -sinF * cosS, cosF * cosS], body.topHalfWidth, body.topHalfDepth),
    bottom: endplate([0, 0, -0.5 * body.height], [-sinS, sinF * cosS, cosF * cosS], body.bottomHalfWidth, body.bottomHalfDepth),
  };
}
