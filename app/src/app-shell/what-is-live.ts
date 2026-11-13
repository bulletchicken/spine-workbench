// Which of the top bar's actions answer right now.
//
// Undo and redo are live in the Adjust step only. The undo list is started
// afresh when a case's first pass lands, so that from there on undo never
// reaches back into films and sizes; before that step there is nothing a
// person would expect it to take back.
//
// Save case is live once the Adjust step can be opened: when the case
// holds the marks a case file is for.

import type { CaseFileV1 } from '@/shapes';
import type { StepContract } from '@/steps/step-contract';

/** The slug of the step where marks are adjusted, undone and saved. */
export const ADJUST_STEP = 'adjust';

/** Undo and redo answer while the step on screen has this slug. */
export function undoIsLive(slugOnScreen: string | null): boolean {
  return slugOnScreen === ADJUST_STEP;
}

/** Save case answers once the Adjust step is among the steps and can be opened. */
export function saveIsLive(steps: readonly StepContract[], c: CaseFileV1): boolean {
  return steps.find((step) => step.slug === ADJUST_STEP)?.isReady(c) === true;
}
