import { hasCurrentFit } from '@/bench/three/build-spine';
import type { StepContract } from '@/steps/step-contract';
import { spineBuild } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import type { CaseFileV1 } from '@/shapes';
import { PLACE_WORDS } from '@/words/place';
import { PlaceScreen } from './PlaceScreen';
function ready(c: CaseFileV1): boolean { return hasCurrentFit(c) && !threeState.getState().moving && spineBuild.getState().accepted?.movePending !== true; }
export default { number: 4, slug: 'place', title: PLACE_WORDS.step, Screen: PlaceScreen, isReady: ready, canLeave: ready } satisfies StepContract;
