import { filmOf } from '@/case-state/selectors';
import type { StepContract } from '@/steps/step-contract';
import { FIND_WORDS } from '@/words/find';
import { foundOnFront } from './find-run';
import { FindScreen } from './FindScreen';

export default {
  number: 2, slug: 'find', title: FIND_WORDS.stepName, Screen: FindScreen,
  isReady: (c) => filmOf(c, 'front') !== null,
  canLeave: (c) => foundOnFront(c) > 0,
} satisfies StepContract;
