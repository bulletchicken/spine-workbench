// The review's place for this session only: whether the walk is on, the keys
// of the items skipped, and how many items there were when it began (for
// "2 of 4"). It is never saved in the case and never survives a reload.
// Whatever puts a different case in place (Start over, opening a saved case,
// a made-up spine) calls `forget`, so one case's skips never hide another's
// items.

import { create } from 'zustand';

export interface ReviewWalkState {
  reviewing: boolean;
  countAtStart: number;
  skipped: readonly string[];
  start(count: number): void;
  leave(): void;
  skip(key: string): void;
  /** Back to no walk and nothing skipped, for a different case. */
  forget(): void;
}

export const useReviewWalk = create<ReviewWalkState>()((set) => ({
  reviewing: false,
  countAtStart: 0,
  skipped: [],
  start: (count) => {
    set({ reviewing: true, countAtStart: count });
  },
  leave: () => {
    set({ reviewing: false });
  },
  skip: (key) => {
    set((state) => (state.skipped.includes(key) ? state : { skipped: [...state.skipped, key] }));
  },
  forget: () => {
    set({ reviewing: false, countAtStart: 0, skipped: [] });
  },
}));
