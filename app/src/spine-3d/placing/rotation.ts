// A rotation vector as a matrix, made exactly as the engine makes it
// (engine/forward-model/src/rotation.rs), so a bone is turned by the same
// rotation the fit found.

import { type Mat3, type Vec3, length } from './vectors';

/**
 * The rotation matrix of a rotation vector (the rotation's axis times its
 * angle in radians), by Rodrigues' formula:
 * `R = I + (sin θ / θ) K + ((1 − cos θ) / θ²) K²`, with `θ = |r|` and `K`
 * the cross-product matrix of `r`. The second factor is written as
 * `½ (sin(θ/2) / (θ/2))²`, which keeps its precision as θ shrinks.
 */
export function matrixFromRotationVector(r: Vec3): Mat3 {
  const theta = length(r);
  let first = 1;
  let second = 0.5;
  if (theta > 0) {
    const half = 0.5 * theta;
    const sincHalf = Math.sin(half) / half;
    first = Math.sin(theta) / theta;
    second = 0.5 * sincHalf * sincHalf;
  }
  const [x, y, z] = r;
  // K = [[0, −z, y], [z, 0, −x], [−y, x, 0]], and K² = r rᵀ − θ² I.
  const square = theta * theta;
  return [
    [1 + second * (x * x - square), -first * z + second * x * y, first * y + second * x * z],
    [first * z + second * y * x, 1 + second * (y * y - square), -first * x + second * y * z],
    [-first * y + second * z * x, first * x + second * z * y, 1 + second * (z * z - square)],
  ];
}
