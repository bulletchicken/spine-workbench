// A bone shaped to a fitted vertebra, so that its body lands on the fit's.
//
// Each bone is one adult's vertebra with that body's own height, sizes and
// wedges. A fitted vertebra has others. Shaping moves every point of the
// bone so that afterwards its two endplates are the fitted ones: the same
// centres, the same four half-sizes and both wedges, with the bone's own
// wedges taken out. What comes back is in the fitted vertebra's own frame;
// `placingOf` then stands it in the patient.
//
// How a point moves:
//
// 1. The body. Each endplate has a frame: its centre, its two directions in
//    the plate and its normal. A point is written in the bone's top frame
//    and read back in the fitted top frame, stretched across by the ratio
//    of half-widths, front to back by the ratio of half-depths and along
//    the normal by the ratio of heights. The same is done with the bottom
//    frames. On the bone's top plate the first answer is exactly the fitted
//    top plate, and on its bottom plate the second is exactly the fitted
//    bottom plate. Between the plates the two answers are blended by how
//    far up the body the point is, with a weight that eases in and out, so
//    the walls stay smooth; above the top plate the top answer is used
//    alone, and below the bottom plate the bottom one.
// 2. The arch. Behind the back wall of the body nothing is stretched with
//    the body's depth or height: a stretched spinous process is a different
//    bone. The arch is scaled evenly, by the ratio of widths, about the
//    place the pedicles leave the body, so it keeps its proportions.
// 3. The join. Over the length of the pedicles the body's rule eases into
//    the arch's. Going back from the wall, each step is first as long as
//    the body's rule makes it and then as long as the arch's, with no
//    sudden change and never shorter than either, so nothing tears and
//    nothing is squeezed.
//
// The shape drawn is a generic bone shaped to the fit, never the patient's
// own.

import type { Bone, BoneBody } from '../bones/read-bones';
import { type BodyFrame, bodyFrameOf } from './bone-body';
import { type BodyNumbers, type Endplate, endplatesOf } from './endplates';
import { type Vec3, along, dot, mix, scale, smoothStep, sub } from './vectors';

/** A point of the bone's frame, moved to where it belongs in the fitted vertebra's own frame. */
export type Shaper = (point: Vec3) => Vec3;

/** The body's rule eases into the arch's over this share of the body's mean half-depth, behind its back wall. */
export const JOIN_SHARE = 0.75;

interface PlatePair {
  from: Endplate;
  to: Endplate;
  across: number;
  frontToBack: number;
}

/**
 * Outside the rim a plate's tilt stops reaching: past this many half-sizes
 * from the plate's centre, what is beside the body is carried level.
 */
const TILT_REACHES = 1.5;

/** A point written in the bone's plate frame and read back in the fitted one. */
function onPlates(pair: PlatePair, upward: number, from: Vec3): Vec3 {
  const moved = along(pair.to.centre, pair.to.widthAxis, dot(from, pair.from.widthAxis) * pair.across);
  const deep = along(moved, pair.to.depthAxis, dot(from, pair.from.depthAxis) * pair.frontToBack);
  return along(deep, pair.to.normal, dot(from, pair.from.normal) * upward);
}

/**
 * Where one plate's rule carries a point. Within the rim that is the
 * plate-to-plate rule exactly. A plate is tilted by its wedge, and a tilt
 * grows without end going outwards, which would squeeze or stretch a
 * process that stands beside the body. So outside the rim the tilt is let
 * go smoothly: the point is carried as if it stood at most `TILT_REACHES`
 * half-sizes out, and the rest of the way it is only stretched, level.
 */
function carried(pair: PlatePair, upward: number, point: Vec3): Vec3 {
  const from = sub(point, pair.from.centre);
  const out = Math.hypot(from[0] / pair.from.halfDepth, from[1] / pair.from.halfWidth);
  if (out <= 1) return onPlates(pair, upward, from);
  const room = TILT_REACHES - 1;
  const kept = (1 + room * (1 - Math.exp(-(out - 1) / room))) / out;
  const within = onPlates(pair, upward, [from[0] * kept, from[1] * kept, from[2]]);
  return [within[0] + pair.frontToBack * from[0] * (1 - kept), within[1] + pair.across * from[1] * (1 - kept), within[2]];
}

/** Where a bone's arch joins its body, in the body's frame. */
export interface ArchJoin {
  /** The back wall of the body: the rearmost end of either rim, as an x. */
  wall: number;
  /** How far behind the wall the body's rule has given way to the arch's, mm. */
  length: number;
  /** The height at which the pedicles leave the body. */
  height: number;
}

/** The back wall and the length of the join, from the body's own landmarks. */
function wallOf(frame: BodyFrame): { wall: number; length: number } {
  const { top, bottom } = frame.endplates;
  const backOfTop = top.centre[0] + top.depthAxis[0] * top.halfDepth;
  const backOfBottom = bottom.centre[0] + bottom.depthAxis[0] * bottom.halfDepth;
  return { wall: Math.min(backOfTop, backOfBottom), length: JOIN_SHARE * 0.5 * (top.halfDepth + bottom.halfDepth) };
}

/**
 * Where a bone's arch joins its body. The height is measured on the bone:
 * just behind the back wall the only bone there is the two pedicles, so the
 * mean height of the points in that slice is theirs.
 */
export function archJoinOf(bone: Bone): ArchJoin {
  const frame = bodyFrameOf(bone.body);
  const { wall, length } = wallOf(frame);
  const [ox, oy, oz] = frame.origin;
  const axes = frame.axes;
  let sum = 0;
  let count = 0;
  for (let i = 0; i + 2 < bone.points.length; i += 3) {
    const [dx, dy, dz] = [(bone.points[i] ?? 0) - ox, (bone.points[i + 1] ?? 0) - oy, (bone.points[i + 2] ?? 0) - oz];
    const x = axes[0][0] * dx + axes[1][0] * dy + axes[2][0] * dz;
    if (x >= wall || x < wall - length) continue;
    sum += axes[0][2] * dx + axes[1][2] * dy + axes[2][2] * dz;
    count += 1;
  }
  return { wall, length, height: count === 0 ? 0 : sum / count };
}

/**
 * The rule that shapes a bone with this body to a vertebra with these
 * numbers. `joinHeight` is the height at which the bone's pedicles leave
 * its body (`archJoinOf`); the endplates land on the fitted ones whatever
 * it is.
 */
export function shaperFor(body: BoneBody, fitted: BodyNumbers, joinHeight = 0): Shaper {
  const frame: BodyFrame = bodyFrameOf(body);
  const own = frame.endplates;
  const goal = endplatesOf(fitted);
  const top: PlatePair = {
    from: own.top,
    to: goal.top,
    across: goal.top.halfWidth / own.top.halfWidth,
    frontToBack: goal.top.halfDepth / own.top.halfDepth,
  };
  const bottom: PlatePair = {
    from: own.bottom,
    to: goal.bottom,
    across: goal.bottom.halfWidth / own.bottom.halfWidth,
    frontToBack: goal.bottom.halfDepth / own.bottom.halfDepth,
  };
  const upward = fitted.height / frame.numbers.height;

  const inBody = (point: Vec3): Vec3 => {
    const aboveBottom = dot(sub(point, own.bottom.centre), own.bottom.normal);
    const aboveTop = dot(sub(point, own.top.centre), own.top.normal);
    // 0 on the bottom plate, 1 on the top plate.
    const weight = smoothStep(aboveBottom / (aboveBottom - aboveTop));
    if (weight === 0) return carried(bottom, upward, point);
    if (weight === 1) return carried(top, upward, point);
    return mix(carried(bottom, upward, point), carried(top, upward, point), weight);
  };

  /** How the body's rule carries a step forward, at a point of the back wall. */
  const forwardAt = (onWall: Vec3): Vec3 => {
    const step = 1e-3;
    return scale(sub(inBody([onWall[0] + step, onWall[1], onWall[2]]), inBody([onWall[0] - step, onWall[1], onWall[2]])), 1 / (2 * step));
  };

  const { wall, length: join } = wallOf(frame);
  const anchor: Vec3 = [wall, 0, joinHeight];
  const anchorMoved = inBody(anchor);
  const forwardAtAnchor = forwardAt(anchor);
  const evenly = (goal.top.halfWidth + goal.bottom.halfWidth) / (own.top.halfWidth + own.bottom.halfWidth);

  const axes = frame.axes;
  const origin = frame.origin;
  return (point) => {
    // Into the body's frame: the transpose of its axes, applied to the offset from its origin.
    const offset = sub(point, origin);
    const local: Vec3 = [
      axes[0][0] * offset[0] + axes[1][0] * offset[1] + axes[2][0] * offset[2],
      axes[0][1] * offset[0] + axes[1][1] * offset[1] + axes[2][1] * offset[2],
      axes[0][2] * offset[0] + axes[1][2] * offset[1] + axes[2][2] * offset[2],
    ];
    const behind = wall - local[0];
    if (behind <= 0) return inBody(local);

    // The point of the wall straight in front of this one, by both rules.
    const onWall: Vec3 = [wall, local[1], local[2]];
    const share = behind / join;
    const eased = smoothStep(share);
    const byBody = inBody(onWall);
    const byArch: Vec3 = [anchorMoved[0], anchorMoved[1] + evenly * local[1], anchorMoved[2] + evenly * (local[2] - joinHeight)];
    const start = mix(byBody, byArch, eased);

    // Then back from the wall. Of the distance `behind`, `withArch` is stepped at the arch's even
    // scale and the rest as the body's rule steps: the area under the easing curve, so the step
    // length changes smoothly from one to the other.
    const withArch = share >= 1 ? behind - 0.5 * join : join * share * share * share * (1 - 0.5 * share);
    const forward = eased === 1 ? forwardAtAnchor : mix(forwardAt(onWall), forwardAtAnchor, eased);
    return [
      start[0] - forward[0] * (behind - withArch) - evenly * withArch,
      start[1] - forward[1] * (behind - withArch),
      start[2] - forward[2] * (behind - withArch),
    ];
  };
}

/** Every point of `bone`, shaped to the fitted vertebra: x, y, z for each, in the vertebra's own frame. */
export function shapeBone(bone: Bone, fitted: BodyNumbers): Float32Array {
  const shape = shaperFor(bone.body, fitted, archJoinOf(bone).height);
  const points = bone.points;
  const shaped = new Float32Array(points.length);
  for (let i = 0; i + 2 < points.length; i += 3) {
    const [x, y, z] = shape([points[i] ?? 0, points[i + 1] ?? 0, points[i + 2] ?? 0]);
    shaped[i] = x;
    shaped[i + 1] = y;
    shaped[i + 2] = z;
  }
  return shaped;
}
