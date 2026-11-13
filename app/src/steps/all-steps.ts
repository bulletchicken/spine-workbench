// Finds the steps by their folders, so no step is added to a list: a
// folder `<n>-<slug>/` holding a `step.ts` whose default export is a
// StepContract is on the rail, in the order of its number. Only the app
// shell imports this file.

import type { StepContract } from './step-contract';

/** The steps of the files found, in the order a surgeon uses them: by number. */
export function stepsFrom(found: Readonly<Record<string, StepContract>>): StepContract[] {
  return Object.values(found).sort((a, b) => a.number - b.number);
}

/** The folder a step file sits in, as `{ number, slug }`: `./2-size-and-facing/step.ts` is step 2, `size-and-facing`. */
export function folderOf(path: string): { number: number; slug: string } | null {
  const named = /\/(\d+)-([^/]+)\/step\.ts$/.exec(path);
  if (named?.[1] === undefined || named[2] === undefined) return null;
  return { number: Number(named[1]), slug: named[2] };
}

/** Every step file in the app, by its path from here. */
export function stepFiles(): Record<string, StepContract> {
  return import.meta.glob<StepContract>('./*/step.ts', { eager: true, import: 'default' });
}

/** Every step in the app, in order. */
export function allSteps(): StepContract[] {
  return stepsFrom(stepFiles());
}
