import type { VertebraPose } from '@/shapes';

export const SHAPE_FIELDS = ['height', 'wedgeFront', 'wedgeSide', 'topHalfWidth', 'topHalfDepth', 'bottomHalfWidth', 'bottomHalfDepth'] as const;

export function sameShape(a: VertebraPose, b: VertebraPose): boolean {
  return a.level === b.level && SHAPE_FIELDS.every((key) => a[key] === b[key]);
}

/** Straight centres and the shortest quaternion arc, sampled without a clock. */
export function poseBetween(from: VertebraPose, to: VertebraPose, progress: number): VertebraPose {
  if (!Number.isFinite(progress) || !sameShape(from, to)) throw new RangeError('Incompatible poses');
  const p = Math.max(0, Math.min(1, progress));
  const quaternion = (r: readonly number[]) => {
    const angle = Math.hypot(...r), scale = angle === 0 ? .5 : Math.sin(angle / 2) / angle;
    return [(r[0] ?? 0) * scale, (r[1] ?? 0) * scale, (r[2] ?? 0) * scale, Math.cos(angle / 2)];
  };
  const a = quaternion(from.rotation), b = quaternion(to.rotation);
  let dot = a.reduce((sum, x, i) => sum + x * (b[i] ?? 0), 0);
  if (dot < 0) { b.forEach((x, i) => { b[i] = -x; }); dot = -dot; }
  const angle = Math.acos(Math.min(1, dot)), sine = Math.sin(angle);
  const first = sine < 1e-8 ? 1 - p : Math.sin((1 - p) * angle) / sine;
  const last = sine < 1e-8 ? p : Math.sin(p * angle) / sine;
  const q = a.map((x, i) => first * x + last * (b[i] ?? 0));
  const norm = Math.hypot(...q); q.forEach((x, i) => { q[i] = x / norm; });
  const w = Math.max(-1, Math.min(1, q[3] ?? 1)), halfSine = Math.hypot(q[0] ?? 0, q[1] ?? 0, q[2] ?? 0);
  const scale = halfSine < 1e-8 ? 2 : 2 * Math.atan2(halfSine, w) / halfSine;
  return { ...from, centre: [0, 1, 2].map((i) => (from.centre[i] ?? 0) * (1 - p) + (to.centre[i] ?? 0) * p) as [number, number, number], rotation: [(q[0] ?? 0) * scale, (q[1] ?? 0) * scale, (q[2] ?? 0) * scale] };
}
