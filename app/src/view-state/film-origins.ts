import { createStore } from 'zustand/vanilla';
import type { FilmKind } from '@/shapes';

export type FilmOrigin = 'yours' | 'sample-film' | 'made-up-case';
export type FilmOrigins = Partial<Record<FilmKind, { from: FilmOrigin; opening: number }>>;

/** The source belongs to an opening, never to the last film added. */
export const filmOrigins = createStore<{
  films: FilmOrigins;
  opened(kind: FilmKind, from: FilmOrigin, opening: number): void;
  forget(kind: FilmKind): void;
  clear(): void;
}>()((set) => ({
  films: {},
  opened: (kind, from, opening) => { set((s) => ({ films: { ...s.films, [kind]: { from, opening } } })); },
  forget: (kind) => { set((s) => ({ films: Object.fromEntries(Object.entries(s.films).filter(([k]) => k !== kind)) })); },
  clear: () => { set({ films: {} }); },
}));
