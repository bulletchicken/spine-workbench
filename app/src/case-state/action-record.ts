// What an action was, as data. The store keeps no sentences: words/
// turns a record into "Undo: Split T12" and "Undid: split T12.".

import type { CaseFileV1, FilmKind, Level } from '@/shapes';
import type { CaseEdit, CaseEditKind, MarksCause } from './case-edits';
import { editedFilm } from './case-edits';
import { vertebraOf } from './selectors';

/**
 * What a caller may call an action instead of what its edits say:
 * `confirm-vertebra` is "Looks right", one `move-mark` per corner a
 * person can see, each to where it already is.
 */
export type CalledAction = 'confirm-vertebra';

export interface ActionRecord {
  /** What the caller called it, else the kind of its first kept edit. */
  kind: CaseEditKind | CalledAction;
  /** For an engine answer (`replace-marks`), what produced it; otherwise `null`. */
  cause: MarksCause | null;
  /** Every film the action touched, in the order it first touched them. */
  films: FilmKind[];
  /**
   * The vertebra the first kept edit was aimed at, with its level as it
   * was named just before the action ("split T12" names the T12 that was
   * split); `null` for an edit aimed at no vertebra, or at one the film
   * did not hold.
   */
  aimedAt: { film: FilmKind; vertebra: number; level: Level } | null;
}

/** The record of an action whose kept edits are `kept` (at least one), applied to `before`. */
export function recordOf(before: CaseFileV1, kept: readonly CaseEdit[], called: CalledAction | null): ActionRecord {
  const [first] = kept;
  if (first === undefined) throw new Error('an action is recorded only when it keeps an edit');
  const films: FilmKind[] = [];
  for (const edit of kept) {
    const film = editedFilm(edit);
    if (film !== null && !films.includes(film)) films.push(film);
  }
  const film = editedFilm(first);
  const vertebra = aimedVertebra(first);
  const level = film === null || vertebra === null ? null : (vertebraOf(before, film, vertebra)?.level ?? null);
  return {
    kind: called ?? first.kind,
    cause: first.kind === 'replace-marks' ? first.cause : null,
    films,
    aimedAt: film === null || vertebra === null || level === null ? null : { film, vertebra, level },
  };
}

function aimedVertebra(edit: CaseEdit): number | null {
  switch (edit.kind) {
    case 'replace-marks': {
      const cause = edit.cause;
      if (cause.by === 'region-wand' || cause.by === 'boundary-wand') return cause.vertebra;
      if (cause.by === 'pin' || cause.by === 'unpin') return cause.target.on === 'landmark' ? null : cause.target.vertebra;
      return null;
    }
    case 'move-mark':
    case 'pin-mark':
      return edit.target.on === 'landmark' ? null : edit.target.vertebra;
    case 'hide-pedicle':
    case 'confirm-l5':
      return edit.vertebra;
    case 'open-case':
    case 'set-film':
    case 'remove-film':
    case 'set-geometry':
    case 'flip-brightness':
    case 'set-blanked':
      return null;
  }
}
