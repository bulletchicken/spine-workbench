// Every change the open case can go through. FROZEN: changes only through
// a contract-change pull request.
//
// The case store applies these edits and nothing else, through one pure
// reducer. Rules the reducer keeps:
// - The case version goes up by one on every applied edit, undo and redo
//   included, and is never restored.
// - Staleness is judged per film. The store remembers, for each film
//   kind, the case version at which that film last changed: the version
//   an applied edit that touched it raised the case to. Every applied
//   edit changes the film it touches (see `editedFilm`), undo and redo
//   included; `open-case` changes every film kind.
// - An edit that carries `basedOn` is an engine answer computed from that
//   case version. It is dropped unapplied in two cases, and only these:
//   - Its own film has changed since: the version at which its film last
//     changed is above `basedOn`. Edits to other films do not make it
//     stale.
//   - It holds a vertebra id that its film does not hold now and that is
//     below the case's `nextVertebraId`. The answer took its new ids from
//     `nextVertebraId` as it was at `basedOn`, so such an id has been
//     given out since, to another film or to marks that undo or redo can
//     bring back, and keeping it could put one id on two vertebrae.
//   So a late answer never overwrites newer work on its film, and an
//   answer that is not late is lost to work on another film only when
//   that work gave out a vertebra id the answer also uses.
// - An auto pass over several films is applied as one `replace-marks`
//   edit per film in `AutoPassResult.marks`, in that order, each with
//   cause `auto-pass` and `basedOn` set to `AutoPassResult.caseVersion`
//   (the version the pass started from). Each is kept or dropped on its
//   own; the kept ones are one action. The order matters: each film's new
//   ids lie above the previous film's, so one film's edit landing never
//   makes the ids of a later film's edit look given out.
// - `nextVertebraId` never goes down: after each applied edit other than
//   `open-case` (which takes the opened case's), it is the larger of its
//   value before the edit and one above the highest vertebra id in the
//   case, so ids are never reused.
// - Each edit except `open-case` touches exactly one film.
// - A list of edits may be applied as one action. Each is kept or dropped
//   by the rules above, in order, and the version goes up once per
//   applied edit. The kept edits are one undo step; a single edit is an
//   action of one (consecutive `move-mark` edits with the same `gesture`
//   join one action). Applying an action that keeps at least one edit
//   clears redo; one that keeps none changes nothing and adds no undo
//   step. Undo and redo keep one list for the case and act on the newest
//   action: one undo or redo restores every film that action touched and
//   raises the version once, and each of those films counts as changed
//   at that version. The store can also start the history afresh without
//   changing the case, as `open-case` does.
// - Applying an action reports which of its edits were kept and which
//   were dropped, so the screens show `twoFilms` and each warning only
//   for what landed, without guessing.
// - After every edit, each film's `landmarks.c7` is what `c7_of` (in
//   `engine/shapes/src/landmarks.rs`) works out from that film's
//   vertebrae: the mean of the C7 vertebra's four corners, source `auto`,
//   not pinned, inferred when any corner is, and `null` when no vertebra
//   is named C7. The reducer works it out again after every edit that
//   changes a film's vertebrae: a C7 corner moving (`move-mark`), the
//   names changing which vertebra is C7 (`confirm-l5`), the names
//   swapping sides after a facing change (`set-geometry`), and an engine
//   answer (`replace-marks`, a wand's split or merge included). The
//   reader refuses a case where it disagrees.

import type {
  BoundaryAction,
  CaseFileV1,
  Film,
  FilmGeometry,
  FilmKind,
  FilmMarks,
  PinTarget,
  Px,
  PxRect,
} from '@/shapes';

export type CaseEdit =
  /** Replace the whole case (a case file was opened, or a new case begun). History starts afresh. */
  | { kind: 'open-case'; case: CaseFileV1 }
  /** Add a film, or replace the film of that kind; replacing drops that film's marks. */
  | { kind: 'set-film'; film: Film }
  /** Remove a film and its marks. */
  | { kind: 'remove-film'; film: FilmKind }
  /**
   * Scale, quarter turns, facing and what the user has confirmed, all
   * replaced by `geometry`. What happens to the film's marks depends on
   * what changed, judged against the film's geometry before the edit:
   * - `quarterTurns` changed: the film's marks are dropped. The upright
   *   direction decides which endplate is upper and the order of the
   *   levels, so the names can no longer be trusted. This holds whatever
   *   else changed. The screens do not start the auto pass themselves:
   *   the film shows as not yet searched until the user runs it, so no
   *   pass replaces work on other films unasked, and undoing the turn
   *   brings the marks straight back.
   * - `quarterTurns` unchanged and `imageRight` changed to a facing (not
   *   to `null`): the marks become `renamedFor(marks, imageRight)`, the
   *   TypeScript mirror of `FilmMarks::renamed_for` in
   *   `engine/shapes/src/vertebra.rs`, which returns them unchanged when
   *   their `namedFor` is already that facing. Otherwise every
   *   left/right (on a side film, anterior/posterior) pair swaps: each
   *   vertebra's corners, a front film's pedicles (each with its seen
   *   state and ring score), the two ends of each of a bending film's
   *   endplate lines, the sacrum top's two ends and the `left` and
   *   `right` hip centres. Each point keeps its position and its flags
   *   (source, pinned, inferred); only which name holds it changes.
   *   `namedFor` becomes the new `imageRight`, and `c7`, the mean of the
   *   same four corners, is worked out again and lands on the same point.
   *   The mirror must reproduce `proving-ground/known-answers/side-swap.json`.
   *   It is mirrored rather than called because this reducer is pure and
   *   synchronous on the main thread, where no engine is loaded.
   * - Anything else (only the scale or the confirmations changed, or
   *   `imageRight` changed to `null`): the marks are untouched, even when
   *   their `namedFor` differs from `imageRight`. So confirming the facing
   *   as it stands never renames, and the marks of a file written by
   *   something other than this app keep their names until the facing
   *   changes (the case check `marks-named-for-other-facing` reports
   *   them meanwhile).
   * Undoing it brings back the marks as they were together with the old
   * geometry.
   */
  | { kind: 'set-geometry'; film: FilmKind; geometry: FilmGeometry }
  /**
   * The user flips the film's brightness: toggles `values.inverted` and
   * `values.invertedByUser`. The film's desk keeps its pixels as opened;
   * the jobs that take the record (`encode-png`, `film-pass`) apply the
   * difference.
   */
  | { kind: 'flip-brightness'; film: FilmKind }
  /** The boxes blanked out because they show names or IDs. */
  | { kind: 'set-blanked'; film: FilmKind; boxes: PxRect[] }
  /**
   * An engine answer: one film's marks, whole. Dropped if its film
   * changed since `basedOn`, or if it uses a vertebra id the case has
   * given out since (see the rules above).
   */
  | { kind: 'replace-marks'; marks: FilmMarks; basedOn: number; cause: MarksCause }
  /**
   * Put one point exactly where the user says, without the engine: its
   * source becomes `hand`, its pinned flag stays, and it is no longer
   * inferred, since the user placed it on what they see. On a bending film
   * the point is an end of an endplate line (`PinTarget` `endplate`).
   * Moving a C7 corner works out `landmarks.c7` again, its `inferred`
   * included (see the rules above). Consecutive moves with the same
   * `gesture` are one undo step (a drag); `null` stands alone.
   */
  | { kind: 'move-mark'; film: FilmKind; target: PinTarget; to: Px; gesture: number | null }
  /** Pin or unpin one point where it is. */
  | { kind: 'pin-mark'; film: FilmKind; target: PinTarget; pinned: boolean }
  /** The user says a pedicle cannot be seen. Moving it again places it. */
  | { kind: 'hide-pedicle'; film: FilmKind; vertebra: number; mark: 'pedL' | 'pedR' }
  /**
   * The user says which vertebra is L5. It becomes L5, and the vertebrae
   * above and below take the names before and after it in
   * `TEMPLATE_NAMES[naming.template]`; `non-standard-count` is flagged if
   * any name falls outside `TEMPLATE_LEVELS[naming.template]`, and
   * `naming.l5ConfirmedOn` becomes the vertebra's id. Offered only when
   * every vertebra then has a name. The renaming can move C7 to another
   * vertebra or off the film, so `landmarks.c7` is worked out again (see
   * the rules above). Each film is confirmed on its own, as
   * each film's chain is its own; the case checks report films that
   * disagree.
   */
  | { kind: 'confirm-l5'; film: FilmKind; vertebra: number };

/** What produced a `replace-marks` edit; the undo label is made from it ("Undo: split T8"). */
export type MarksCause =
  | { by: 'auto-pass' }
  | { by: 'region-wand'; vertebra: number }
  | { by: 'boundary-wand'; action: BoundaryAction; vertebra: number }
  | { by: 'pin'; target: PinTarget }
  | { by: 'unpin'; target: PinTarget };

export type CaseEditKind = CaseEdit['kind'];

/** The film an edit touches, or `null` for one that replaces the whole case. */
export function editedFilm(edit: CaseEdit): FilmKind | null {
  switch (edit.kind) {
    case 'open-case':
      return null;
    case 'set-film':
      return edit.film.kind;
    case 'replace-marks':
      return edit.marks.film;
    case 'remove-film':
    case 'set-geometry':
    case 'flip-brightness':
    case 'set-blanked':
    case 'move-mark':
    case 'pin-mark':
    case 'hide-pedicle':
    case 'confirm-l5':
      return edit.film;
  }
}
