import { TEMPLATE_LEVELS } from '@/shapes';
import { hasCurrentFit } from '@/bench/three/build-spine';
import { filmOf, marksOf } from '@/case-state/selectors';
import type { StepContract } from '@/steps/step-contract';
import { THREE_WORDS } from '@/words/three';
import { ThreeScreen } from './ThreeScreen';
export default {
  number: 3, slug: 'three', title: THREE_WORDS.stepName, Screen: ThreeScreen,
  isReady: (c) => filmOf(c, 'front') !== null && (marksOf(c, 'front')?.vertebrae.some((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level)) ?? false),
  canLeave: hasCurrentFit,
} satisfies StepContract;
