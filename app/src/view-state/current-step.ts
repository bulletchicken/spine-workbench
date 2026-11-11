// The step on screen, by its slug: the folder name without its number.
// Never saved with the case. Going back to an earlier step is always
// allowed, so nothing here checks a step before it moves.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';

export interface CurrentStepState {
  /** The folder name without its number, such as `size-and-facing`. */
  slug: string;
  /** Show this step. */
  go(slug: string): void;
}

export type CurrentStepApi = StoreApi<CurrentStepState>;

/** A store on `start`, which is the first step unless a test says otherwise. */
export function createCurrentStep(start = 'films'): CurrentStepApi {
  return createStore<CurrentStepState>()((set) => ({
    slug: start,
    go: (slug) => {
      set({ slug });
    },
  }));
}

/** The app's current step. */
export const currentStep: CurrentStepApi = createCurrentStep();

/** The current step, in a component. */
export function useCurrentStep(): CurrentStepState {
  return useStore(currentStep);
}
