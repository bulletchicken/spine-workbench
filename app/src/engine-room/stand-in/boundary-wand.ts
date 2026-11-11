// The stand-in's boundary wand: plain geometry over the marks each call
// carries. It merges two vertebrae when the click is in the gap between
// them or close to it, and splits one when the click is inside it and far
// enough from both endplates. The upper piece keeps its id and name; the
// lower one takes the new id and the next name, and every name below
// shifts by one. Each answer echoes the case version it was asked from,
// and marks the vertebrae the click was aimed at, and only those,
// `edited-by-user`.

import type {
  BoundaryWandInput,
  BoundaryWandOutput,
  FilmMarks,
  Level,
  Mark,
  MissReason,
  PedicleMark,
  Px,
  ToolWarning,
  VertebraLevel,
  VertebraOnFilm,
} from '@/shapes';
import { axisOf, c7Of, cornersOf, distanceToPolygon, dot, downShare, isInside, lineAcross, pointIn, quadOf, shiftedLevels, sub, withCorners } from './quads';
import type { AdjustScript } from './scripts/adjust';
import { isBending, markedEdited, outsideFilm, refusal, type ToolFilm } from './tool-film';

export type BoundaryHit =
  | { kind: 'split'; index: number; down: number; levels: Level[] }
  | { kind: 'merge'; upper: number; levels: Level[] }
  | { kind: 'none'; reason: MissReason };

export function boundaryWand(film: ToolFilm, input: BoundaryWandInput, script: AdjustScript): BoundaryWandOutput {
  const { marks, caseVersion } = input;
  const hit = boundaryHit(film, marks, input.at, script);
  if (hit.kind === 'none') throw refusal(hit.reason);
  const warnings: ToolWarning[] = [];

  if (hit.kind === 'split') {
    const parent = marks.vertebrae[hit.index] as VertebraOnFilm;
    const [upper, lower] = splitVertebra(parent, hit.down, input.newVertebraId, script);
    const vertebrae = [...marks.vertebrae.slice(0, hit.index), upper, lower, ...marks.vertebrae.slice(hit.index + 1)].map((vertebra, index) => ({
      ...vertebra,
      level: hit.levels[index] as Level,
    }));
    const after = rename(marks, vertebrae, warnings);
    return {
      caseVersion,
      action: 'split',
      marks: after,
      changed: changedIds(marks.vertebrae, after.vertebrae, [parent.id, input.newVertebraId]),
      newVertebra: input.newVertebraId,
      removedVertebra: null,
      warnings,
    };
  }

  const upper = marks.vertebrae[hit.upper] as VertebraOnFilm;
  const lower = marks.vertebrae[hit.upper + 1] as VertebraOnFilm;
  const merged = mergeVertebrae(upper, lower, warnings);
  const vertebrae = [...marks.vertebrae.slice(0, hit.upper), merged, ...marks.vertebrae.slice(hit.upper + 2)].map((vertebra, index) => ({
    ...vertebra,
    level: hit.levels[index] as Level,
  }));
  const after = rename(marks, vertebrae, warnings);
  return {
    caseVersion,
    action: 'merge',
    marks: after,
    changed: changedIds(marks.vertebrae, after.vertebrae, [upper.id]),
    newVertebra: null,
    removedVertebra: lower.id,
    warnings,
  };
}

export function boundaryHit(film: ToolFilm, marks: FilmMarks, at: Px, script: AdjustScript): BoundaryHit {
  if (isBending(film.kind)) return { kind: 'none', reason: 'not-supported-on-film' };
  if (outsideFilm(film, at)) return { kind: 'none', reason: 'outside-film' };
  const vertebrae = marks.vertebrae;
  const levels = vertebrae.map((vertebra) => vertebra.level);
  const template = marks.naming.template;

  let gap: { upper: number; reach: number } | null = null;
  for (let upper = 0; upper + 1 < vertebrae.length; upper += 1) {
    const above = quadOf(vertebrae[upper] as VertebraOnFilm);
    const below = quadOf(vertebrae[upper + 1] as VertebraOnFilm);
    const tolerance = (script.mergeReach * (axisOf(above).height + axisOf(below).height)) / 2;
    const reach = distanceToPolygon([above[3], above[2], below[1], below[0]], at);
    if (reach <= tolerance && (gap === null || reach < gap.reach)) gap = { upper, reach };
  }
  if (gap !== null) {
    const merged = [...levels.slice(0, gap.upper + 1), ...levels.slice(gap.upper + 2)];
    const shifted = shiftedLevels(merged, gap.upper + 1, -1, template);
    if (shifted === null) return { kind: 'none', reason: 'names-run-out' };
    return { kind: 'merge', upper: gap.upper, levels: shifted };
  }

  const index = vertebrae.findIndex((vertebra) => isInside(quadOf(vertebra), at));
  if (index >= 0) {
    const down = downShare(quadOf(vertebrae[index] as VertebraOnFilm), at);
    if (down < script.splitMargin || down > 1 - script.splitMargin) return { kind: 'none', reason: 'near-endplate' };
    const parent = levels[index] as Level;
    const split = [...levels.slice(0, index + 1), parent, ...levels.slice(index + 1)];
    const shifted = shiftedLevels(split, index + 1, 1, template);
    if (shifted === null) return { kind: 'none', reason: 'names-run-out' };
    return { kind: 'split', index, down, levels: shifted };
  }

  const first = vertebrae[0];
  const last = vertebrae.at(-1);
  if (first !== undefined && last !== undefined) {
    const top = axisOf(quadOf(first));
    const bottom = axisOf(quadOf(last));
    if (dot(sub(at, top.top), top.unit) < 0 || dot(sub(at, bottom.bottom), bottom.unit) > 0) return { kind: 'none', reason: 'outside-chain' };
  }
  return { kind: 'none', reason: 'off-spine' };
}

function splitVertebra(parent: VertebraOnFilm, down: number, newId: number, script: AdjustScript): [VertebraOnFilm, VertebraOnFilm] {
  const quad = quadOf(parent);
  const [supA, supB, infB, infA] = cornersOf(parent);
  const halfDisc = script.discShare / 2;
  const [upperA, upperB] = lineAcross(quad, Math.max(0, down - halfDisc));
  const [lowerA, lowerB] = lineAcross(quad, Math.min(1, down + halfDisc));
  const placed = (at: Px): Mark => ({ at, source: 'boundary-wand', pinned: false, inferred: false });
  let upper = withCorners(parent, [supA, supB, placed(upperB), placed(upperA)]);
  let lower = withCorners({ ...parent, id: newId }, [placed(lowerA), placed(lowerB), infB, infA]);

  if (parent.marks.kind === 'front') {
    // Each pedicle goes to the piece it lies on; a piece left without one gets it inferred.
    const pedicles = parent.marks.points;
    const onLower = (pedicle: PedicleMark) => pedicle.seen === 'visible' && downShare(quad, pedicle.at) > down;
    const give = (piece: VertebraOnFilm, lowerPiece: boolean): VertebraOnFilm => {
      if (piece.marks.kind !== 'front') return piece;
      const pedicle = (name: 'pedL' | 'pedR', across: number): PedicleMark =>
        onLower(pedicles[name]) === lowerPiece
          ? pedicles[name]
          : { seen: 'visible', at: pointIn(quadOf(piece), across, script.pedicleSpot.down), source: 'boundary-wand', pinned: false, inferred: true, ringScore: null };
      const { across } = script.pedicleSpot;
      return { ...piece, marks: { kind: 'front', points: { ...piece.marks.points, pedL: pedicle('pedL', across), pedR: pedicle('pedR', 1 - across) } } };
    };
    upper = give(upper, false);
    lower = give(lower, true);
  }
  const resolved = (piece: VertebraOnFilm) => markedEdited({ ...piece, outline: null }, ['odd-height']);
  return [resolved(upper), resolved(lower)];
}

function mergeVertebrae(upper: VertebraOnFilm, lower: VertebraOnFilm, warnings: ToolWarning[]): VertebraOnFilm {
  const [supA, supB, infB, infA] = cornersOf(upper);
  const [lowerSupA, lowerSupB, lowerInfB, lowerInfA] = cornersOf(lower);
  const pinsRemoved = [infB, infA, lowerSupA, lowerSupB].filter((corner) => corner.pinned).length;
  if (pinsRemoved > 0) warnings.push({ code: 'pins-removed', count: pinsRemoved });
  let merged = withCorners(upper, [supA, supB, lowerInfB, lowerInfA]);
  if (merged.marks.kind === 'front') {
    const kept = (pedicle: PedicleMark): PedicleMark => (pedicle.seen === 'visible' && !pedicle.pinned ? { ...pedicle, inferred: true } : pedicle);
    merged = { ...merged, marks: { kind: 'front', points: { ...merged.marks.points, pedL: kept(merged.marks.points.pedL), pedR: kept(merged.marks.points.pedR) } } };
  }
  return markedEdited({ ...merged, outline: null });
}

/** The film's marks after a split or merge: names counted again, C7 worked out again, L5's confirmation kept only while its vertebra keeps the name. */
function rename(before: FilmMarks, vertebrae: VertebraOnFilm[], warnings: ToolWarning[]): FilmMarks {
  const levels = vertebrae.map((vertebra) => vertebra.level);
  const anchor = before.naming.l5ConfirmedOn;
  const stillL5 = anchor !== null && vertebrae.some((vertebra) => vertebra.id === anchor && vertebra.level === 'L5');
  if (anchor !== null && !stillL5) warnings.push({ code: 'naming-conflict' });
  return {
    ...before,
    vertebrae,
    naming: {
      ...before.naming,
      fromTop: before.naming.fromTop === null ? null : levels,
      fromBottom: before.naming.fromBottom === null ? null : levels,
      l5ConfirmedOn: stillL5 ? anchor : null,
    },
    landmarks: { ...before.landmarks, c7: c7Of(vertebrae) },
  };
}

export function levelsAfterSplit(vertebrae: readonly VertebraOnFilm[], hit: Extract<BoundaryHit, { kind: 'split' }>, newId: number): VertebraLevel[] {
  const ids = [...vertebrae.slice(0, hit.index + 1).map((vertebra) => vertebra.id), newId, ...vertebrae.slice(hit.index + 1).map((vertebra) => vertebra.id)];
  return levelChanges(vertebrae, ids, hit.levels);
}

export function levelsAfterMerge(vertebrae: readonly VertebraOnFilm[], hit: Extract<BoundaryHit, { kind: 'merge' }>): VertebraLevel[] {
  const ids = vertebrae.filter((_, index) => index !== hit.upper + 1).map((vertebra) => vertebra.id);
  return levelChanges(vertebrae, ids, hit.levels);
}

/** Every vertebra whose level would differ from now, a new one included. */
function levelChanges(before: readonly VertebraOnFilm[], ids: number[], levels: Level[]): VertebraLevel[] {
  return ids.flatMap((id, index) => {
    const level = levels[index] as Level;
    return before.find((vertebra) => vertebra.id === id)?.level === level ? [] : [{ vertebra: id, level }];
  });
}

/** The ids that changed: those aimed at, and any whose name changed. */
function changedIds(before: readonly VertebraOnFilm[], after: readonly VertebraOnFilm[], aimed: number[]): number[] {
  const renamed = after.filter((vertebra) => before.find((was) => was.id === vertebra.id)?.level !== vertebra.level).map((vertebra) => vertebra.id);
  return [...new Set([...aimed, ...renamed])];
}
