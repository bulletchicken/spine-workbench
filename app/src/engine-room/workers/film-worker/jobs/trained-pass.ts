// Run the trained finder on this front film through the worker's shared
// step loop.

import type { Step, TrainedPassResult } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('trained-pass', {
  start: ({ film, firstVertebraId }, room) => {
    const pass = room.desk().start_trained_pass(film, firstVertebraId);
    return {
      advance: (workUnits) => pass.advance(workUnits) as Step,
      finish: () => pass.finish() as TrainedPassResult,
      free: () => {
        pass.free();
      },
    };
  },
});
