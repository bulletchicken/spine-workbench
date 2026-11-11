// Whether the case sheet is open. Set by the Save case button and by
// Cmd or Ctrl+S; the sheet reads it. Never saved with the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';

export interface CaseSheetOpenState {
  open: boolean;
  setOpen(open: boolean): void;
}

export type CaseSheetOpenApi = StoreApi<CaseSheetOpenState>;

/** A store with the sheet shut. The app has one, `caseSheetOpen`; tests make their own. */
export function createCaseSheetOpen(): CaseSheetOpenApi {
  return createStore<CaseSheetOpenState>()((set) => ({
    open: false,
    setOpen: (open) => {
      set({ open });
    },
  }));
}

/** The app's case sheet flag. */
export const caseSheetOpen: CaseSheetOpenApi = createCaseSheetOpen();

/** The sheet flag, in a component. */
export function useCaseSheetOpen(): CaseSheetOpenState {
  return useStore(caseSheetOpen);
}
