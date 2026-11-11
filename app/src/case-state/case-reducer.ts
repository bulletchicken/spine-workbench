// The one pure reducer the case store runs, keeping the rules in the
// header of case-edits.ts: actions of several edits, each kept or dropped
// in order; a version raised once per applied edit, undo and redo
// included, and never restored; staleness judged per film; one undo list
// for the case; the region wand's clicks per vertebra.
//
// Every function here takes a state and gives back a new one; none
// changes what it is given.

import type { CaseFileV1, FilmKind, WandClick } from '@/shapes';
import { recordOf, type ActionRecord, type CalledAction } from './action-record';
import { applyEdit, withFilm, withMarks } from './apply-edit';
import { editedFilm, type CaseEdit } from './case-edits';
import { aboveEveryId, FILM_KINDS, vertebraOf } from './selectors';
import { staleness, type Stale } from './staleness';
import { filmState, NO_HISTORY, takeRedo, takeUndo, withAction, type FilmStates, type History } from './undo-history';
import { afterChange, answeredClicks, clearedSessions, NO_WAND_SESSIONS, vertebraHeight, withClick, withoutSent, withSent, type WandSessions } from './wand-sessions';

export interface CaseState {
  case: CaseFileV1;
  /** Up by one on every applied edit, undo and redo included; never restored. Not `CASE_VERSION`. */
  version: number;
  /** For each film kind, the version at which that film last changed. */
  changedAt: Readonly<Record<FilmKind, number>>;
  history: History;
  wand: WandSessions;
}

/** Why an edit was dropped unapplied: stale, or naming something the case does not hold. */
export type Dropped = Stale | 'not-in-case';

/** What became of one edit of an action. */
export type EditOutcome = { kept: true } | { kept: false; why: Dropped };

export interface ActionReport {
  /** One per edit, in the order given. */
  outcomes: EditOutcome[];
  /** What the action was, when it kept at least one edit; `null` when it kept none and changed nothing. */
  record: ActionRecord | null;
}

export interface ActionOptions {
  /** Record the action under this name rather than its first edit's kind. */
  as?: CalledAction;
}

/** A store holding `c`, at version 0, with no history. */
export function stateOf(c: CaseFileV1): CaseState {
  return { case: c, version: 0, changedAt: allAt(0), history: NO_HISTORY, wand: NO_WAND_SESSIONS };
}

/**
 * Apply a list of edits as one action. Each is kept or dropped in order,
 * against the case the edits before it left; the version goes up once per
 * kept edit. The kept edits become one undo step (a move of the drag the
 * newest action was joins it) and clear redo. An action that keeps none
 * gives back the very same state. An `open-case` starts the history
 * afresh where it stands, so only the edits after it can be undone.
 */
export function applyAction(state: CaseState, edits: readonly CaseEdit[], options: ActionOptions = {}): { state: CaseState; report: ActionReport } {
  let { case: c, version, changedAt, history, wand } = state;
  const outcomes: EditOutcome[] = [];
  let kept: CaseEdit[] = [];
  let before: FilmStates = {};
  let start = c;

  for (const edit of edits) {
    const stale = staleness(c, changedAt, edit);
    const next = stale === null ? applyEdit(c, edit) : null;
    if (next === null) {
      outcomes.push({ kept: false, why: stale ?? 'not-in-case' });
      continue;
    }
    version += 1;
    const film = editedFilm(edit);
    if (film === null) {
      changedAt = allAt(version);
      history = NO_HISTORY;
      wand = clearedSessions(wand);
      kept = [];
      before = {};
      start = next;
    } else {
      changedAt = { ...changedAt, [film]: version };
      before = { [film]: filmState(c, film), ...before };
      wand = afterChange(wand, c, next, landedWand(wand, edit));
      kept.push(edit);
    }
    c = next;
    outcomes.push({ kept: true });
  }

  if (version === state.version) return { state, report: { outcomes, record: null } };
  if (kept.length === 0) {
    // Only an opened case landed: it starts the history, it is not a step in it.
    const opened = recordOf(state.case, edits.filter((edit) => edit.kind === 'open-case'), options.as ?? null);
    return { state: { case: c, version, changedAt, history, wand }, report: { outcomes, record: opened } };
  }

  const record = recordOf(start, kept, options.as ?? null);
  const after: FilmStates = {};
  for (const film of Object.keys(before) as FilmKind[]) after[film] = filmState(c, film);
  history = withAction(history, { record, before, after, gesture: dragOf(kept) });
  return { state: { case: c, version, changedAt, history, wand }, report: { outcomes, record } };
}

/** Undo the newest action: every film it touched as it was before, the version up by one. `record` is `null` when there was nothing to undo. */
export function undo(state: CaseState): { state: CaseState; record: ActionRecord | null } {
  const taken = takeUndo(state.history);
  if (taken === null) return { state, record: null };
  return { state: restored(state, taken.entry.before, taken.history), record: taken.entry.record };
}

/** Redo the newest undone action: every film it touched as it was after, the version up by one. */
export function redo(state: CaseState): { state: CaseState; record: ActionRecord | null } {
  const taken = takeRedo(state.history);
  if (taken === null) return { state, record: null };
  return { state: restored(state, taken.entry.after, taken.history), record: taken.entry.record };
}

/** Start the history afresh without changing the case: nothing to undo or redo. */
export function startAfresh(state: CaseState): CaseState {
  return { ...state, history: NO_HISTORY };
}

/**
 * The clicks to hand the engine with a new region wand click on a
 * vertebra: its session with `click` added, and the request's number.
 * They become its session if the answer, asked at this state's version,
 * lands; a click made before an earlier answer is in waits behind it.
 * `null` when the film does not hold that vertebra.
 */
export function wandClicksWith(
  state: CaseState,
  film: FilmKind,
  vertebra: number,
  click: WandClick,
): { state: CaseState; clicks: WandClick[]; ask: number } | null {
  const target = vertebraOf(state.case, film, vertebra);
  if (target === null) return null;
  const clicks = withClick(state.wand.clicks[vertebra] ?? [], click, vertebraHeight(target.marks));
  const { sessions, ask } = withSent(state.wand, vertebra, { askedAt: state.version, clicks }, state.changedAt[film]);
  return { state: { ...state, wand: sessions }, clicks, ask };
}

/**
 * Region wand request `ask` ended with no answer to apply: the engine
 * refused it or the tool gave it up. It stops waiting, so a later answer
 * asked at the same version is not taken for its answer. The case and
 * its version do not change; the very same state when it was not waiting.
 */
export function wandUnanswered(state: CaseState, ask: number): CaseState {
  const wand = withoutSent(state.wand, ask);
  return wand === state.wand ? state : { ...state, wand };
}

function restored(state: CaseState, films: FilmStates, history: History): CaseState {
  const version = state.version + 1;
  let c = state.case;
  let changedAt = state.changedAt;
  for (const [film, { film: record, marks }] of Object.entries(films) as [FilmKind, NonNullable<FilmStates[FilmKind]>][]) {
    c = withMarks(withFilm(c, film, record), film, marks);
    changedAt = { ...changedAt, [film]: version };
  }
  c = { ...c, nextVertebraId: Math.max(c.nextVertebraId, aboveEveryId(c)) };
  return { case: c, version, changedAt, history, wand: afterChange(state.wand, state.case, c, null) };
}

/** When this edit is a vertebra's own region wand answer: that vertebra, and the clicks it was asked with. */
function landedWand(wand: WandSessions, edit: CaseEdit): { vertebra: number; clicks: readonly WandClick[] | null } | null {
  if (edit.kind !== 'replace-marks' || edit.cause.by !== 'region-wand') return null;
  const vertebra = edit.cause.vertebra;
  return { vertebra, clicks: answeredClicks(wand, vertebra, edit.basedOn) };
}

/** The drag an action is: the gesture every edit shares when all are moves of one drag; otherwise `null`. */
function dragOf(edits: readonly CaseEdit[]): number | null {
  const [first] = edits;
  if (first?.kind !== 'move-mark' || first.gesture === null) return null;
  const gesture = first.gesture;
  return edits.every((edit) => edit.kind === 'move-mark' && edit.gesture === gesture) ? gesture : null;
}

function allAt(version: number): Record<FilmKind, number> {
  return Object.fromEntries(FILM_KINDS.map((film) => [film, version])) as Record<FilmKind, number>;
}
