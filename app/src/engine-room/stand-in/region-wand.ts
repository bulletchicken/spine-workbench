// The stand-in's region wand: it moves the wall of the outline nearest the
// click so it runs through the click, outward to add and inward to take
// away. Plain geometry over the marks each call carries; nothing here
// looks at the film's pixels.

import type { FilmMarks, Mark, MissReason, Px, RegionWandInput, RegionWandOutput, VertebraOnFilm } from '@/shapes';
import { EngineFailure } from '../engine-client';
import {
  add,
  axisOf,
  c7Of,
  centroid,
  cornersOf,
  distance,
  distanceToPolygon,
  distanceToSegment,
  dot,
  isInside,
  quadOf,
  sub,
  times,
  withCorners,
  type Corners,
} from './quads';
import type { AdjustScript } from './scripts/adjust';
import { isBending, markedEdited, outsideFilm, refusal, type ToolFilm } from './tool-film';

export function regionWand(film: ToolFilm, input: RegionWandInput, script: AdjustScript): RegionWandOutput {
  const { marks, caseVersion } = input;
  const click = input.clicks.at(-1);
  if (click === undefined) throw new EngineFailure({ code: 'tool-refused', message: 'a region wand call carries no click', detail: null });
  if (isBending(film.kind)) throw refusal('not-supported-on-film');
  if (outsideFilm(film, click.at)) throw refusal('outside-film');
  const index = marks.vertebrae.findIndex((vertebra) => vertebra.id === input.vertebra);
  const vertebra = marks.vertebrae[index];
  if (vertebra === undefined) throw new EngineFailure({ code: 'tool-unknown-vertebra', message: 'no vertebra on this film has that id', detail: String(input.vertebra) });
  const refused = regionRefusal(vertebra, script);
  if (refused !== null) throw refusal(refused);

  const quad = quadOf(vertebra);
  const inward = centroid(quad);
  const walls = [0, 1, 2, 3].map((wall) => ({ wall, gap: distanceToSegment(click.at, quad[wall] as Px, quad[(wall + 1) % 4] as Px) }));
  const nearest = walls.reduce((best, wall) => (wall.gap < best.gap ? wall : best));
  const a = quad[nearest.wall] as Px;
  const b = quad[(nearest.wall + 1) % 4] as Px;
  const along = sub(b, a);
  const outward = normalised(dot([-along[1], along[0]], sub(a, inward)) >= 0 ? [-along[1], along[0]] : [along[1], -along[0]]);
  const move = dot(sub(click.at, a), outward);
  const grows = move > 0;
  const changes = click.polarity === 'add' ? grows : !grows;

  const corners = cornersOf(vertebra).map((corner, slot) => {
    if (!changes || corner.pinned || (slot !== nearest.wall && slot !== (nearest.wall + 1) % 4)) return corner;
    return { ...corner, at: add(corner.at, times(outward, move)), source: 'region-wand', inferred: false } satisfies Mark;
  }) as Corners;
  const reshaped = withCorners(vertebra, corners);
  const outline = quadOf(reshaped);
  const edited: VertebraOnFilm = { ...markedEdited(reshaped), outline };
  const vertebrae = marks.vertebrae.map((each, at) => (at === index ? edited : each));
  return {
    caseVersion,
    marks: { ...marks, vertebrae, landmarks: { ...marks.landmarks, c7: c7Of(vertebrae) } },
    changed: [vertebra.id],
    outline,
    growth: [],
    solverIterations: 1,
    solverResidual: 0,
    warnings: [],
  };
}

export type RegionHit = { kind: 'add' | 'subtract'; vertebra: VertebraOnFilm } | { kind: 'none'; reason: MissReason };

export function regionHit(film: ToolFilm, marks: FilmMarks, at: Px, script: AdjustScript): RegionHit {
  if (isBending(film.kind)) return { kind: 'none', reason: 'not-supported-on-film' };
  if (outsideFilm(film, at)) return { kind: 'none', reason: 'outside-film' };
  const inside = marks.vertebrae.filter((vertebra) => isInside(quadOf(vertebra), at));
  let target: { kind: 'add' | 'subtract'; vertebra: VertebraOnFilm };
  if (inside.length > 0) {
    const nearest = inside.reduce((best, vertebra) => (distance(centroid(quadOf(vertebra)), at) < distance(centroid(quadOf(best)), at) ? vertebra : best));
    target = { kind: 'subtract', vertebra: nearest };
  } else {
    const reach = marks.vertebrae
      .map((vertebra) => ({ vertebra, gap: distanceToPolygon(quadOf(vertebra), at), height: axisOf(quadOf(vertebra)).height }))
      .filter((candidate) => candidate.gap <= script.regionReach * candidate.height)
      .sort((a, b) => a.gap - b.gap);
    const [best, next] = reach;
    if (best === undefined) return { kind: 'none', reason: 'off-spine' };
    if (next !== undefined && next.gap - best.gap <= Math.max(2, 0.03 * best.height)) return { kind: 'none', reason: 'ambiguous' };
    target = { kind: 'add', vertebra: best.vertebra };
  }
  const refused = regionRefusal(target.vertebra, script);
  return refused === null ? target : { kind: 'none', reason: refused };
}

function regionRefusal(vertebra: VertebraOnFilm, script: AdjustScript): MissReason | null {
  if (axisOf(quadOf(vertebra)).height < script.smallestRegionPx) return 'too-small';
  if (cornersOf(vertebra).every((corner) => corner.pinned)) return 'all-pinned';
  return null;
}

function normalised(vector: Px): Px {
  const length = Math.hypot(vector[0], vector[1]);
  return length === 0 ? [0, 0] : times(vector, 1 / length);
}
