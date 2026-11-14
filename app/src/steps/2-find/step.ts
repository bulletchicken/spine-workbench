import { filmOf } from '@/case-state/selectors';
import type { StepContract } from '@/steps/step-contract';
import { FIND_WORDS } from '@/words/find';
import { foundOnLead } from './find-run';
import { FindScreen } from './FindScreen';

export default {
  number: 2, slug: 'find', title: FIND_WORDS.stepName, Screen: FindScreen,
  isReady: (c) => filmOf(c, 'front') !== null || filmOf(c, 'side') !== null,
  canLeave: (c) => foundOnLead(c) > 0,
} satisfies StepContract;
