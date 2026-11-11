// The pass's clock: time on the pass's own timeline, whether it plays,
// and how fast. Whoever runs the animation loop calls `advance` with the
// milliseconds since the last frame; everything drawn reads `time` and
// asks the timeline what stands there.
//
// The clock never passes the end of the last beat whose data has
// arrived (`setReady`), so a slow engine holds the picture on that beat's
// last moment instead of playing ahead of what was found. Seeking is held
// there too. Skip goes as far as the data allows, and keeps going as more
// arrives, until it reaches the end.
//
// Under reduced motion there is no travelling light: playing steps
// through the end of each beat, then the end of the pass, 750 ms apart,
// whatever the speed.
//
// A time, a step or a count that is not a finite number moves nothing.

import { createStore, type StoreApi } from 'zustand/vanilla';
import { BEATS, PASS_END } from './beats';

export type Speed = 0.5 | 1 | 2;

/** Under reduced motion, the pictures the pass steps through. */
export const STILLS: readonly number[] = [...BEATS.map((beat) => beat.end), PASS_END];
/** Under reduced motion, how long each still stays. */
export const STILL_LASTS = 750;

export interface ClockState {
  /** Milliseconds on the pass's own timeline, 0 to PASS_END. */
  time: number;
  playing: boolean;
  speed: Speed;
  /** How many beats, from the first, have all their data. */
  readyBeats: number;
  reduced: boolean;
  /** Skip was pressed before the data reached the end. */
  skipping: boolean;
  /** Under reduced motion, how long the current still has been shown. */
  sinceStill: number;
}

export interface ClockActions {
  /** Plays on; from the start again when at the end. */
  play(): void;
  pause(): void;
  toggle(): void;
  /** Plays from the start. */
  replay(): void;
  /** Pauses and goes to time t, as far as the data allows. */
  seek(t: number): void;
  /** Goes to the end, as soon as the data allows. */
  skip(): void;
  setSpeed(speed: Speed): void;
  /** How many beats now have all their data. */
  setReady(beats: number): void;
  setReduced(reduced: boolean): void;
  /** Moves the clock on by `ms` of real time. */
  advance(ms: number): void;
}

export type PassClock = StoreApi<ClockState & ClockActions>;

/** The furthest the clock may go with this many beats ready. */
export function holdPoint(readyBeats: number): number {
  if (readyBeats >= BEATS.length) return PASS_END;
  if (readyBeats <= 0) return BEATS[0]?.start ?? 0;
  return BEATS[readyBeats - 1]?.end ?? 0;
}

/** Playing, but waiting for data before it can go on. */
export function isHeld(state: ClockState): boolean {
  if (!state.playing || state.time >= PASS_END) return false;
  const limit = holdPoint(state.readyBeats);
  return state.reduced ? (nextStill(state.time) ?? PASS_END) > limit : state.time >= limit;
}

export function createPassClock(start: Partial<Pick<ClockState, 'readyBeats' | 'reduced' | 'speed'>> = {}): PassClock {
  return createStore<ClockState & ClockActions>()((set) => ({
    time: 0,
    playing: false,
    speed: 1,
    readyBeats: 0,
    reduced: false,
    skipping: false,
    sinceStill: 0,
    ...start,
    play: () => {
      set(playing);
    },
    pause: () => {
      set({ playing: false });
    },
    toggle: () => {
      set((state) => (state.playing ? { playing: false } : playing(state)));
    },
    replay: () => {
      set((state) => playing({ ...state, time: 0 }));
    },
    seek: (t) => {
      set((state) => ({ time: clamp(Number.isFinite(t) ? t : state.time, 0, holdPoint(state.readyBeats)), playing: false, skipping: false, sinceStill: 0 }));
    },
    skip: () => {
      set((state) => skipped(holdPoint(state.readyBeats)));
    },
    setSpeed: (speed) => {
      set({ speed });
    },
    setReady: (beats) => {
      set((state) => {
        if (!Number.isFinite(beats)) return {};
        const readyBeats = clamp(Math.floor(beats), 0, BEATS.length);
        const limit = holdPoint(readyBeats);
        if (state.skipping) return { readyBeats, ...skipped(limit) };
        return { readyBeats, time: Math.min(state.time, limit) };
      });
    },
    setReduced: (reduced) => {
      set({ reduced, sinceStill: 0 });
    },
    advance: (ms) => {
      set((state) => advanced(state, ms));
    },
  }));
}

function playing(state: ClockState): Partial<ClockState> {
  const from = state.time >= PASS_END ? 0 : state.time;
  // Under reduced motion the next still shows at once.
  const time = state.reduced ? stepToNextStill(from, state.readyBeats) : from;
  return { time, playing: time < PASS_END, skipping: false, sinceStill: 0 };
}

function skipped(limit: number): Partial<ClockState> {
  return limit >= PASS_END ? { time: PASS_END, playing: false, skipping: false } : { time: limit, playing: false, skipping: true };
}

function advanced(state: ClockState, ms: number): Partial<ClockState> {
  if (!state.playing || !Number.isFinite(ms)) return {};
  const limit = holdPoint(state.readyBeats);
  if (!state.reduced) {
    const time = Math.min(limit, state.time + ms * state.speed);
    return { time, playing: time < PASS_END };
  }
  let time = state.time;
  let since = state.sinceStill + ms;
  while (since >= STILL_LASTS) {
    const next = nextStill(time);
    if (next === null || next > limit) {
      // Held: the next still shows the moment its data arrives.
      since = STILL_LASTS;
      break;
    }
    time = next;
    since -= STILL_LASTS;
  }
  return { time, sinceStill: since, playing: time < PASS_END };
}

function stepToNextStill(time: number, readyBeats: number): number {
  const next = nextStill(time);
  return next !== null && next <= holdPoint(readyBeats) ? next : time;
}

function nextStill(time: number): number | null {
  return STILLS.find((still) => still > time) ?? null;
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}
