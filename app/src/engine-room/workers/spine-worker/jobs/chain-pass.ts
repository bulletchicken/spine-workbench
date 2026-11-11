// Run the chain stages through the worker's shared step loop.

import type { ChainAnswer, Step } from '@/shapes';
import { ChainPass } from '../../../wasm-build/wasm_bridge';
import { defineJob } from '../../boot';

export default defineJob('chain-pass', {
  start: ({ results, options }) => {
    const pass = ChainPass.start(results, options);
    return {
      advance: (workUnits) => pass.advance(workUnits) as Step,
      finish: () => pass.finish() as ChainAnswer,
      free: () => {
        pass.free();
      },
    };
  },
});
