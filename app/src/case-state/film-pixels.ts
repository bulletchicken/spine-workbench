// Each film's pixels for the screen and its thumbnail, kept by film kind
// outside the case store: they are large, they are never saved, and no
// edit or undo touches them. `open-and-save/open-a-film.ts` keeps them
// when a film opens; the bench and the case sheet read them.
//
// With them, how many times a film has been opened in each slot. A film's
// record holds its size, not a fingerprint, so this count is how steps 3
// and 4 tell that a recording was made from a film since replaced.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmKind } from '@/shapes';

/** One opened film, ready for the screen. */
export interface FilmPixels {
  /** Half-float bits, the film's width × height (`EngineClient.displayPixels`). */
  pixels: Uint16Array;
  /** An 8-bit greyscale PNG for the film slots (`OpenedFilm.thumbnail`). */
  thumbnail: Uint8Array;
}

export interface FilmPixelsStore {
  films: Partial<Record<FilmKind, FilmPixels>>;
  /** How many times a film has been opened in each slot this session; never goes down. */
  openings: Record<FilmKind, number>;
  /** A film was opened in this slot: keep its pixels and count the opening. */
  opened(kind: FilmKind, film: FilmPixels): void;
  /** The film in this slot was retired: let its pixels go. Its count stays. */
  retired(kind: FilmKind): void;
}

export type FilmPixelsApi = StoreApi<FilmPixelsStore>;

/** A store with no film. The app has one, `filmPixels`; tests make their own. */
export function createFilmPixels(): FilmPixelsApi {
  return createStore<FilmPixelsStore>()((set) => ({
    films: {},
    openings: { front: 0, side: 0, 'bend-left': 0, 'bend-right': 0 },
    opened: (kind, film) => {
      set((state) => ({ films: { ...state.films, [kind]: film }, openings: { ...state.openings, [kind]: state.openings[kind] + 1 } }));
    },
    retired: (kind) => {
      set((state) => {
        if (state.films[kind] === undefined) return state;
        const films = Object.fromEntries(Object.entries(state.films).filter(([slot]) => slot !== kind));
        return { films };
      });
    },
  }));
}

/** The app's opened films. */
export const filmPixels: FilmPixelsApi = createFilmPixels();

/** A film's pixels and thumbnail in a component, or `null` while its slot holds none. */
export function useFilmPixels(kind: FilmKind): FilmPixels | null {
  return useStore(filmPixels, (state) => state.films[kind] ?? null);
}

/** How many times a film has been opened in this slot, in a component. */
export function useOpenings(kind: FilmKind): number {
  return useStore(filmPixels, (state) => state.openings[kind]);
}
