import { createStore } from 'zustand/vanilla';
import type { ReplaySignal } from '@/marks/TravellingLight';
import { lightShareAt, replayEnds, shownAt, type ReplayTimes, type RevealPlan } from './find-replay';

interface ReplayState {
  plan: RevealPlan | null;
  times: ReplayTimes;
  time: number;
  playing: boolean;
  revealed: number;
  allShown: boolean;
  start(plan: RevealPlan, times: ReplayTimes): void;
  advance(ms: number): void;
  pause(): void;
  clear(): void;
}

const EMPTY = { plan: null, times: { lead: 0, travel: 0, hold: 0 }, time: 0, playing: false, revealed: 0, allShown: false };

export function createFindReplay() {
  const clock = createStore<ReplayState>()((set, get) => ({
    ...EMPTY,
    start: (plan, times) => { set({ ...EMPTY, plan, times, playing: true }); },
    advance: (ms) => {
      const { playing, plan, times, time: was } = get();
      if (!playing || plan === null || !Number.isFinite(ms) || ms < 0) return;
      const end = replayEnds(plan, times);
      const time = Math.min(end, was + ms);
      const { shown, allShown } = shownAt(plan, times, time);
      set({ time, playing: time < end, revealed: shown.size, allShown });
    },
    pause: () => { set({ playing: false }); },
    clear: () => { set(EMPTY); },
  }));
  const signal: ReplaySignal = {
    share: () => { const { plan, times, time } = clock.getState(); return plan === null ? null : lightShareAt(times, time); },
    subscribe: (listener) => clock.subscribe(listener),
  };
  return Object.assign(clock, { signal });
}

export type FindReplayClock = ReturnType<typeof createFindReplay>;
export const findReplay = createFindReplay();
