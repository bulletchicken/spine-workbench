// The answer key of a made-up spine, if the case is one. The key is the
// truth the made-up spine was drawn from, so the app shows it only where
// the design does. Never saved with the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { AnswerKey } from '@/shapes';

export interface AnswerKeyState {
  /** The made-up spine's answer key, or `null` for a case that is not one. */
  key: AnswerKey | null;
  setKey(key: AnswerKey | null): void;
}

export type AnswerKeyApi = StoreApi<AnswerKeyState>;

/** A store with no answer key. The app has one, `answerKey`; tests make their own. */
export function createAnswerKey(): AnswerKeyApi {
  return createStore<AnswerKeyState>()((set) => ({
    key: null,
    setKey: (key) => {
      set({ key });
    },
  }));
}

/** The app's answer key. */
export const answerKey: AnswerKeyApi = createAnswerKey();

/** The answer key, in a component. */
export function useAnswerKey(): AnswerKeyState {
  return useStore(answerKey);
}
