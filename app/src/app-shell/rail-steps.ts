// What the rail shows for each step, worked out from the case alone.
//
// A step is `now` when it is on screen, `done` when everything it settles
// is settled (the step's own `canLeave`), and `todo` otherwise. Going back
// is always allowed, so every step before the one on screen can be
// pressed. A later step can be pressed only when the way to it is open and
// it has nothing left to do itself: it and every step from the one on
// screen up to it are done, and it can be opened. So the rail never drops
// a person into a step whose work has not started; that way forward is
// the lit button's.

import type { CaseFileV1 } from '@/shapes';
import type { StepContract } from '@/steps/step-contract';

export interface RailStep {
  slug: string;
  number: number;
  title: string;
  state: 'now' | 'done' | 'todo';
  /** A press goes to this step. */
  open: boolean;
}

/** The step on screen: the one with this slug, or the first when no step has it; `null` when there are no steps. */
export function stepOnScreen(steps: readonly StepContract[], slug: string): StepContract | null {
  return steps.find((step) => step.slug === slug) ?? steps[0] ?? null;
}

/** The rail for `steps`, in order, with the step whose slug is `slug` on screen. */
export function railSteps(steps: readonly StepContract[], c: CaseFileV1, slug: string): RailStep[] {
  const now = Math.max(0, steps.findIndex((step) => step.slug === slug));
  const done = steps.map((step) => step.canLeave(c));
  return steps.map((step, at) => ({
    slug: step.slug,
    number: step.number,
    title: step.title,
    state: at === now ? 'now' : done[at] === true ? 'done' : 'todo',
    open: at < now || (at > now && step.isReady(c) && done.slice(now, at + 1).every(Boolean)),
  }));
}
