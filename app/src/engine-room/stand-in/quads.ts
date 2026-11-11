// A vertebra's outline as a quad, and the plain geometry the stand-in's
// tools work with. Corners are in quad order: upper a-side, upper b-side,
// lower b-side, lower a-side, where the a-side is the patient's left on a
// front or bending film and anterior on a side film.

import type { Level, Mark, Px, Template, VertebraOnFilm } from '@/shapes';
import { TEMPLATE_NAMES } from '@/shapes';

export type Quad = [Px, Px, Px, Px];
export type Corners = [Mark, Mark, Mark, Mark];

export const add = (a: Px, b: Px): Px => [a[0] + b[0], a[1] + b[1]];
export const sub = (a: Px, b: Px): Px => [a[0] - b[0], a[1] - b[1]];
export const times = (a: Px, k: number): Px => [a[0] * k, a[1] * k];
export const dot = (a: Px, b: Px): number => a[0] * b[0] + a[1] * b[1];
export const distance = (a: Px, b: Px): number => Math.hypot(a[0] - b[0], a[1] - b[1]);
export const lerp = (a: Px, b: Px, t: number): Px => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
export const middle = (a: Px, b: Px): Px => lerp(a, b, 0.5);

export function cornersOf(vertebra: VertebraOnFilm): Corners {
  const marks = vertebra.marks;
  switch (marks.kind) {
    case 'front':
      return [marks.points.supL, marks.points.supR, marks.points.infR, marks.points.infL];
    case 'side':
      return [marks.points.supA, marks.points.supP, marks.points.infP, marks.points.infA];
    case 'bending':
      return [marks.lines.sup[0], marks.lines.sup[1], marks.lines.inf[1], marks.lines.inf[0]];
  }
}

/** The vertebra with its four corners replaced; pedicles and everything else kept. */
export function withCorners(vertebra: VertebraOnFilm, corners: Corners): VertebraOnFilm {
  const [supA, supB, infB, infA] = corners;
  const marks = vertebra.marks;
  switch (marks.kind) {
    case 'front':
      return { ...vertebra, marks: { kind: 'front', points: { ...marks.points, supL: supA, supR: supB, infR: infB, infL: infA } } };
    case 'side':
      return { ...vertebra, marks: { kind: 'side', points: { supA, supP: supB, infP: infB, infA } } };
    case 'bending':
      return { ...vertebra, marks: { kind: 'bending', lines: { sup: [supA, supB], inf: [infA, infB] } } };
  }
}

export function quadOf(vertebra: VertebraOnFilm): Quad {
  const [a, b, c, d] = cornersOf(vertebra);
  return [a.at, b.at, c.at, d.at];
}

/** The line from the middle of the upper endplate to the middle of the lower one. */
export function axisOf(quad: Quad): { top: Px; bottom: Px; unit: Px; height: number } {
  const top = middle(quad[0], quad[1]);
  const bottom = middle(quad[3], quad[2]);
  const height = distance(top, bottom);
  const unit: Px = height === 0 ? [0, 1] : times(sub(bottom, top), 1 / height);
  return { top, bottom, unit, height };
}

/** How far down the axis `at` lies, as a share of the height (0 at the upper endplate, 1 at the lower). */
export function downShare(quad: Quad, at: Px): number {
  const axis = axisOf(quad);
  return axis.height === 0 ? 0 : dot(sub(at, axis.top), axis.unit) / axis.height;
}

/** The point `across` of the way from the a-side to the b-side and `down` of the way from the upper endplate. */
export function pointIn(quad: Quad, across: number, down: number): Px {
  return lerp(lerp(quad[0], quad[1], across), lerp(quad[3], quad[2], across), down);
}

/** The line across the vertebra `down` of the way from its upper endplate: on its a-side, then its b-side. */
export function lineAcross(quad: Quad, down: number): [Px, Px] {
  return [lerp(quad[0], quad[3], down), lerp(quad[1], quad[2], down)];
}

export function centroid(points: readonly Px[]): Px {
  const sum = points.reduce<Px>((total, point) => add(total, point), [0, 0]);
  return times(sum, 1 / Math.max(1, points.length));
}

/** Whether `at` is inside the polygon (edges count as inside only by chance; a hover never lands exactly on one). */
export function isInside(polygon: readonly Px[], at: Px): boolean {
  let inside = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i, i += 1) {
    const a = polygon[i] as Px;
    const b = polygon[j] as Px;
    if (a[1] > at[1] !== b[1] > at[1] && at[0] < ((b[0] - a[0]) * (at[1] - a[1])) / (b[1] - a[1]) + a[0]) inside = !inside;
  }
  return inside;
}

export function distanceToSegment(at: Px, a: Px, b: Px): number {
  const along = sub(b, a);
  const length = dot(along, along);
  const t = length === 0 ? 0 : Math.min(1, Math.max(0, dot(sub(at, a), along) / length));
  return distance(at, add(a, times(along, t)));
}

/** How far `at` is from the polygon: 0 inside it, else the distance to its nearest edge. */
export function distanceToPolygon(polygon: readonly Px[], at: Px): number {
  if (isInside(polygon, at)) return 0;
  let nearest = Number.POSITIVE_INFINITY;
  for (let i = 0; i < polygon.length; i += 1) {
    nearest = Math.min(nearest, distanceToSegment(at, polygon[i] as Px, polygon[(i + 1) % polygon.length] as Px));
  }
  return nearest;
}

/**
 * The names the vertebrae from `from` on would take after shifting `by`
 * places along the film's count's names, or null when the names run out.
 */
export function shiftedLevels(levels: readonly Level[], from: number, by: number, template: Template): Level[] | null {
  const names = TEMPLATE_NAMES[template];
  const shifted: Level[] = [];
  for (const [index, level] of levels.entries()) {
    if (index < from) {
      shifted.push(level);
      continue;
    }
    const name = names[names.indexOf(level) + by];
    if (names.indexOf(level) < 0 || name === undefined) return null;
    shifted.push(name);
  }
  return shifted;
}

/** The film's C7 landmark: the mean of the C7 vertebra's corners, or null when no vertebra is named C7. */
export function c7Of(vertebrae: readonly VertebraOnFilm[]): Mark | null {
  const c7 = vertebrae.find((vertebra) => vertebra.level === 'C7');
  if (c7 === undefined) return null;
  const corners = cornersOf(c7);
  return { at: centroid(corners.map((corner) => corner.at)), source: 'auto', pinned: false, inferred: corners.some((corner) => corner.inferred) };
}
