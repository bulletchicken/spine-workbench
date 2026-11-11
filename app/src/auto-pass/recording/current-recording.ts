// This session's recording, or none. Each film's part is kept with the
// stamp its caller gives and is given out only while that stamp still
// matches, so a turned or replaced film never shows a stale strip and
// Watch again never plays a pass over films that have changed since.
//
// Strips rebuilt after a case is reopened are kept here too, each with its
// own stamp. Keeping a new recording supersedes what it covers: a film it
// does not cover keeps its strip from before, still under its old stamp.

import type { FilmKind } from '@/shapes';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmStamp } from './film-stamp';
import type { FilmStrip, PassRecording } from './pass-recording';
import { filmStrip } from './record-frames';

export type FilmStamps = Partial<Record<FilmKind, FilmStamp>>;

interface KeptStrip {
  strip: FilmStrip;
  stamp: FilmStamp;
}

export interface CurrentRecordingState {
  recording: PassRecording | null;
  /** The stamp of each film in `recording`, as it stood when the pass ran. */
  stamps: FilmStamps;
  /** Strips kept on their own: rebuilt, or carried over from an earlier recording. */
  strips: Partial<Record<FilmKind, KeptStrip>>;

  /** Keeps a pass's recording with each of its films' stamps. */
  keep(recording: PassRecording, stamps: FilmStamps): void;
  /** Keeps one film's strip, rebuilt on its own. */
  keepStrip(film: FilmKind, strip: FilmStrip, stamp: FilmStamp): void;
  /** A film's strip, only while it was worked out from the film as it stands now. */
  stripFor(film: FilmKind, stamp: FilmStamp): FilmStrip | null;
  /** The whole recording, only while every film in it is as it stood when the pass ran. */
  recordingFor(stamps: FilmStamps): PassRecording | null;
  /** Lets everything go: a new case. */
  forget(): void;
}

export type CurrentRecording = StoreApi<CurrentRecordingState>;

export function createCurrentRecording(): CurrentRecording {
  return createStore<CurrentRecordingState>()((set, get) => ({
    recording: null,
    stamps: {},
    strips: {},
    keep: (recording, stamps) => {
      set((state) => {
        const covered = new Set(recording.films.map((part) => part.film));
        const strips: Partial<Record<FilmKind, KeptStrip>> = {};
        for (const [film, kept] of entries(state.strips)) {
          if (!covered.has(film)) strips[film] = kept;
        }
        for (const part of state.recording?.films ?? []) {
          const strip = filmStrip(part);
          const stamp = state.stamps[part.film];
          if (!covered.has(part.film) && strip !== null && stamp !== undefined) strips[part.film] = { strip, stamp };
        }
        return { recording, stamps, strips };
      });
    },
    keepStrip: (film, strip, stamp) => {
      set((state) => ({ strips: { ...state.strips, [film]: { strip, stamp } } }));
    },
    stripFor: (film, stamp) => {
      const { recording, stamps, strips } = get();
      const part = recording?.films.find((p) => p.film === film);
      const recorded = part === undefined || stamps[film] !== stamp ? null : filmStrip(part);
      if (recorded !== null) return recorded;
      const kept = strips[film];
      return kept !== undefined && kept.stamp === stamp ? kept.strip : null;
    },
    recordingFor: (now) => {
      const { recording, stamps } = get();
      if (recording === null) return null;
      const unchanged = recording.films.every((part) => stamps[part.film] !== undefined && stamps[part.film] === now[part.film]);
      return unchanged ? recording : null;
    },
    forget: () => {
      set({ recording: null, stamps: {}, strips: {} });
    },
  }));
}

/** This session's recording. */
export const currentRecording = createCurrentRecording();

function entries(strips: Partial<Record<FilmKind, KeptStrip>>): [FilmKind, KeptStrip][] {
  return (Object.keys(strips) as FilmKind[]).flatMap((film) => {
    const kept = strips[film];
    return kept === undefined ? [] : [[film, kept]];
  });
}
