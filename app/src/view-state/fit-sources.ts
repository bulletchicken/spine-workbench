import { createStore } from 'zustand/vanilla';
import type { FilmKind, FrontFinder } from '@/shapes';

/** A kept finding belongs to one opening, even after its marks are reviewed. */
export type FitSources = Partial<Record<FilmKind, { finder: FrontFinder; opening: number }>>;

interface FitSourcesState {
  films: FitSources;
  kept(film: FilmKind, finder: FrontFinder, opening: number): void;
  forget(film: FilmKind): void;
  clear(): void;
}

/** Visit-only provenance. Tests and other visits have independent stores. */
export function createFitSources() {
  return createStore<FitSourcesState>()((set) => ({
    films: {},
    kept: (film, finder, opening) => { set((state) => ({ films: { ...state.films, [film]: { finder, opening } } })); },
    forget: (film) => { set((state) => ({ films: Object.fromEntries(Object.entries(state.films).filter(([kind]) => kind !== film)) })); },
    clear: () => { set({ films: {} }); },
  }));
}

export const fitSources = createFitSources();
