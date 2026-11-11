// The selected vertebra and the hovered one, each with its film. Never
// saved with the case. Hovering changes only the hot one; selecting changes
// only the selected one.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmKind } from '@/shapes';

/** One vertebra on one film. */
export interface VertebraOnScreen {
  film: FilmKind;
  vertebra: number;
}

export interface SelectionState {
  /** The selected vertebra, or `null`. */
  selected: VertebraOnScreen | null;
  /** The vertebra under the pointer, or `null`. */
  hot: VertebraOnScreen | null;
  select(at: VertebraOnScreen | null): void;
  hover(at: VertebraOnScreen | null): void;
}

export type SelectionApi = StoreApi<SelectionState>;

/** A store with nothing selected or hovered. The app has one, `selection`; tests make their own. */
export function createSelection(): SelectionApi {
  return createStore<SelectionState>()((set) => ({
    selected: null,
    hot: null,
    select: (selected) => {
      set({ selected });
    },
    hover: (hot) => {
      set({ hot });
    },
  }));
}

/** The app's selection. */
export const selection: SelectionApi = createSelection();

/** The selection, in a component. */
export function useSelection(): SelectionState {
  return useStore(selection);
}
