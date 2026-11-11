// One case edit applied to the case, as the header of case-edits.ts says.
// Pure: it never changes the case it is given. Whether an engine answer
// is stale, the version and the history are the reducer's
// (case-reducer.ts); this is only what each edit does to the case.

import type { CaseFileV1, Film, FilmKind, FilmMarks } from '@/shapes';
import type { CaseEdit } from './case-edits';
import { c7Of } from './c7-of';
import { namedFromL5 } from './name-from-l5';
import { hidePedicle, movePoint, pinPoint } from './point-edits';
import { renamedFor } from './renamed-for';
import { aboveEveryId, compareFilmKinds, filmOf, marksOf } from './selectors';

/**
 * The case after `edit`, or `null` when the edit names something the case
 * does not hold: a film that is not there, a vertebra that is gone, a
 * mark of the other view, a landmark never found, a pedicle that cannot be
 * seen to pin, or an L5 that leaves a vertebra without a name. Such an
 * edit is dropped unapplied.
 *
 * After every edit but `open-case`, `nextVertebraId` is the larger of its
 * value before and one above the highest id in the case, and each film
 * whose vertebrae the edit changed has its `landmarks.c7` worked out
 * again.
 */
export function applyEdit(c: CaseFileV1, edit: CaseEdit): CaseFileV1 | null {
  if (edit.kind === 'open-case') return inOrder(edit.case);
  const changed = changeFilm(c, edit);
  if (changed === null) return null;
  return { ...changed, nextVertebraId: Math.max(c.nextVertebraId, aboveEveryId(changed)) };
}

function changeFilm(c: CaseFileV1, edit: Exclude<CaseEdit, { kind: 'open-case' }>): CaseFileV1 | null {
  switch (edit.kind) {
    case 'set-film':
      return withMarks(withFilm(c, edit.film.kind, edit.film), edit.film.kind, null);
    case 'remove-film':
      if (filmOf(c, edit.film) === null) return null;
      return withMarks(withFilm(c, edit.film, null), edit.film, null);
    case 'set-geometry':
      return setGeometry(c, edit.film, edit.geometry);
    case 'flip-brightness':
      return changeRecord(c, edit.film, (film) => ({
        ...film,
        values: { ...film.values, inverted: !film.values.inverted, invertedByUser: !film.values.invertedByUser },
      }));
    case 'set-blanked':
      return changeRecord(c, edit.film, (film) => ({ ...film, blanked: edit.boxes }));
    case 'replace-marks':
      if (filmOf(c, edit.marks.film) === null) return null;
      return withMarks(c, edit.marks.film, settled(edit.marks));
    case 'move-mark':
      return changeMarks(c, edit.film, (marks) => movePoint(marks, edit.target, edit.to));
    case 'pin-mark':
      return changeMarks(c, edit.film, (marks) => pinPoint(marks, edit.target, edit.pinned));
    case 'hide-pedicle':
      return changeMarks(c, edit.film, (marks) => hidePedicle(marks, edit.vertebra, edit.mark));
    case 'confirm-l5':
      return changeMarks(c, edit.film, (marks) => namedFromL5(marks, edit.vertebra));
  }
}

/**
 * The three cases of `set-geometry`, judged against the geometry before:
 * new quarter turns drop the marks; a new facing renames them for it;
 * anything else leaves them as they are.
 */
function setGeometry(c: CaseFileV1, kind: FilmKind, geometry: Film['geometry']): CaseFileV1 | null {
  const film = filmOf(c, kind);
  if (film === null) return null;
  const before = film.geometry;
  const withGeometry = withFilm(c, kind, { ...film, geometry });
  const marks = marksOf(c, kind);
  if (marks === null) return withGeometry;
  if (geometry.quarterTurns !== before.quarterTurns) return withMarks(withGeometry, kind, null);
  if (geometry.imageRight !== null && geometry.imageRight !== before.imageRight) {
    return withMarks(withGeometry, kind, renamedFor(marks, geometry.imageRight));
  }
  return withGeometry;
}

function changeRecord(c: CaseFileV1, kind: FilmKind, change: (film: Film) => Film): CaseFileV1 | null {
  const film = filmOf(c, kind);
  return film === null ? null : withFilm(c, kind, change(film));
}

function changeMarks(c: CaseFileV1, kind: FilmKind, change: (marks: FilmMarks) => FilmMarks | null): CaseFileV1 | null {
  const marks = marksOf(c, kind);
  const changed = marks === null ? null : change(marks);
  return changed === null ? null : withMarks(c, kind, settled(changed));
}

/** The marks with `c7` worked out again from their vertebrae. */
function settled(marks: FilmMarks): FilmMarks {
  return { ...marks, landmarks: { ...marks.landmarks, c7: c7Of(marks.vertebrae) } };
}

/** The case with the film of this kind replaced, added or (for `null`) removed, films kept in kind order. */
export function withFilm(c: CaseFileV1, kind: FilmKind, film: Film | null): CaseFileV1 {
  const others = c.films.filter((kept) => kept.kind !== kind);
  return { ...c, films: film === null ? others : [...others, film].sort((a, b) => compareFilmKinds(a.kind, b.kind)) };
}

/** The case with this film's marks replaced, added or (for `null`) removed, marks kept in kind order. */
export function withMarks(c: CaseFileV1, kind: FilmKind, marks: FilmMarks | null): CaseFileV1 {
  const others = c.marks.filter((kept) => kept.film !== kind);
  return { ...c, marks: marks === null ? others : [...others, marks].sort((a, b) => compareFilmKinds(a.film, b.film)) };
}

/** An opened case with its films and marks in kind order, so equal cases are equal objects. */
function inOrder(c: CaseFileV1): CaseFileV1 {
  return {
    ...c,
    films: [...c.films].sort((a, b) => compareFilmKinds(a.kind, b.kind)),
    marks: [...c.marks].sort((a, b) => compareFilmKinds(a.film, b.film)),
  };
}
