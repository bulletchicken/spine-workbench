// The rule for "needs a look", on one vertebra of each film kind from the
// sample case, changed one thing at a time.

import { describe, expect, it } from 'vitest';
import type { ConfidenceBand, FilmKind, Mark, VertebraOnFilm } from '@/shapes';
import { caseAt } from '@/sample-case/stages';
import { filledIn, needsALook } from './needs-a-look';

/** A vertebra from the middle of the film, every corner seen, at the given band. */
function vertebraOn(kind: FilmKind, band: ConfidenceBand): VertebraOnFilm {
  const film = caseAt('pass-result').case.marks.find((marks) => marks.film === kind);
  const vertebra = structuredClone(film?.vertebrae[8]);
  if (vertebra === undefined) throw new Error(`the sample case has no vertebra on ${kind}`);
  for (const corner of cornersOf(vertebra)) corner.inferred = false;
  vertebra.confidence.band = band;
  return vertebra;
}

/** The corners the rule asks, as the test changes them. */
function cornersOf(vertebra: VertebraOnFilm): Mark[] {
  const { marks } = vertebra;
  switch (marks.kind) {
    case 'front':
      return [marks.points.supL, marks.points.supR, marks.points.infL, marks.points.infR];
    case 'side':
      return [marks.points.supA, marks.points.supP, marks.points.infA, marks.points.infP];
    case 'bending':
      return [...marks.lines.sup, ...marks.lines.inf];
  }
}

const KINDS: FilmKind[] = ['front', 'side', 'bend-left'];

describe('needsALook', () => {
  it.each(KINDS)('asks for a look at a low band on %s', (kind) => {
    expect(needsALook(vertebraOn(kind, 'low'))).toBe(true);
  });

  it.each(KINDS)('asks for a look at one filled-in corner on %s, at any band', (kind) => {
    for (const [i] of cornersOf(vertebraOn(kind, 'high')).entries()) {
      const vertebra = vertebraOn(kind, 'high');
      const corner = cornersOf(vertebra)[i];
      if (corner === undefined) throw new Error('no corner');
      corner.inferred = true;
      expect(filledIn(vertebra)).toBe(true);
      expect(needsALook(vertebra)).toBe(true);
    }
  });

  it.each(KINDS)('does not ask at a medium or high band with every corner seen on %s', (kind) => {
    expect(needsALook(vertebraOn(kind, 'medium'))).toBe(false);
    expect(needsALook(vertebraOn(kind, 'high'))).toBe(false);
    expect(filledIn(vertebraOn(kind, 'high'))).toBe(false);
  });

  it('does not ask about pedicles', () => {
    const vertebra = vertebraOn('front', 'high');
    if (vertebra.marks.kind !== 'front') throw new Error('a front film holds front marks');
    vertebra.marks.points.pedL = { seen: 'not-visible', source: 'auto' };
    vertebra.marks.points.pedR = { seen: 'visible', at: [0, 0], source: 'auto', pinned: false, inferred: true, ringScore: null };
    expect(needsALook(vertebra)).toBe(false);
  });
});
