// How open each point of a bone is to the light around it, worked out once
// from the bone's own shape: the canal, the notches and the roots of the
// processes come out darker than the open faces of the body. Drawing with
// it costs nothing: it is one number a point, baked into the shape.
//
// The bone is marked into a coarse grid of little cubes. From just above
// each point, a spread of straight lines is walked outwards through the
// grid; a line that meets bone within reach is shut. A point's openness is
// the share of its lines that got out, each counted by how squarely it
// leaves the surface.

/** The side of a grid cube, mm. */
const CUBE = 1.5;
/** How far a line is walked before it counts as having got out, mm. */
const REACH = 16;
/** How many lines are spread round the whole sphere; about half face out from any one point. */
const LINES = 48;

/** `count` directions spread evenly over the sphere. */
function spread(count: number): [number, number, number][] {
  const golden = Math.PI * (3 - Math.sqrt(5));
  return Array.from({ length: count }, (_, i) => {
    const z = 1 - (2 * (i + 0.5)) / count;
    const r = Math.sqrt(1 - z * z);
    return [r * Math.cos(golden * i), r * Math.sin(golden * i), z];
  });
}

/** Each point's normal: the mean of its triangles' normals, the larger triangles counting for more. */
export function pointNormals(points: Float32Array, triangles: Uint16Array): Float32Array {
  const normals = new Float32Array(points.length);
  for (let i = 0; i + 2 < triangles.length; i += 3) {
    const [a, b, c] = [(triangles[i] ?? 0) * 3, (triangles[i + 1] ?? 0) * 3, (triangles[i + 2] ?? 0) * 3];
    const [ux, uy, uz] = [(points[b] ?? 0) - (points[a] ?? 0), (points[b + 1] ?? 0) - (points[a + 1] ?? 0), (points[b + 2] ?? 0) - (points[a + 2] ?? 0)];
    const [vx, vy, vz] = [(points[c] ?? 0) - (points[a] ?? 0), (points[c + 1] ?? 0) - (points[a + 1] ?? 0), (points[c + 2] ?? 0) - (points[a + 2] ?? 0)];
    const [nx, ny, nz] = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
    for (const at of [a, b, c]) {
      normals[at] = (normals[at] ?? 0) + nx;
      normals[at + 1] = (normals[at + 1] ?? 0) + ny;
      normals[at + 2] = (normals[at + 2] ?? 0) + nz;
    }
  }
  for (let i = 0; i + 2 < normals.length; i += 3) {
    const size = Math.hypot(normals[i] ?? 0, normals[i + 1] ?? 0, normals[i + 2] ?? 0) || 1;
    normals[i] = (normals[i] ?? 0) / size;
    normals[i + 1] = (normals[i + 1] ?? 0) / size;
    normals[i + 2] = (normals[i + 2] ?? 0) / size;
  }
  return normals;
}

/**
 * How open each point of a closed surface is: 1 on an open face, toward 0
 * deep in a hollow. `points` holds x, y, z for each point in millimetres,
 * `triangles` three point numbers for each triangle.
 */
export function hollowShade(points: Float32Array, triangles: Uint16Array): Float32Array {
  const count = points.length / 3;
  const low = [Infinity, Infinity, Infinity];
  const high = [-Infinity, -Infinity, -Infinity];
  for (let i = 0; i < points.length; i += 1) {
    const axis = i % 3;
    low[axis] = Math.min(low[axis] ?? 0, points[i] ?? 0);
    high[axis] = Math.max(high[axis] ?? 0, points[i] ?? 0);
  }
  const [lx, ly, lz] = [(low[0] ?? 0) - CUBE, (low[1] ?? 0) - CUBE, (low[2] ?? 0) - CUBE];
  const nx = Math.ceil(((high[0] ?? 0) - lx) / CUBE) + 2;
  const ny = Math.ceil(((high[1] ?? 0) - ly) / CUBE) + 2;
  const nz = Math.ceil(((high[2] ?? 0) - lz) / CUBE) + 2;
  const bone = new Uint8Array(nx * ny * nz);
  const cubeAt = (x: number, y: number, z: number): number => {
    const [i, j, k] = [Math.floor((x - lx) / CUBE), Math.floor((y - ly) / CUBE), Math.floor((z - lz) / CUBE)];
    return i < 0 || j < 0 || k < 0 || i >= nx || j >= ny || k >= nz ? -1 : (k * ny + j) * nx + i;
  };

  // Mark every cube the surface passes through, by stepping over each triangle more finely than a cube.
  for (let t = 0; t + 2 < triangles.length; t += 3) {
    const [a, b, c] = [(triangles[t] ?? 0) * 3, (triangles[t + 1] ?? 0) * 3, (triangles[t + 2] ?? 0) * 3];
    const [ax, ay, az] = [points[a] ?? 0, points[a + 1] ?? 0, points[a + 2] ?? 0];
    const [ux, uy, uz] = [(points[b] ?? 0) - ax, (points[b + 1] ?? 0) - ay, (points[b + 2] ?? 0) - az];
    const [vx, vy, vz] = [(points[c] ?? 0) - ax, (points[c + 1] ?? 0) - ay, (points[c + 2] ?? 0) - az];
    const longest = Math.max(Math.hypot(ux, uy, uz), Math.hypot(vx, vy, vz), Math.hypot(ux - vx, uy - vy, uz - vz));
    const steps = Math.max(1, Math.ceil(longest / (0.6 * CUBE)));
    for (let i = 0; i <= steps; i += 1) {
      for (let j = 0; j <= steps - i; j += 1) {
        const [s, r] = [i / steps, j / steps];
        const cube = cubeAt(ax + ux * s + vx * r, ay + uy * s + vy * r, az + uz * s + vz * r);
        if (cube >= 0) bone[cube] = 1;
      }
    }
  }

  const normals = pointNormals(points, triangles);
  const lines = spread(LINES);
  const open = new Float32Array(count);
  const stride = 0.9 * CUBE;
  for (let p = 0; p < count; p += 1) {
    const [nxp, nyp, nzp] = [normals[p * 3] ?? 0, normals[p * 3 + 1] ?? 0, normals[p * 3 + 2] ?? 0];
    // Start clear of the point's own cubes.
    const [sx, sy, sz] = [(points[p * 3] ?? 0) + nxp * 1.75 * CUBE, (points[p * 3 + 1] ?? 0) + nyp * 1.75 * CUBE, (points[p * 3 + 2] ?? 0) + nzp * 1.75 * CUBE];
    let got = 0;
    let all = 0;
    for (const [dx, dy, dz] of lines) {
      const squarely = dx * nxp + dy * nyp + dz * nzp;
      if (squarely < 0.1) continue;
      all += squarely;
      let shut = false;
      for (let far = 0; far <= REACH; far += stride) {
        const cube = cubeAt(sx + dx * far, sy + dy * far, sz + dz * far);
        if (cube < 0) break;
        if (bone[cube] === 1) {
          shut = true;
          break;
        }
      }
      if (!shut) got += squarely;
    }
    open[p] = all === 0 ? 1 : got / all;
  }

  // Even it out along the surface, twice: a point takes half its neighbours' mean.
  const sums = new Float32Array(count);
  const shared = new Uint16Array(count);
  for (let pass = 0; pass < 2; pass += 1) {
    sums.fill(0);
    shared.fill(0);
    for (let t = 0; t + 2 < triangles.length; t += 3) {
      const corners = [triangles[t] ?? 0, triangles[t + 1] ?? 0, triangles[t + 2] ?? 0];
      for (let k = 0; k < 3; k += 1) {
        const [from, to] = [corners[k] ?? 0, corners[(k + 1) % 3] ?? 0];
        sums[from] = (sums[from] ?? 0) + (open[to] ?? 0);
        shared[from] = (shared[from] ?? 0) + 1;
      }
    }
    for (let p = 0; p < count; p += 1) {
      const neighbours = shared[p] ?? 0;
      if (neighbours > 0) open[p] = 0.5 * (open[p] ?? 0) + (0.5 * (sums[p] ?? 0)) / neighbours;
    }
  }
  return open;
}
