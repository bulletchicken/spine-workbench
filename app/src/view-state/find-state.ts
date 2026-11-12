import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';
import type { ErrorCode, Level } from '@/shapes';

export interface FindState {
  phase: 'idle' | 'searching' | 'replaying' | 'found' | 'failed';
  share: number;
  searched: number;
  failure: ErrorCode | null;
  reviewing: readonly Level[];
  review: number | null;
  looked: readonly Level[];
  holding: boolean;
  searching(films: number): void;
  progress(share: number): void;
  replaying(): void;
  found(): void;
  failed(code: ErrorCode): void;
  openReview(levels: readonly Level[]): void;
  lookedAt(level: Level): void;
  look(at: number | null): void;
  /** Leaving Find: the search's own state goes, and the looks stay while its outlines stand. */
  rest(): void;
  clear(): void;
}

const EMPTY = { phase: 'idle', share: 0, searched: 0, failure: null, reviewing: [], review: null, looked: [], holding: false } as const;

/** A look belongs to this visit, and never changes the case's marks. */
export function createFindState() {
  return createStore<FindState>()((set, get) => ({
    ...EMPTY,
    searching: (searched) => { set({ ...EMPTY, phase: 'searching', searched }); },
    progress: (share) => {
      if (get().phase === 'searching' && Number.isFinite(share)) set({ share: Math.max(get().share, Math.min(1, share)) });
    },
    replaying: () => { set({ phase: 'replaying' }); },
    found: () => { set({ phase: 'found', failure: null }); },
    failed: (failure) => { set({ phase: 'failed', failure, reviewing: [], review: null, holding: false }); },
    openReview: (levels) => { set({ reviewing: [...levels], review: levels.length === 0 ? null : 0, holding: false }); },
    lookedAt: (level) => { set({ looked: get().looked.includes(level) ? get().looked : [...get().looked, level], holding: true }); },
    look: (at) => { set({ review: at !== null && get().reviewing[at] !== undefined ? at : null, holding: false }); },
    rest: () => { set({ ...EMPTY, looked: get().phase === 'found' ? get().looked : [] }); },
    clear: () => { set(EMPTY); },
  }));
}

export const findState = createFindState();
export function useFindState(): FindState { return useStore(findState); }
