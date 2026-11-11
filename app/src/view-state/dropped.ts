// Files waiting to be taken: films to hang on the bench, and a case file to
// open. Films come with their kind when it is known (a made-up spine's
// films have one; a dropped file does not). Each is held as the blob it was
// given, not read into memory, and is read in full only when its film is
// opened. Each is taken once, and the taking empties it. Never saved with
// the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmKind } from '@/shapes';

/** A film waiting to be hung: the file it was given as, and its kind when it is known. */
export interface WaitingFilm {
  kind: FilmKind | null;
  blob: Blob;
}

export interface DroppedState {
  films: WaitingFilm[];
  /** A case file waiting to be opened, or `null`. */
  caseFile: Blob | null;
  /** Add films to the ones waiting, in the order given. */
  dropFilms(films: WaitingFilm[]): void;
  /** Replace the films waiting with these, and let go of the ones before. */
  replaceFilms(films: WaitingFilm[]): void;
  /** Take every film waiting, and leave none. */
  takeFilms(): WaitingFilm[];
  /** Leave a case file to open; it replaces one already waiting. */
  dropCaseFile(file: Blob): void;
  /** Take the case file waiting, and leave none. */
  takeCaseFile(): Blob | null;
}

export type DroppedApi = StoreApi<DroppedState>;

/** A store with nothing waiting. The app has one, `dropped`; tests make their own. */
export function createDropped(): DroppedApi {
  return createStore<DroppedState>()((set, get) => ({
    films: [],
    caseFile: null,
    dropFilms: (films) => {
      set((state) => ({ films: [...state.films, ...films] }));
    },
    replaceFilms: (films) => {
      set({ films });
    },
    takeFilms: () => {
      const taken = get().films;
      if (taken.length > 0) set({ films: [] });
      return taken;
    },
    dropCaseFile: (file) => {
      set({ caseFile: file });
    },
    takeCaseFile: () => {
      const taken = get().caseFile;
      if (taken !== null) set({ caseFile: null });
      return taken;
    },
  }));
}

/** The app's waiting files. */
export const dropped: DroppedApi = createDropped();

/** The waiting files, in a component. */
export function useDropped(): DroppedState {
  return useStore(dropped);
}
