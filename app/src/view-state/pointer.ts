// Where the pointer is: the film under it, the point in film pixels and on
// the screen. `null` when it is on no film. Never saved with the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmKind, Px } from '@/shapes';
import type { ScreenPoint } from '@/film-viewer/film-space';

export interface PointerReading {
  film: FilmKind;
  /** Original film pixels. */
  at: Px;
  screen: ScreenPoint;
}

export interface PointerState {
  /** Where the pointer is over a film, or `null` when it is over none. */
  reading: PointerReading | null;
  move(reading: PointerReading): void;
  leave(): void;
}

export type PointerApi = StoreApi<PointerState>;

/** A store with no pointer over a film. The app has one, `pointer`; tests make their own. */
export function createPointer(): PointerApi {
  return createStore<PointerState>()((set) => ({
    reading: null,
    move: (reading) => {
      set({ reading });
    },
    leave: () => {
      set({ reading: null });
    },
  }));
}

/** The app's pointer. */
export const pointer: PointerApi = createPointer();

/** The pointer, in a component. */
export function usePointer(): PointerState {
  return useStore(pointer);
}
