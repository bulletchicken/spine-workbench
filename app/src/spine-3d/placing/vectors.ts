// Three numbers as a point or a direction, and nine as a rotation or a
// stretch. Just enough arithmetic for placing bones: plain arrays, no
// library, so the placing code can be tested without a GPU.

export type Vec3 = readonly [number, number, number];

/** Three rows of three. `M v` is each row dotted with `v`. */
export type Mat3 = readonly [Vec3, Vec3, Vec3];

export const IDENTITY: Mat3 = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
];

export const add = (a: Vec3, b: Vec3): Vec3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
export const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
export const scale = (a: Vec3, by: number): Vec3 => [a[0] * by, a[1] * by, a[2] * by];
export const dot = (a: Vec3, b: Vec3): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
export const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
export const length = (a: Vec3): number => Math.hypot(a[0], a[1], a[2]);
export const unit = (a: Vec3): Vec3 => scale(a, 1 / length(a));

/** `a + b * by`: a point moved along a direction. */
export const along = (a: Vec3, b: Vec3, by: number): Vec3 => [a[0] + b[0] * by, a[1] + b[1] * by, a[2] + b[2] * by];

/** The point `share` of the way from `a` to `b`. */
export const mix = (a: Vec3, b: Vec3, share: number): Vec3 => [
  a[0] + (b[0] - a[0]) * share,
  a[1] + (b[1] - a[1]) * share,
  a[2] + (b[2] - a[2]) * share,
];

export const apply = (m: Mat3, v: Vec3): Vec3 => [dot(m[0], v), dot(m[1], v), dot(m[2], v)];

export const transpose = (m: Mat3): Mat3 => [
  [m[0][0], m[1][0], m[2][0]],
  [m[0][1], m[1][1], m[2][1]],
  [m[0][2], m[1][2], m[2][2]],
];

/** `a b`: the matrix that does `b` first, then `a`. */
export function times(a: Mat3, b: Mat3): Mat3 {
  const columns = transpose(b);
  return [
    [dot(a[0], columns[0]), dot(a[0], columns[1]), dot(a[0], columns[2])],
    [dot(a[1], columns[0]), dot(a[1], columns[1]), dot(a[1], columns[2])],
    [dot(a[2], columns[0]), dot(a[2], columns[1]), dot(a[2], columns[2])],
  ];
}

export const determinant = (m: Mat3): number => dot(m[0], cross(m[1], m[2]));

/** The matrix whose columns are the three directions given. */
export const fromColumns = (x: Vec3, y: Vec3, z: Vec3): Mat3 => transpose([x, y, z]);

/** A value eased from 0 to 1 with no jump in slope at either end; outside 0 to 1 it stays at the end. */
export function smoothStep(share: number): number {
  const t = Math.min(1, Math.max(0, share));
  return t * t * (3 - 2 * t);
}
