import { createStore } from 'zustand/vanilla';
import type { FilmKind } from '@/shapes';

export type FilmOrigin = 'yours' | 'sample-film' | 'made-up-case' | 'real-case';
export type FilmOrigins = Partial<Record<FilmKind, { from: FilmOrigin; opening: number }>>;

/** The source belongs to an opening, never to the last film added. */
export const filmOrigins = createStore<{
  films: FilmOrigins;
  credit: string | null;
  setCredit(credit: string | null): void;
  opened(kind: FilmKind, from: FilmOrigin, opening: number): void;
  forget(kind: FilmKind): void;
  clear(): void;
}>()((set) => ({
  films: {},
  credit: null,
  setCredit: (credit) => { set({ credit }); },
  opened: (kind, from, opening) => { set((s) => ({ credit: null, films: { ...s.films, [kind]: { from, opening } } })); },
  forget: (kind) => { set((s) => ({ credit: null, films: Object.fromEntries(Object.entries(s.films).filter(([k]) => k !== kind)) })); },
  clear: () => { set({ films: {}, credit: null }); },
}));
