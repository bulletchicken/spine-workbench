// One explicit split or merge on the film held by this worker.

import type { BoundaryWandOutput } from '@/shapes';
import { defineJob } from '../../boot';

export default defineJob('boundary-wand', {
  run: (input, room) => room.desk().boundary_wand(input) as BoundaryWandOutput,
});
