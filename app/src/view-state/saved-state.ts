// The case's name as shown, and the case version at the last save or open,
// with which of the two it was. Together they say what the top bar reads:
// not saved yet, saved just now, or changes not saved. Never saved with
// the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';

/** How the case last reached the disk or the screen. */
export type SavedHow = 'saved' | 'opened';

export interface SavedMark {
  /** The case version at the save or the open. */
  version: number;
  how: SavedHow;
}

/** Not saved yet, saved at the version now, or edited since the mark. */
export type SaveStatus = 'not-saved-yet' | 'up-to-date' | 'changes-not-saved';

/** The status of a case at `version`, given the mark it last had. */
export function saveStatus(mark: SavedMark | null, version: number): SaveStatus {
  if (mark === null) return 'not-saved-yet';
  return mark.version === version ? 'up-to-date' : 'changes-not-saved';
}

export interface SavedStateState {
  /** The name shown for the case, or `null` for Untitled case. */
  name: string | null;
  /** The last save or open, or `null` before either. */
  mark: SavedMark | null;
  setName(name: string | null): void;
  markSaved(version: number): void;
  markOpened(version: number): void;
}

export type SavedStateApi = StoreApi<SavedStateState>;

/** A store with no name and no mark. The app has one, `savedState`; tests make their own. */
export function createSavedState(): SavedStateApi {
  return createStore<SavedStateState>()((set) => ({
    name: null,
    mark: null,
    setName: (name) => {
      set({ name });
    },
    markSaved: (version) => {
      set({ mark: { version, how: 'saved' } });
    },
    markOpened: (version) => {
      set({ mark: { version, how: 'opened' } });
    },
  }));
}

/** The app's saved state. */
export const savedState: SavedStateApi = createSavedState();

/** The saved state, in a component. */
export function useSavedState(): SavedStateState {
  return useStore(savedState);
}
