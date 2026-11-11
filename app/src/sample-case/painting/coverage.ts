// How much of each pixel a filled or stroked path covers, 0 to 1, in the
// box of pixels it reaches. Pixel (i, j) is the square from (i, j) to
// (i + 1, j + 1), as on a canvas.
//
// A fill follows the nonzero rule, as a canvas does: across each row it is
// exact, and down each pixel it is the mean of four rows. A stroke has
// round joins and round caps: a pixel's cover falls from 1 to 0 as its
// centre goes from half a pixel inside the stroke's edge to half a pixel
// outside it.

import type { Path } from './path';

export interface Coverage {
  /** The box's top-left pixel. */
  x0: number;
  y0: number;
  width: number;
  height: number;
  /** Row-major, `width` × `height`. */
  cover: Float32Array;
}

const ROWS_PER_PIXEL = 4;

const empty = (): Coverage => ({ x0: 0, y0: 0, width: 0, height: 0, cover: new Float32Array(0) });

function boxOf(path: Path, pad: number, filmWidth: number, filmHeight: number) {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const { points } of path.subpaths) {
    for (let k = 0; k + 1 < points.length; k += 2) {
      const x = points[k] ?? 0;
      const y = points[k + 1] ?? 0;
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
      minY = Math.min(minY, y);
      maxY = Math.max(maxY, y);
    }
  }
  const x0 = Math.max(0, Math.floor(minX - pad));
  const y0 = Math.max(0, Math.floor(minY - pad));
  const x1 = Math.min(filmWidth, Math.ceil(maxX + pad));
  const y1 = Math.min(filmHeight, Math.ceil(maxY + pad));
  return { x0, y0, width: Math.max(0, x1 - x0), height: Math.max(0, y1 - y0) };
}

/** The cover of the path filled, every subpath closed. */
export function fillCoverage(path: Path, filmWidth: number, filmHeight: number): Coverage {
  const box = boxOf(path, 0, filmWidth, filmHeight);
  if (box.width === 0 || box.height === 0) return empty();

  // Every edge as x0, y0, x1, y1.
  const edges: number[] = [];
  for (const { points } of path.subpaths) {
    const n = points.length / 2;
    for (let k = 0; k < n; k++) {
      const next = (k + 1) % n;
      edges.push(points[2 * k] ?? 0, points[2 * k + 1] ?? 0, points[2 * next] ?? 0, points[2 * next + 1] ?? 0);
    }
  }

  const cover = new Float32Array(box.width * box.height);
  const crossings: { x: number; winding: number }[] = [];
  const weight = 1 / ROWS_PER_PIXEL;
  const left = box.x0;
  const right = box.x0 + box.width;

  for (let row = 0; row < box.height; row++) {
    const offset = row * box.width;
    for (let sub = 0; sub < ROWS_PER_PIXEL; sub++) {
      const y = box.y0 + row + (sub + 0.5) / ROWS_PER_PIXEL;
      crossings.length = 0;
      for (let e = 0; e < edges.length; e += 4) {
        const ax = edges[e] ?? 0;
        const ay = edges[e + 1] ?? 0;
        const bx = edges[e + 2] ?? 0;
        const by = edges[e + 3] ?? 0;
        if (ay === by) continue;
        const down = by > ay;
        const top = down ? ay : by;
        const bottom = down ? by : ay;
        if (y < top || y >= bottom) continue;
        crossings.push({ x: ax + ((y - ay) * (bx - ax)) / (by - ay), winding: down ? 1 : -1 });
      }
      crossings.sort((p, q) => p.x - q.x);
      let winding = 0;
      for (let c = 0; c + 1 < crossings.length; c++) {
        const here = crossings[c];
        const next = crossings[c + 1];
        if (here === undefined || next === undefined) break;
        winding += here.winding;
        if (winding === 0) continue;
        const xa = Math.max(left, here.x);
        const xb = Math.min(right, next.x);
        if (xb <= xa) continue;
        const ia = Math.floor(xa);
        const ib = Math.floor(xb);
        if (ia === ib) {
          cover[offset + ia - left] = (cover[offset + ia - left] ?? 0) + (xb - xa) * weight;
          continue;
        }
        cover[offset + ia - left] = (cover[offset + ia - left] ?? 0) + (ia + 1 - xa) * weight;
        for (let i = ia + 1; i < ib; i++) cover[offset + i - left] = (cover[offset + i - left] ?? 0) + weight;
        if (ib < right) cover[offset + ib - left] = (cover[offset + ib - left] ?? 0) + (xb - ib) * weight;
      }
    }
  }
  return { ...box, cover };
}

/** The cover of the path stroked `lineWidth` wide. */
export function strokeCoverage(path: Path, lineWidth: number, filmWidth: number, filmHeight: number): Coverage {
  const half = lineWidth / 2;
  const box = boxOf(path, half + 1, filmWidth, filmHeight);
  if (box.width === 0 || box.height === 0) return empty();

  const distance = new Float32Array(box.width * box.height).fill(Infinity);
  const segment = (ax: number, ay: number, bx: number, by: number) => {
    const dx = bx - ax;
    const dy = by - ay;
    const length2 = dx * dx + dy * dy;
    const i0 = Math.max(box.x0, Math.floor(Math.min(ax, bx) - half - 1));
    const i1 = Math.min(box.x0 + box.width, Math.ceil(Math.max(ax, bx) + half + 1));
    const j0 = Math.max(box.y0, Math.floor(Math.min(ay, by) - half - 1));
    const j1 = Math.min(box.y0 + box.height, Math.ceil(Math.max(ay, by) + half + 1));
    for (let j = j0; j < j1; j++) {
      const py = j + 0.5;
      const offset = (j - box.y0) * box.width - box.x0;
      for (let i = i0; i < i1; i++) {
        const px = i + 0.5;
        const t = length2 === 0 ? 0 : Math.min(1, Math.max(0, ((px - ax) * dx + (py - ay) * dy) / length2));
        const d = Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
        if (d < (distance[offset + i] ?? Infinity)) distance[offset + i] = d;
      }
    }
  };
  for (const { points, closed } of path.subpaths) {
    const n = points.length / 2;
    if (n === 1) segment(points[0] ?? 0, points[1] ?? 0, points[0] ?? 0, points[1] ?? 0);
    const last = closed ? n : n - 1;
    for (let k = 0; k < last; k++) {
      const next = (k + 1) % n;
      segment(points[2 * k] ?? 0, points[2 * k + 1] ?? 0, points[2 * next] ?? 0, points[2 * next + 1] ?? 0);
    }
  }

  const cover = new Float32Array(distance.length);
  for (let k = 0; k < cover.length; k++) {
    cover[k] = Math.min(1, Math.max(0, half - (distance[k] ?? Infinity) + 0.5));
  }
  return { ...box, cover };
}
