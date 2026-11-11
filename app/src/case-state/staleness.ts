// When an engine answer is stale: the two cases in the header of
// case-edits.ts, and only these.

import type { CaseFileV1, FilmKind } from '@/shapes';
import type { CaseEdit } from './case-edits';
import { marksOf } from './selectors';

/** Why an engine answer was dropped unapplied. */
export type Stale =
  /** Its own film changed after the version the answer was computed from. */
  | 'film-changed'
  /** It holds a vertebra id its film does not hold now, below `nextVertebraId`: given out since, perhaps. */
  | 'id-given-out';

/**
 * Why `edit` must be dropped, or `null` when it may land. Only an edit
 * with `basedOn` (an engine answer) can be stale. `changedAt` is, for each
 * film kind, the case version at which that film last changed.
 */
export function staleness(c: CaseFileV1, changedAt: Readonly<Record<FilmKind, number>>, edit: CaseEdit): Stale | null {
  if (edit.kind !== 'replace-marks') return null;
  const film = edit.marks.film;
  if (changedAt[film] > edit.basedOn) return 'film-changed';
  const held = new Set(marksOf(c, film)?.vertebrae.map((vertebra) => vertebra.id));
  const givenOut = edit.marks.vertebrae.some((vertebra) => !held.has(vertebra.id) && vertebra.id < c.nextVertebraId);
  return givenOut ? 'id-given-out' : null;
}
