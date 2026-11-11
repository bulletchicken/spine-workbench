// Reduced motion: follow the system setting, or always use reduced motion.
// This is the one preference the app remembers between visits, and the
// one place browser storage is used. A choice made from a link (such as
// ?motion=reduced) lasts for this visit only: it is set with `remember`
// false, so a later visit goes back to following the system. Every read and
// write sits inside try and catch: where storage is blocked the choice lasts
// for this visit and the app works unchanged. Never saved with the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';

export type MotionPreference = 'system' | 'reduced';

/** The storage key. Nothing else is ever written to browser storage. */
export const MOTION_KEY = 'spine-workbench.motion';

/** The page's local storage, or `null` where the browser withholds it. */
function storageOrNull(): Storage | null {
  try {
    // A page without storage has no value here at all, so the type says it may be missing.
    const storage = globalThis.localStorage as Storage | undefined;
    return storage ?? null;
  } catch {
    return null;
  }
}

/** The remembered choice; `system` when none is remembered or storage is blocked. */
export function readMotionPreference(): MotionPreference {
  try {
    return storageOrNull()?.getItem(MOTION_KEY) === 'reduced' ? 'reduced' : 'system';
  } catch {
    return 'system';
  }
}

/** Remember the choice. Where storage is blocked, nothing is remembered. */
export function writeMotionPreference(motion: MotionPreference): void {
  try {
    storageOrNull()?.setItem(MOTION_KEY, motion);
  } catch {
    // Blocked storage: the choice still holds for this visit.
  }
}

export interface PreferencesState {
  motion: MotionPreference;
  /**
   * Set the choice. With `remember`, it is kept for later visits; without,
   * it holds for this visit only.
   */
  setMotion(motion: MotionPreference, remember: boolean): void;
}

export type PreferencesApi = StoreApi<PreferencesState>;

/** A store starting from `initial`. The app has one, `preferences`; tests make their own. */
export function createPreferences(initial: MotionPreference): PreferencesApi {
  return createStore<PreferencesState>()((set) => ({
    motion: initial,
    setMotion: (motion, remember) => {
      if (remember) writeMotionPreference(motion);
      set({ motion });
    },
  }));
}

/** The app's preferences, starting from what the browser remembers. */
export const preferences: PreferencesApi = createPreferences(readMotionPreference());

/** The preferences, in a component. */
export function usePreferences(): PreferencesState {
  return useStore(preferences);
}
