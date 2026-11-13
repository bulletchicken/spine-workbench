// The rail's states, on a four-step journey whose steps can be left one
// after another as a counter goes up: a stand-in for a case filling in.

import { describe, expect, it } from 'vitest';
import { emptyCase } from '@/case-state/empty-case';
import type { CaseFileV1 } from '@/shapes';
import type { StepContract } from '@/steps/step-contract';
import { railSteps, stepOnScreen } from './rail-steps';

/** A case that has come as far as `settled` steps: its `nextVertebraId` carries the count. */
function caseWith(settled: number): CaseFileV1 {
  return { ...emptyCase(), nextVertebraId: settled };
}

/** Step `number` can be opened once the step before it is settled, and left once it is settled itself. */
function step(number: number, slug: string): StepContract {
  return {
    number,
    slug,
    title: slug,
    Screen: () => null,
    isReady: (c) => c.nextVertebraId >= number - 1,
    canLeave: (c) => c.nextVertebraId >= number,
  };
}

const STEPS = [step(1, 'films'), step(2, 'size-and-facing'), step(3, 'find'), step(4, 'adjust')];

const states = (settled: number, slug: string) => railSteps(STEPS, caseWith(settled), slug).map((each) => each.state);
const open = (settled: number, slug: string) => railSteps(STEPS, caseWith(settled), slug).map((each) => each.open);

describe('the rail', () => {
  it('starts with the first step on screen and nothing to press', () => {
    expect(states(0, 'films')).toEqual(['now', 'todo', 'todo', 'todo']);
    expect(open(0, 'films')).toEqual([false, false, false, false]);
  });

  it('ticks the steps behind and lets each be pressed to go back', () => {
    expect(states(2, 'find')).toEqual(['done', 'done', 'now', 'todo']);
    expect(open(2, 'find')).toEqual([true, true, false, false]);
  });

  it('does not open the next step from the rail when its own work has not started', () => {
    // Films are settled, so step 2 can be opened; but nothing of step 2 is settled, and the way there is the lit button.
    expect(open(1, 'films')).toEqual([false, false, false, false]);
  });

  it('after going back, opens the later steps that are done, and no further', () => {
    // Three steps settled, the fourth begun: back on step 1, steps 2 and 3 can be pressed, step 4 cannot.
    expect(states(3, 'films')).toEqual(['now', 'done', 'done', 'todo']);
    expect(open(3, 'films')).toEqual([false, true, true, false]);
    // Everything settled: every other step can be pressed.
    expect(open(4, 'size-and-facing')).toEqual([true, false, true, true]);
  });

  it('closes the way forward when the step on screen is no longer settled', () => {
    const unsettled: StepContract = { ...step(2, 'size-and-facing'), canLeave: () => false };
    const rail = railSteps([STEPS[0], unsettled, STEPS[2], STEPS[3]].filter((each) => each !== undefined), caseWith(4), 'size-and-facing');
    expect(rail.map((each) => each.open)).toEqual([true, false, false, false]);
    // The steps beyond still show what the case says of them.
    expect(rail.map((each) => each.state)).toEqual(['done', 'now', 'done', 'done']);
  });

  it('shows an earlier step that can no longer be left as not done, and still lets it be pressed', () => {
    const rail = railSteps(STEPS, caseWith(0), 'find');
    expect(rail.map((each) => each.state)).toEqual(['todo', 'todo', 'now', 'todo']);
    expect(rail.map((each) => each.open)).toEqual([true, true, false, false]);
  });

  it('puts the first step on screen when no step has the slug asked for', () => {
    const two = [step(2, 'size-and-facing'), step(4, 'adjust')];
    expect(stepOnScreen(two, 'films')?.slug).toBe('size-and-facing');
    expect(railSteps(two, caseWith(0), 'films').map((each) => each.state)).toEqual(['now', 'todo']);
    expect(stepOnScreen([], 'films')).toBeNull();
    expect(railSteps([], caseWith(0), 'films')).toEqual([]);
  });
});
