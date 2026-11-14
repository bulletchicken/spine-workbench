import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';
import type { ErrorCode, FilmKind } from '@/shapes';

export type IntakeNotice =
  | { kind: 'refused'; count: number; code: ErrorCode | 'a-saved-case' }
  | { kind: 'left-out'; count: number }
  | { kind: 'search-stopped' };

export interface FilmIntakeState {
  from: 'yours' | 'sample-film' | 'made-up-case' | 'real-case' | null;
  sample: number | null;
  nextSample: number;
  reading: readonly FilmKind[];
  readDone: number;
  readTotal: number;
  making: boolean;
  notice: IntakeNotice | null;
  leaving: boolean;
  source(from: FilmIntakeState['from'], sample?: number | null): void;
  next(nextSample: number): void;
  begin(places: readonly FilmKind[]): void;
  read(kind: FilmKind, kept?: boolean): void;
  make(making: boolean): void;
  tell(notice: IntakeNotice | null): void;
  leave(leaving: boolean): void;
  clear(): void;
}

const EMPTY = { from: null, sample: null, nextSample: 0, reading: [], readDone: 0, readTotal: 0, making: false, notice: null, leaving: false } as const;

/** The sample round lasts for the visit, even when the stage is emptied. */
export function createFilmIntake() {
  return createStore<FilmIntakeState>()((set) => ({
    ...EMPTY,
    source: (from, sample = null) => { set({ from, sample }); },
    next: (nextSample) => { set({ nextSample }); },
    begin: (reading) => { set({ reading, readDone: 0, readTotal: reading.length }); },
    read: (kind, kept = true) => { set((state) => state.reading.includes(kind) ? { reading: state.reading.filter((place) => place !== kind), readDone: state.readDone + (kept ? 1 : 0) } : state); },
    make: (making) => { set({ making }); },
    tell: (notice) => { set({ notice }); },
    leave: (leaving) => { set({ leaving }); },
    clear: () => { set((state) => ({ ...EMPTY, nextSample: state.nextSample })); },
  }));
}

export const filmIntake = createFilmIntake();
export function useFilmIntakeState(): FilmIntakeState { return useStore(filmIntake); }
