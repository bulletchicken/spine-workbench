// Whether the promise's popover is open. A store, so that a dev scene can
// open it as a press on the badge does. Never saved.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';

export interface PopoverOpenState {
  open: boolean;
  setOpen(open: boolean): void;
}

/** The app's one promise popover. */
export const popoverOpen: StoreApi<PopoverOpenState> = createStore<PopoverOpenState>()((set) => ({
  open: false,
  setOpen: (open) => {
    set({ open });
  },
}));

/** The popover flag, in a component. */
export function usePopoverOpen(): PopoverOpenState {
  return useStore(popoverOpen);
}
