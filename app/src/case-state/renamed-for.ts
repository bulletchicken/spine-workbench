// Swapping a film's left/right (or anterior/posterior) names after its
// facing changes: the TypeScript mirror of `FilmMarks::renamed_for` in
// engine/shapes/src/vertebra.rs. It is mirrored rather than called
// because the case reducer is pure and synchronous on the main thread,
// where no engine is loaded. renamed-for.test.ts holds it to
// proving-ground/known-answers/side-swap.json, the answers the Rust one
// gives.

import type { FilmMarks, HipCentre, ImageRight, Landmarks, VertebraMarks } from '@/shapes';
import { c7Of } from './c7-of';

/**
 * These marks with their names given for `facing`: the same marks when
 * `facing` is already their `namedFor`, otherwise every pair swapped.
 * Each point keeps its position and flags; only which name holds it
 * changes. Levels, ids, confidence, outlines and naming stay as they are,
 * and `c7` is worked out again from the same four corners.
 */
export function renamedFor(marks: FilmMarks, facing: ImageRight): FilmMarks {
  if (facing === marks.namedFor) return marks;
  const vertebrae = marks.vertebrae.map((vertebra) => ({ ...vertebra, marks: swapped(vertebra.marks) }));
  const landmarks: Landmarks = {
    c7: c7Of(vertebrae),
    sacrumTop: swappedSacrum(marks.landmarks.sacrumTop),
    hips: marks.landmarks.hips.map(swappedHip),
  };
  return { ...marks, vertebrae, landmarks, namedFor: facing };
}

function swapped(marks: VertebraMarks): VertebraMarks {
  switch (marks.kind) {
    case 'front': {
      const { supL, supR, infL, infR, pedL, pedR } = marks.points;
      return { kind: 'front', points: { supL: supR, supR: supL, infL: infR, infR: infL, pedL: pedR, pedR: pedL } };
    }
    case 'side': {
      const { supA, supP, infA, infP } = marks.points;
      return { kind: 'side', points: { supA: supP, supP: supA, infA: infP, infP: infA } };
    }
    case 'bending': {
      const { sup, inf } = marks.lines;
      return { kind: 'bending', lines: { sup: [sup[1], sup[0]], inf: [inf[1], inf[0]] } };
    }
  }
}

function swappedSacrum(top: Landmarks['sacrumTop']): Landmarks['sacrumTop'] {
  if (top === null) return null;
  return top.view === 'front' ? { view: 'front', sacL: top.sacR, sacR: top.sacL } : { view: 'side', sacA: top.sacP, sacP: top.sacA };
}

function swappedHip(hip: HipCentre): HipCentre {
  if (hip.side === 'left') return { ...hip, side: 'right' };
  if (hip.side === 'right') return { ...hip, side: 'left' };
  return hip;
}
