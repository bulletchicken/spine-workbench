// The see-through line version of a bone: a coarse polygon mesh of it,
// drawn as lines. A bone's own triangles are too many and too uneven to
// read as a mesh: drawn as lines they are a scribble. So the bone is made
// coarse first. Its points are gathered into cubes a few millimetres
// across; each cube's points become one, at their middle; a triangle whose
// corners fall in three different cubes is kept, and its sides are the
// lines. What is left is the bone's shape in a few hundred polygons.

/** The side of the cubes points are gathered into, mm. */
export const CAGE_CUBE = 8;

/**
 * The lines of a surface's coarse mesh: x, y, z of one end, then of the
 * other, for each line. `points` holds x, y, z for each point in
 * millimetres, `triangles` three point numbers for each triangle.
 */
export function cageLines(points: Float32Array, triangles: Uint16Array, cube = CAGE_CUBE): Float32Array {
  const count = points.length / 3;
  /** The gathered point each point belongs to. */
  const gathered = new Uint32Array(count);
  const cubes = new Map<string, number>();
  const sums: number[] = [];
  const sizes: number[] = [];
  for (let p = 0; p < count; p += 1) {
    const [x, y, z] = [points[p * 3] ?? 0, points[p * 3 + 1] ?? 0, points[p * 3 + 2] ?? 0];
    const key = `${String(Math.floor(x / cube))},${String(Math.floor(y / cube))},${String(Math.floor(z / cube))}`;
    let at = cubes.get(key);
    if (at === undefined) {
      at = sizes.length;
      cubes.set(key, at);
      sums.push(0, 0, 0);
      sizes.push(0);
    }
    gathered[p] = at;
    sums[at * 3] = (sums[at * 3] ?? 0) + x;
    sums[at * 3 + 1] = (sums[at * 3 + 1] ?? 0) + y;
    sums[at * 3 + 2] = (sums[at * 3 + 2] ?? 0) + z;
    sizes[at] = (sizes[at] ?? 0) + 1;
  }

  const many = sizes.length;
  const sides = new Set<number>();
  for (let t = 0; t + 2 < triangles.length; t += 3) {
    const [a, b, c] = [gathered[triangles[t] ?? 0] ?? 0, gathered[triangles[t + 1] ?? 0] ?? 0, gathered[triangles[t + 2] ?? 0] ?? 0];
    if (a === b || b === c || c === a) continue;
    for (const [from, to] of [[a, b], [b, c], [c, a]] as const) sides.add(Math.min(from, to) * many + Math.max(from, to));
  }

  const lines = new Float32Array(sides.size * 6);
  let at = 0;
  for (const side of [...sides].sort((one, other) => one - other)) {
    for (const end of [Math.floor(side / many), side % many]) {
      const size = sizes[end] ?? 1;
      lines[at] = (sums[end * 3] ?? 0) / size;
      lines[at + 1] = (sums[end * 3 + 1] ?? 0) / size;
      lines[at + 2] = (sums[end * 3 + 2] ?? 0) / size;
      at += 3;
    }
  }
  return lines;
}
