// What every step of the journey provides. FROZEN: changes only through a
// contract-change pull request.
//
// The steps are folders in steps/, numbered in the order a surgeon uses
// them; each holds a step.ts whose default export is a StepContract. The
// app shell finds them, orders them by number and draws the step rail.

import type { ComponentType } from 'react';
import type { CaseFileV1 } from '@/shapes';

export interface StepContract {
  /** Position in the journey, from 1: the number in the folder's name. */
  number: number;
  /** The folder's name without its number, such as `square-up`. */
  slug: string;
  /** The step's name on the step rail, from its words file. */
  title: string;
  /** The whole screen for this step. It reads the case and the engine through hooks. */
  Screen: ComponentType;
  /** The step can be opened: what it works on is in the case. */
  isReady(c: CaseFileV1): boolean;
  /**
   * Everything this step settles is settled: the rail shows a tick and the
   * way forward opens. Going back to an earlier step is always allowed.
   */
  canLeave(c: CaseFileV1): boolean;
}
