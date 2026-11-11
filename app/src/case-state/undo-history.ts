// The case's one undo list. Each entry is one action: the films it
// touched as they were before it and after it, and what it was. Undo and
// redo act on the newest action, whichever films it touched.

import type { CaseFileV1, Film, FilmKind, FilmMarks } from '@/shapes';
import type { ActionRecord } from './action-record';
import { filmOf, marksOf } from './selectors';

/** At most this many actions can be undone; the oldest goes first. */
export const HISTORY_LIMIT = 1000;

/** One film slot as it stands: its record and its marks, each absent or not. */
export interface FilmState {
  film: Film | null;
  marks: FilmMarks | null;
}

/** Some film slots as they stood. */
export type FilmStates = Partial<Record<FilmKind, FilmState>>;

export interface HistoryEntry {
  record: ActionRecord;
  before: FilmStates;
  after: FilmStates;
  /** The drag this action was, so that the next move of the same drag joins it; `null` for any other action. */
  gesture: number | null;
}

export interface History {
  /** Oldest first; the last is undone next. */
  undo: readonly HistoryEntry[];
  /** Oldest first; the last is redone next. */
  redo: readonly HistoryEntry[];
  /** The gesture of the newest action, while nothing has been undone, redone or started afresh since. */
  joinable: number | null;
}

export const NO_HISTORY: History = { undo: [], redo: [], joinable: null };

/** A film slot as the case holds it now. */
export function filmState(c: CaseFileV1, kind: FilmKind): FilmState {
  return { film: filmOf(c, kind), marks: marksOf(c, kind) };
}

/**
 * The history with a kept action added. A move of the drag the newest
 * action was joins that action: it keeps its record and its films as they
 * were before the drag began, and takes the films as they are now.
 * Anything else becomes the newest action. Either way redo is cleared.
 */
export function withAction(history: History, entry: HistoryEntry): History {
  const newest = history.undo.at(-1);
  if (newest !== undefined && entry.gesture !== null && entry.gesture === history.joinable) {
    const joined: HistoryEntry = {
      ...newest,
      before: { ...entry.before, ...newest.before },
      after: { ...newest.after, ...entry.after },
    };
    return { undo: [...history.undo.slice(0, -1), joined], redo: [], joinable: entry.gesture };
  }
  return { undo: [...history.undo, entry].slice(-HISTORY_LIMIT), redo: [], joinable: entry.gesture };
}

/** The newest action to undo, and the history after undoing it; `null` when there is none. */
export function takeUndo(history: History): { entry: HistoryEntry; history: History } | null {
  const entry = history.undo.at(-1);
  if (entry === undefined) return null;
  return { entry, history: { undo: history.undo.slice(0, -1), redo: [...history.redo, entry], joinable: null } };
}

/** The newest undone action to redo, and the history after redoing it; `null` when there is none. */
export function takeRedo(history: History): { entry: HistoryEntry; history: History } | null {
  const entry = history.redo.at(-1);
  if (entry === undefined) return null;
  return { entry, history: { undo: [...history.undo, entry], redo: history.redo.slice(0, -1), joinable: null } };
}

/** What Undo would undo now, for its tooltip ("Undo: Split T12"); `null` when nothing. */
export function nextUndo(history: History): ActionRecord | null {
  return history.undo.at(-1)?.record ?? null;
}

/** What Redo would redo now; `null` when nothing. */
export function nextRedo(history: History): ActionRecord | null {
  return history.redo.at(-1)?.record ?? null;
}
