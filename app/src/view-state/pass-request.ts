// "Find the spine" was pressed and the pass has not yet been started. The
// pass step takes the request once; a request is never repeated by itself.
// Never saved with the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';

export interface PassRequestState {
  /** A request is waiting to be taken. */
  requested: boolean;
  /** Leave a request for the pass step to take. */
  request(): void;
  /** Take the waiting request: `true` once, then `false` until the next request. */
  take(): boolean;
}

export type PassRequestApi = StoreApi<PassRequestState>;

/** A store with no request waiting. The app has one, `passRequest`; tests make their own. */
export function createPassRequest(): PassRequestApi {
  return createStore<PassRequestState>()((set, get) => ({
    requested: false,
    request: () => {
      set({ requested: true });
    },
    take: () => {
      const taken = get().requested;
      if (taken) set({ requested: false });
      return taken;
    },
  }));
}

/** The app's pass request. */
export const passRequest: PassRequestApi = createPassRequest();

/** The pass request, in a component. */
export function usePassRequest(): PassRequestState {
  return useStore(passRequest);
}
