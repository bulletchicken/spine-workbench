import { Line3, Triangle, Vector3 } from 'three';
import { type Vec3, along, dot, sub } from '../placing/vectors';
import { CAGE_CUBE, cageMesh } from './cage';

export interface FormationMesh {
  points: Vec3[];
  edges: readonly (readonly [number, number])[];
  rank: number[];
  poles: readonly [Vec3, Vec3];
  cell: Float32Array;
  turn: Float32Array;
}

/** The bone's surface on its own long axis, nearest each endplate. */
export function axisEnds(points: Float32Array, triangles: Uint16Array, height: number): readonly [Vec3, Vec3] {
  const hits: number[] = [];
  for (let i = 0; i < triangles.length; i += 3) {
    const a = (triangles[i] ?? 0) * 3, b = (triangles[i + 1] ?? 0) * 3, c = (triangles[i + 2] ?? 0) * 3;
    const ax = points[a] ?? 0, ay = points[a + 1] ?? 0;
    const bx = (points[b] ?? 0) - ax, by = (points[b + 1] ?? 0) - ay;
    const cx = (points[c] ?? 0) - ax, cy = (points[c + 1] ?? 0) - ay;
    const determinant = bx * cy - by * cx;
    if (Math.abs(determinant) < 1e-10) continue;
    const u = (-ax * cy + ay * cx) / determinant, v = (-ay * bx + ax * by) / determinant;
    if (u >= -1e-8 && v >= -1e-8 && u + v <= 1 + 1e-8) hits.push((points[a + 2] ?? 0) * (1 - u - v) + (points[b + 2] ?? 0) * u + (points[c + 2] ?? 0) * v);
  }
  if (hits.length === 0) throw new Error('bone-axis-misses-surface');
  const near = (target: number) => hits.reduce((best, z) => Math.abs(z - target) < Math.abs(best - target) ? z : best, hits[0] ?? 0);
  return [[0, 0, near(height / 2)], [0, 0, near(-height / 2)]];
}

/** Project along the film's beam. Its thread ends exactly at the original point. */
export function threadFoot(point: Vec3, plane: { towardReader: Vec3; out: number }): Vec3 {
  return along(point, plane.towardReader, plane.out - dot(point, plane.towardReader));
}

/** Shortest paths along the mesh, paced half by distance and half by population. */
export function growthRanks(points: readonly Vec3[], edges: readonly (readonly [number, number])[], seeds: readonly number[]): number[] {
  const distances = points.map(() => Infinity), pending = new Set(seeds);
  const neighbours = points.map(() => [] as { at: number; distance: number }[]);
  for (const [a, b] of edges) {
    const distance = Math.hypot(...sub(points[a] ?? [0, 0, 0], points[b] ?? [0, 0, 0]));
    neighbours[a]?.push({ at: b, distance }); neighbours[b]?.push({ at: a, distance });
  }
  for (const at of seeds) distances[at] = 0;
  while (pending.size > 0) {
    let next = -1;
    for (const at of pending) if (next < 0 || (distances[at] ?? Infinity) < (distances[next] ?? Infinity)) next = at;
    pending.delete(next);
    for (const { at, distance } of neighbours[next] ?? []) {
      const candidate = (distances[next] ?? 0) + distance;
      if (candidate < (distances[at] ?? Infinity)) { distances[at] = candidate; pending.add(at); }
    }
  }
  const finite = distances.filter(Number.isFinite).sort((a, b) => a - b), reach = finite.at(-1) ?? 1;
  return distances.map((distance) => {
    if (distance === 0) return 0;
    if (!Number.isFinite(distance)) return 1;
    const within = finite.findLastIndex((value) => value <= distance) / Math.max(1, finite.length - 1);
    return (distance / (reach || 1) + within) / 2;
  });
}

/** The fitted surface and its coarse net share one pair of exact axis endpoints. */
export function formationMesh(surface: Float32Array, triangles: Uint16Array, height: number): FormationMesh {
  const thin = cageMesh(surface, triangles);
  const poles = axisEnds(surface, triangles, height);
  const points: Vec3[] = [poles[0], poles[1]];
  for (let i = 0; i < thin.points.length; i += 3) points.push([thin.points[i] ?? 0, thin.points[i + 1] ?? 0, thin.points[i + 2] ?? 0]);
  const seen = new Set<string>(), edges: [number, number][] = [];
  const edge = (a: number, b: number) => { const key = `${String(Math.min(a, b))},${String(Math.max(a, b))}`; if (!seen.has(key)) { seen.add(key); edges.push([a, b]); } };
  for (let i = 0; i < thin.triangles.length; i += 3) {
    const a = (thin.triangles[i] ?? 0) + 2, b = (thin.triangles[i + 1] ?? 0) + 2, c = (thin.triangles[i + 2] ?? 0) + 2;
    edge(a, b); edge(b, c); edge(c, a);
  }
  // Join the two exact surface points to the nearest triangle of the coarse net.
  const triangle = new Triangle(), target = new Vector3(), closest = new Vector3();
  for (const seed of [0, 1]) {
    target.fromArray(points[seed] ?? [0, 0, 0]);
    let best = Infinity, face = 0;
    for (let i = 0; i < thin.triangles.length; i += 3) {
      triangle.a.fromArray(points[(thin.triangles[i] ?? 0) + 2] ?? [0, 0, 0]);
      triangle.b.fromArray(points[(thin.triangles[i + 1] ?? 0) + 2] ?? [0, 0, 0]);
      triangle.c.fromArray(points[(thin.triangles[i + 2] ?? 0) + 2] ?? [0, 0, 0]);
      const distance = triangle.closestPointToPoint(target, closest).distanceToSquared(target);
      if (distance < best) { best = distance; face = i; }
    }
    for (let j = 0; j < 3; j++) edge(seed, (thin.triangles[face + j] ?? 0) + 2);
  }
  const rank = growthRanks(points, edges, [0, 1]);
  const cell = new Float32Array(surface.length / 3), turn = new Float32Array(surface.length / 3), line = new Line3();
  // Distance to the net makes the surface arrive from its lines inward, including curved cells.
  for (let i = 0; i < cell.length; i++) {
    target.fromArray(surface, i * 3);
    let nearest = Infinity, order = 0;
    for (const [a, b] of edges) {
      line.start.fromArray(points[a] ?? [0, 0, 0]); line.end.fromArray(points[b] ?? [0, 0, 0]);
      line.closestPointToPoint(target, true, closest);
      const distance = closest.distanceToSquared(target);
      if (distance < nearest) { nearest = distance; const share = line.closestPointToPointParameter(target, true); order = (rank[a] ?? 0) * (1 - share) + (rank[b] ?? 0) * share; }
    }
    cell[i] = Math.min(1, Math.sqrt(nearest) / (CAGE_CUBE / 2)); turn[i] = order;
  }
  return { points, edges: edges.map(([a, b]) => (rank[a] ?? 0) <= (rank[b] ?? 0) ? [a, b] : [b, a]), rank, poles, cell, turn };
}
