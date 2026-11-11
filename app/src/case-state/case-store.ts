// The open case, as one store the screens read and change. It holds what
// gets saved (`case`), its version, which film changed when, the one undo
// list and the region wand's clicks, and it changes only through the
// reducer in case-reducer.ts.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { CaseFileV1, FilmKind, WandClick } from '@/shapes';
import type { ActionRecord } from './action-record';
import type { CaseEdit } from './case-edits';
import * as reducer from './case-reducer';
import { emptyCase } from './empty-case';

export interface CaseStore extends reducer.CaseState {
  /**
   * Apply a list of edits as one action (a single edit is an action of
   * one) and say which were kept and which dropped. An action that keeps
   * none changes nothing and tells no one. A `move-mark` lands where its
   * `to` says, so the tool keeps `to` on the film (see `movePoint`).
   */
  apply(edits: readonly CaseEdit[], options?: reducer.ActionOptions): reducer.ActionReport;
  /** Undo the newest action; what it was, or `null` when there was nothing to undo. */
  undo(): ActionRecord | null;
  /** Redo the newest undone action; what it was, or `null` when there was nothing to redo. */
  redo(): ActionRecord | null;
  /** Nothing to undo or redo from here on; the case stays as it is. */
  startAfresh(): void;
  /**
   * The clicks to hand the engine with a new region wand click on a
   * vertebra (its clicks so far with this one added), and the request's
   * number. They become its session if the answer lands. `null` when the
   * film does not hold that vertebra.
   *
   * A click made before an earlier answer is in waits behind it, but if
   * the earlier answer is kept, its film has changed, so the later
   * answer is dropped as `film-changed` and the later click does
   * nothing. The tool chooses whether to hold such a click until the
   * earlier answer is in, or to ask again after it.
   *
   * What the tool does with each request, which the engine answers in
   * the order asked:
   * - An answer comes back: `apply` it, `basedOn` the version the request
   *   was asked at. Kept or dropped, nothing more is needed.
   * - No answer comes back (the engine refused it, say with
   *   `tool-no-contrast`, `tool-did-not-converge` or `engine-crashed`, or
   *   the tool gave it up): call `wandUnanswered` with its number at
   *   once, before applying any later answer. Once a request is given
   *   up, its answer must never be applied, even if it arrives later:
   *   region wand jobs cannot be cancelled, and that answer would be
   *   matched to a retry asked at the same version, making the retry's
   *   clicks the session of an outline grown from the old ones.
   */
  wandClicksWith(film: FilmKind, vertebra: number, click: WandClick): { clicks: WandClick[]; ask: number } | null;
  /** Region wand request `ask` ended with no answer; it stops waiting. The case does not change. */
  wandUnanswered(ask: number): void;
}

export type CaseStoreApi = StoreApi<CaseStore>;

/** A store holding `initial` at version 0. The app has one, `caseStore`; tests make their own. */
export function createCaseStore(initial: CaseFileV1 = emptyCase()): CaseStoreApi {
  return createStore<CaseStore>()((set, get) => {
    /** Run one reducer step, and tell the store's readers only if it changed anything. */
    function step<T>(run: (state: reducer.CaseState) => { state: reducer.CaseState; result: T }): T {
      const before = get();
      const { state, result } = run(before);
      if (state !== before) set(state);
      return result;
    }
    return {
      ...reducer.stateOf(initial),
      apply: (edits, options) =>
        step((state) => {
          const { state: next, report } = reducer.applyAction(state, edits, options);
          return { state: next, result: report };
        }),
      undo: () =>
        step((state) => {
          const { state: next, record } = reducer.undo(state);
          return { state: next, result: record };
        }),
      redo: () =>
        step((state) => {
          const { state: next, record } = reducer.redo(state);
          return { state: next, result: record };
        }),
      startAfresh: () => {
        step((state) => ({ state: reducer.startAfresh(state), result: undefined }));
      },
      wandClicksWith: (film, vertebra, click) =>
        step((state) => {
          const asked = reducer.wandClicksWith(state, film, vertebra, click);
          return asked === null ? { state, result: null } : { state: asked.state, result: { clicks: asked.clicks, ask: asked.ask } };
        }),
      wandUnanswered: (ask) => {
        step((state) => ({ state: reducer.wandUnanswered(state, ask), result: undefined }));
      },
    };
  });
}

/** The app's open case. */
export const caseStore: CaseStoreApi = createCaseStore();

/** Read from the open case in a component; it renders again when what `select` picks changes. */
export function useCase<T>(select: (store: CaseStore) => T): T {
  return useStore(caseStore, select);
}
