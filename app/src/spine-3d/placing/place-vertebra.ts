// Where a fitted vertebra stands: `patient = centre + R × own`, with R made
// from the fit's rotation vector exactly as the engine makes it. A shaped
// bone is in the vertebra's own frame, so this is the last step of placing
// one.

import type { VertebraPose } from '@/shapes';
import { matrixFromRotationVector } from './rotation';
import { type Mat3, type Vec3, add, apply } from './vectors';

export interface Placing {
  centre: Vec3;
  /** The vertebra's own x (forward), y (left) and z (up) as columns, in patient axes. */
  rotation: Mat3;
  /** A point of the vertebra's own frame, in the patient frame. */
  toPatient(own: Vec3): Vec3;
}

export function placingOf(pose: Pick<VertebraPose, 'centre' | 'rotation'>): Placing {
  const rotation = matrixFromRotationVector(pose.rotation);
  const centre: Vec3 = [pose.centre[0], pose.centre[1], pose.centre[2]];
  return { centre, rotation, toPatient: (own) => add(centre, apply(rotation, own)) };
}
