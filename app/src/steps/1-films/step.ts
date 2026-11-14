import { filmOf } from '@/case-state/selectors';
import { FILMS_WORDS } from '@/words/films';
import type { StepContract } from '../step-contract';
import { FilmsScreen } from './FilmsScreen';

export default {
  number: 1, slug: 'films', title: FILMS_WORDS.stepName, Screen: FilmsScreen,
  isReady: () => true,
  canLeave: (c) => filmOf(c, 'front') !== null || filmOf(c, 'side') !== null,
} satisfies StepContract;
