// One front film, or a front and side pair, fitted as one spine.

import type { SpineFit } from '@/shapes';
import { fit_from_films } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('fit-spine', {
  run: (input) => fit_from_films(input) as SpineFit,
});
