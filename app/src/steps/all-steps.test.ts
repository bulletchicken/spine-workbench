// The steps are found by their folders and ordered by their numbers. The
// test on the real folders asks only that the numbers are unique and that
// each step says what its folder says, so the steps can land in any order.

import { describe, expect, it } from 'vitest';
import { allSteps, folderOf, stepFiles, stepsFrom } from './all-steps';
import type { StepContract } from './step-contract';

function step(number: number, slug: string): StepContract {
  return { number, slug, title: slug, Screen: () => null, isReady: () => true, canLeave: () => false };
}

describe('the steps found', () => {
  it('come in the order of their numbers, whatever order the files were found in', () => {
    const found = { './4-adjust/step.ts': step(4, 'adjust'), './2-size-and-facing/step.ts': step(2, 'size-and-facing'), './3-find/step.ts': step(3, 'find') };
    expect(stepsFrom(found).map((each) => each.slug)).toEqual(['size-and-facing', 'find', 'adjust']);
  });

  it('are none when there are no step folders yet', () => {
    expect(stepsFrom({})).toEqual([]);
  });

  it('read a step’s number and slug from its folder', () => {
    expect(folderOf('./2-size-and-facing/step.ts')).toEqual({ number: 2, slug: 'size-and-facing' });
    expect(folderOf('./10-bones/step.ts')).toEqual({ number: 10, slug: 'bones' });
    expect(folderOf('./step-contract.ts')).toBeNull();
  });
});

describe('the steps in the app', () => {
  it('have numbers that are unique and ascending', () => {
    const numbers = allSteps().map((each) => each.number);
    expect(numbers).toEqual([...new Set(numbers)].sort((a, b) => a - b));
  });

  it('each say the number and the slug of the folder they are in', () => {
    for (const [path, found] of Object.entries(stepFiles())) {
      expect({ number: found.number, slug: found.slug }, path).toEqual(folderOf(path));
    }
  });
});
